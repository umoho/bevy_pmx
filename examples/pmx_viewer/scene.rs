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
    window::PrimaryWindow,
};
use bevy_pmx::prelude::*;

use crate::{gizmos, orbit};

#[derive(Debug, Resource)]
pub(crate) struct SceneRequest {
    pub(crate) path: PathBuf,
}

#[derive(Debug, Resource)]
pub(crate) struct LoadedScene {
    pub(crate) path: PathBuf,
    pub(crate) model: Pmx,
    pub(crate) bounds_min: Vec3,
    pub(crate) bounds_max: Vec3,
    pub(crate) bounds_center: Vec3,
    pub(crate) bounds_radius: f32,
    pub(crate) title: String,
}

impl LoadedScene {
    pub(crate) fn model_transform(&self) -> Transform {
        Transform::from_translation(-self.bounds_center)
    }

    pub(crate) fn world_bounds(&self) -> (Vec3, Vec3) {
        (
            self.bounds_min - self.bounds_center,
            self.bounds_max - self.bounds_center,
        )
    }
}

#[derive(Component)]
pub(crate) struct ViewerPrimitive;

#[derive(Debug)]
struct DecodedTexture {
    path: PmxResolvedPath,
    image: Image,
    has_alpha: bool,
    fallback: bool,
}

#[derive(Debug, Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn from_point(point: Vec3) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    fn include(&mut self, point: Vec3) {
        self.min.x = self.min.x.min(point.x);
        self.min.y = self.min.y.min(point.y);
        self.min.z = self.min.z.min(point.z);
        self.max.x = self.max.x.max(point.x);
        self.max.y = self.max.y.max(point.y);
        self.max.z = self.max.z.max(point.z);
    }
}

pub(crate) fn bootstrap_scene(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    gizmo_settings: Res<gizmos::ViewerGizmoSettings>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
    request: Res<SceneRequest>,
) {
    let (scene, textures) = load_scene(&request.path).unwrap_or_else(|error| {
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

    log_scene_summary(&scene, &textures);
    spawn_scene_entities(
        &mut commands,
        &mut images,
        &mut meshes,
        &mut materials,
        &scene,
        &textures,
    );
    orbit::spawn_orbit_camera(&mut commands, &scene);
    gizmos::spawn_gizmo_overlay(&mut commands, &gizmo_settings);

    commands.insert_resource(scene);
}

fn load_scene(path: &Path) -> Result<(LoadedScene, Vec<DecodedTexture>), Box<dyn Error>> {
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
    let (bounds_min, bounds_max) = bounds_for_model(&model);
    let bounds_center = (bounds_min + bounds_max) * 0.5;
    let bounds_radius = ((bounds_max - bounds_min) * 0.5).length().max(1.0);
    let title = build_window_title(path, &model);

    Ok((
        LoadedScene {
            path: path.to_path_buf(),
            model,
            bounds_min,
            bounds_max,
            bounds_center,
            bounds_radius,
            title,
        },
        textures,
    ))
}

fn spawn_scene_entities(
    commands: &mut Commands,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    scene: &LoadedScene,
    textures: &[DecodedTexture],
) {
    let texture_handles: Vec<Handle<Image>> = textures
        .iter()
        .map(|texture| images.add(texture.image.clone()))
        .collect();
    let texture_has_alpha: Vec<bool> = textures.iter().map(|texture| texture.has_alpha).collect();
    let model_transform = scene.model_transform();
    let mut spawned_any = false;

    if scene.model.primitives().is_empty() {
        let mesh = meshes.add(scene.model.geometry().to_mesh());
        let material = materials.add(default_viewer_material());

        commands.spawn((
            Name::new("PMX Model"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            ViewerPrimitive,
            model_transform,
        ));
        spawned_any = true;
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
                ViewerPrimitive,
                model_transform,
            ));
            spawned_any = true;
        }
    }

    if !spawned_any {
        let mesh = meshes.add(scene.model.geometry().to_mesh());
        let material = materials.add(default_viewer_material());

        commands.spawn((
            Name::new("PMX Model"),
            Mesh3d(mesh),
            MeshMaterial3d(material),
            ViewerPrimitive,
            model_transform,
        ));
    }

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

fn log_scene_summary(scene: &LoadedScene, textures: &[DecodedTexture]) {
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
    info!("Morphs: {}", scene.model.morph_records().len());
    info!("Textures: {}", scene.model.texture_paths().len());
    info!("Decoded textures: {}", textures.len());
    for (index, texture) in textures.iter().enumerate() {
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
    info!(
        "Bones: {} ({} roots)",
        scene.model.bone_records().len(),
        scene.model.root_bones().count()
    );
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

fn bounds_for_model(model: &Pmx) -> (Vec3, Vec3) {
    let Some(first_position) = model.geometry().positions.first() else {
        return (Vec3::ZERO, Vec3::ONE);
    };

    let mut bounds = Bounds::from_point(Vec3::from(*first_position));
    for position in &model.geometry().positions[1..] {
        bounds.include(Vec3::from(*position));
    }
    for bone in model.bone_records() {
        bounds.include(Vec3::from(bone.position));
    }

    (bounds.min, bounds.max)
}
