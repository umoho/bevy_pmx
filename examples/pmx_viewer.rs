//! Load a PMX file from the command line and open a Bevy window to inspect it.
//!
//! This example intentionally uses the crate's public API instead of any private internals:
//! `parse_pmx` parses the file, `import_pmx` builds the Bevy-friendly model, and
//! `PmxMeshGeometry::to_mesh()` turns the imported geometry into a renderable mesh.

use std::{
    error::Error,
    fs, io,
    path::{Path, PathBuf},
};

use bevy::{
    asset::RenderAssetUsages,
    image::{CompressedImageFormats, ImageSampler, ImageType},
    prelude::*,
    window::WindowPlugin,
};
use bevy_pmx::prelude::*;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "pmx_viewer",
    version,
    about = "Load a PMX file and open a viewer window"
)]
struct Cli {
    /// Path to the PMX file to open.
    #[arg(value_name = "PMX")]
    path: PathBuf,
}

#[derive(Debug, Resource)]
struct ViewerScene {
    path: PathBuf,
    model: Pmx,
    bounds_center: Vec3,
    bounds_radius: f32,
    title: String,
    textures: Vec<DecodedTexture>,
}

#[derive(Debug)]
struct DecodedTexture {
    path: PmxResolvedPath,
    image: Image,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let scene = load_scene(&cli.path)?;
    let window_title = scene.title.clone();

    App::new()
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        })
        .insert_resource(scene)
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: window_title,
                    ..default()
                }),
                ..default()
            }),
            PmxPlugin::default(),
        ))
        .add_systems(Startup, (log_scene_summary, setup_scene))
        .run();

    Ok(())
}

fn load_scene(path: &Path) -> Result<ViewerScene, Box<dyn Error>> {
    let bytes = fs::read(path)?;
    let document = parse_pmx(&bytes)?;
    let source_root = path.parent().unwrap_or_else(|| Path::new("."));
    let context = PmxImportContext::with_source(PmxSource::folder(source_root));
    let model = import_pmx(document, &context).model;

    if model.geometry().positions.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("{} contains no vertices", path.display()),
        )
        .into());
    }

    let textures = load_textures(model.texture_paths())?;
    let (bounds_center, bounds_radius) = bounds_for_geometry(model.geometry());
    let title = build_window_title(path, &model);

    Ok(ViewerScene {
        path: path.to_path_buf(),
        model,
        bounds_center,
        bounds_radius,
        title,
        textures,
    })
}

fn setup_scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    scene: Res<ViewerScene>,
) {
    let texture_handles: Vec<Handle<Image>> = scene
        .textures
        .iter()
        .map(|texture| images.add(texture.image.clone()))
        .collect();

    let base_color_texture = choose_base_color_texture(&scene.model, &texture_handles);
    let mesh = meshes.add(scene.model.geometry().to_mesh());
    let material = materials.add(StandardMaterial {
        base_color: primary_material_color(&scene.model),
        base_color_texture,
        cull_mode: None,
        alpha_mode: if texture_handles.is_empty() {
            AlphaMode::Opaque
        } else {
            AlphaMode::Blend
        },
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    });

    commands.spawn((
        Name::new("PMX Model"),
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform::from_translation(-scene.bounds_center),
    ));

    let camera_distance = (scene.bounds_radius * 2.8).max(2.0);
    let camera_height = (scene.bounds_radius * 0.25).max(0.5);
    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, camera_height, camera_distance).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Directional Light"),
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 20_000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.9, 0.0)),
    ));
}

fn log_scene_summary(scene: Res<ViewerScene>) {
    let model_name = scene
        .model
        .raw_document()
        .and_then(|document| {
            if !document.header.model_name.is_empty() {
                Some(document.header.model_name.as_str())
            } else if !document.header.model_name_english.is_empty() {
                Some(document.header.model_name_english.as_str())
            } else {
                None
            }
        })
        .or_else(|| scene.path.file_stem().and_then(|stem| stem.to_str()))
        .unwrap_or("pmx viewer");

    info!("PMX file: {}", scene.path.display());
    info!("Model name: {model_name}");
    info!("Vertices: {}", scene.model.geometry().positions.len());
    info!("Indices: {}", scene.model.geometry().indices.len());
    info!("Primitives: {}", scene.model.primitives().len());
    info!("Materials: {}", scene.model.material_records().len());
    info!("Textures: {}", scene.model.texture_paths().len());
    info!("Decoded textures: {}", scene.textures.len());
    for (index, texture) in scene.textures.iter().enumerate() {
        let size = texture.image.texture_descriptor.size;
        info!(
            "  [{index}] {} -> {} ({}x{})",
            texture.path.original,
            texture.path.resolved.display(),
            size.width,
            size.height
        );
    }
    info!("Bones: {}", scene.model.bones().len());
}

fn build_window_title(path: &Path, model: &Pmx) -> String {
    let model_name = model
        .raw_document()
        .and_then(|document| {
            if !document.header.model_name.is_empty() {
                Some(document.header.model_name.clone())
            } else if !document.header.model_name_english.is_empty() {
                Some(document.header.model_name_english.clone())
            } else {
                None
            }
        })
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "pmx viewer".to_owned());

    format!("bevy_pmx - {model_name}")
}

fn primary_material_color(model: &Pmx) -> Color {
    model
        .material_records()
        .first()
        .map(|record| {
            let [r, g, b, _] = record.material.diffuse;
            Color::srgb(r, g, b)
        })
        .unwrap_or_else(|| Color::srgb(0.85, 0.85, 0.85))
}

fn choose_base_color_texture(
    model: &Pmx,
    texture_handles: &[Handle<Image>],
) -> Option<Handle<Image>> {
    let diffuse_index = model.material_records().first().and_then(|record| {
        (record.material.texture_index >= 0).then_some(record.material.texture_index as usize)
    });

    diffuse_index
        .and_then(|index| texture_handles.get(index).cloned())
        .or_else(|| texture_handles.first().cloned())
}

fn load_textures(paths: &[PmxResolvedPath]) -> Result<Vec<DecodedTexture>, io::Error> {
    let mut textures = Vec::with_capacity(paths.len());

    for path in paths {
        let bytes = fs::read(path.resolved_path())?;
        let image = decode_texture(path.resolved_path(), &bytes)?;
        textures.push(DecodedTexture {
            path: path.clone(),
            image,
        });
    }

    Ok(textures)
}

fn decode_texture(path: &Path, bytes: &[u8]) -> Result<Image, io::Error> {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("texture {} does not have a file extension", path.display()),
            )
        })?;

    let extension = extension.to_ascii_lowercase();
    let image_type = ImageType::Extension(extension.as_str());
    Image::from_buffer(
        bytes,
        image_type,
        CompressedImageFormats::all(),
        true,
        ImageSampler::Default,
        RenderAssetUsages::default(),
    )
    .map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("failed to decode texture {}: {error}", path.display()),
        )
    })
}

fn bounds_for_geometry(geometry: &PmxMeshGeometry) -> (Vec3, f32) {
    let Some(first) = geometry.positions.first() else {
        return (Vec3::ZERO, 1.0);
    };

    let mut min = Vec3::from(*first);
    let mut max = Vec3::from(*first);

    for position in &geometry.positions[1..] {
        let point = Vec3::from(*position);
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        min.z = min.z.min(point.z);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
        max.z = max.z.max(point.z);
    }

    let center = (min + max) * 0.5;
    let radius = ((max - min) * 0.5).length().max(1.0);
    (center, radius)
}
