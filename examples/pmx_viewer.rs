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
    render::render_resource::{Extent3d, Face, TextureDimension, TextureFormat},
    window::{PrimaryWindow, WindowPlugin},
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
struct SceneRequest {
    path: PathBuf,
}

#[derive(Debug)]
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
    has_alpha: bool,
    fallback: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    App::new()
        .insert_resource(SceneRequest { path: cli.path })
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 200.0,
            ..default()
        })
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy_pmx".to_owned(),
                    ..default()
                }),
                ..default()
            }),
            PmxPlugin::default(),
        ))
        .add_systems(Startup, bootstrap_scene)
        .run();

    Ok(())
}

fn load_scene(path: &Path) -> Result<ViewerScene, Box<dyn Error>> {
    let bytes = fs::read(path).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("failed to read PMX file {}: {error}", path.display()),
        )
    })?;
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

    let textures = load_textures(model.texture_paths());
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

fn bootstrap_scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    request: Res<SceneRequest>,
) {
    let scene = load_scene(&request.path).unwrap_or_else(|error| {
        error!(
            "failed to load PMX scene {}: {error}",
            request.path.display()
        );
        panic!(
            "failed to load PMX scene {}: {error}",
            request.path.display()
        );
    });

    if let Ok(mut window) = windows.single_mut() {
        window.title = scene.title.clone();
    }

    log_scene_summary(&scene);
    setup_scene(
        &mut commands,
        &mut images,
        &mut meshes,
        &mut materials,
        &scene,
    );
}

fn setup_scene(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scene: &ViewerScene,
) {
    let texture_handles: Vec<Handle<Image>> = scene
        .textures
        .iter()
        .map(|texture| images.add(texture.image.clone()))
        .collect();
    let texture_has_alpha: Vec<bool> = scene
        .textures
        .iter()
        .map(|texture| texture.has_alpha)
        .collect();
    let model_transform = Transform::from_translation(-scene.bounds_center);

    if scene.model.primitives().is_empty() {
        let mesh = meshes.add(scene.model.geometry().to_mesh());
        let material = materials.add(default_viewer_material());

        commands.spawn((
            Name::new("PMX Model"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            model_transform,
        ));
    } else {
        for (primitive_index, primitive) in scene.model.primitives().iter().enumerate() {
            let Some(record) = scene.model.material_records().get(primitive.material_index) else {
                continue;
            };

            let mesh = meshes.add(scene.model.geometry().to_mesh_for_primitive(*primitive));
            let material = materials.add(material_for_record(
                record,
                &texture_handles,
                &texture_has_alpha,
            ));

            commands.spawn((
                Name::new(format!(
                    "PMX Primitive {primitive_index} ({})",
                    record.material.name
                )),
                Mesh3d(mesh),
                MeshMaterial3d(material),
                model_transform.clone(),
            ));
        }
    }

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

fn log_scene_summary(scene: &ViewerScene) {
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
        if texture.fallback {
            info!(
                "  [{index}] {} -> {} (placeholder, {}x{})",
                texture.path.original,
                texture.path.resolved.display(),
                size.width,
                size.height
            );
        } else {
            info!(
                "  [{index}] {} -> {} ({}x{})",
                texture.path.original,
                texture.path.resolved.display(),
                size.width,
                size.height
            );
        }
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

fn default_viewer_material() -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.85, 0.85, 0.85),
        cull_mode: Some(Face::Back),
        double_sided: false,
        alpha_mode: AlphaMode::Opaque,
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    }
}

fn material_for_record(
    record: &PmxMaterialRecord,
    texture_handles: &[Handle<Image>],
    texture_has_alpha: &[bool],
) -> StandardMaterial {
    let material = &record.material;
    let diffuse_texture = (material.texture_index >= 0)
        .then_some(material.texture_index as usize)
        .and_then(|index| texture_handles.get(index).cloned());
    let diffuse_texture_has_alpha = (material.texture_index >= 0)
        .then_some(material.texture_index as usize)
        .and_then(|index| texture_has_alpha.get(index).copied())
        .unwrap_or(false);
    let [r, g, b, a] = material.diffuse;
    let no_cull = material.flags.contains(PmxMaterialFlags::NO_CULL);

    StandardMaterial {
        base_color: Color::srgba(r, g, b, a),
        base_color_texture: diffuse_texture.clone(),
        cull_mode: if no_cull { None } else { Some(Face::Back) },
        double_sided: no_cull,
        alpha_mode: if a < 0.999 {
            AlphaMode::Blend
        } else if diffuse_texture_has_alpha {
            AlphaMode::AlphaToCoverage
        } else {
            AlphaMode::Opaque
        },
        perceptual_roughness: 0.95,
        metallic: 0.0,
        ..default()
    }
}

fn load_textures(paths: &[PmxResolvedPath]) -> Vec<DecodedTexture> {
    let mut textures = Vec::with_capacity(paths.len());

    for path in paths {
        match load_texture(path) {
            Ok((image, has_alpha)) => textures.push(DecodedTexture {
                path: path.clone(),
                image,
                has_alpha,
                fallback: false,
            }),
            Err(error) => {
                warn!(
                    "missing texture: {} (resolved to {}) - using magenta placeholder",
                    path.original,
                    path.resolved_path().display()
                );
                warn!("  read error: {error}");
                textures.push(DecodedTexture {
                    path: path.clone(),
                    image: placeholder_texture_image(),
                    has_alpha: false,
                    fallback: true,
                });
            }
        }
    }

    textures
}

fn load_texture(path: &PmxResolvedPath) -> Result<(Image, bool), io::Error> {
    let bytes = fs::read(path.resolved_path()).map_err(|error| {
        io::Error::new(
            error.kind(),
            format!(
                "failed to read texture {} (resolved to {})",
                path.original,
                path.resolved_path().display()
            ),
        )
    })?;
    let image = decode_texture(path.resolved_path(), &bytes)?;
    let has_alpha = image_has_alpha(&image);
    Ok((image, has_alpha))
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

fn image_has_alpha(image: &Image) -> bool {
    matches!(
        image.texture_descriptor.format,
        TextureFormat::Rgba8Unorm
            | TextureFormat::Rgba8UnormSrgb
            | TextureFormat::Rgba8Snorm
            | TextureFormat::Rgba8Uint
            | TextureFormat::Rgba8Sint
            | TextureFormat::Bgra8Unorm
            | TextureFormat::Bgra8UnormSrgb
            | TextureFormat::Rgb10a2Unorm
            | TextureFormat::Rgb10a2Uint
            | TextureFormat::Rgba16Unorm
            | TextureFormat::Rgba16Snorm
            | TextureFormat::Rgba16Uint
            | TextureFormat::Rgba16Sint
            | TextureFormat::Rgba16Float
            | TextureFormat::Rgba32Uint
            | TextureFormat::Rgba32Sint
            | TextureFormat::Rgba32Float
            | TextureFormat::Bc1RgbaUnorm
            | TextureFormat::Bc1RgbaUnormSrgb
            | TextureFormat::Bc2RgbaUnorm
            | TextureFormat::Bc2RgbaUnormSrgb
            | TextureFormat::Bc3RgbaUnorm
            | TextureFormat::Bc3RgbaUnormSrgb
            | TextureFormat::Bc7RgbaUnorm
            | TextureFormat::Bc7RgbaUnormSrgb
            | TextureFormat::Etc2Rgb8A1Unorm
            | TextureFormat::Etc2Rgb8A1UnormSrgb
            | TextureFormat::Etc2Rgba8Unorm
            | TextureFormat::Etc2Rgba8UnormSrgb
    )
}

fn placeholder_texture_image() -> Image {
    Image::new_fill(
        Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[255, 0, 255, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
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
