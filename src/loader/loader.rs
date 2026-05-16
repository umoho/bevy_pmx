use bevy::{
    asset::{AssetLoader, LoadContext, RenderAssetUsages, io::Reader},
    image::{CompressedImageFormats, ImageSampler, ImageType},
    log::warn,
    prelude::{FromWorld, Image, Resource, World},
    reflect::TypePath,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(feature = "zip")]
use std::io::{Cursor, Read};

#[cfg(feature = "zip")]
use zip::ZipArchive;

use crate::{
    asset::{Pmx, PmxMaterialAsset},
    error::{PmxError, PmxResult},
    format::PmxDocument,
    import::{PmxImportContext, import_pmx},
    labels::PmxAssetLabel,
    resolver::{PmxResolvedPath, PmxResolverSettings},
    source::{PmxSource, PmxSourceLocation},
};

#[cfg(feature = "zip")]
use crate::source::{ZipNameEncoding, find_first_pmx_zip_entry_root};

#[derive(Debug, Clone, Resource, PartialEq, Eq)]
pub struct PmxLoaderSettings {
    pub load_textures: bool,
    /// Materializes PMX material subassets and stores their handles on `Pmx::material_handles`.
    pub load_materials: bool,
    /// Materializes a Bevy `Mesh` subasset and stores its handle on `Pmx::mesh_handle`.
    pub load_meshes: bool,
    /// Materializes PMX bone subassets and stores their handles on `Pmx::bone_handles`.
    pub load_bones: bool,
    /// Materializes PMX morph subassets and stores their handles on `Pmx::morph_handles`.
    pub load_morphs: bool,
    /// Materializes PMX physics subassets and stores their handles on the relevant `Pmx`
    /// handle lists.
    pub load_physics: bool,
    /// Retains the original parsed PMX document in `Pmx::raw_document`.
    pub keep_raw_document: bool,
    /// Decoding strategy for ZIP entry names when loading `.zip` archives.
    #[cfg(feature = "zip")]
    pub zip_name_encoding: ZipNameEncoding,
    pub resolver: PmxResolverSettings,
}

impl Default for PmxLoaderSettings {
    fn default() -> Self {
        Self {
            load_textures: true,
            load_materials: true,
            load_meshes: true,
            load_bones: true,
            load_morphs: true,
            load_physics: true,
            keep_raw_document: true,
            #[cfg(feature = "zip")]
            zip_name_encoding: ZipNameEncoding::Auto,
            resolver: PmxResolverSettings::default(),
        }
    }
}

#[derive(Debug, Clone, TypePath)]
pub struct PmxLoader {
    pub settings: PmxLoaderSettings,
}

impl PmxLoader {
    pub fn new(settings: PmxLoaderSettings) -> Self {
        Self { settings }
    }

    pub fn default() -> Self {
        Self::new(PmxLoaderSettings::default())
    }
}

impl FromWorld for PmxLoader {
    fn from_world(world: &mut World) -> Self {
        let settings = world
            .get_resource::<PmxLoaderSettings>()
            .cloned()
            .unwrap_or_default();
        Self::new(settings)
    }
}

impl From<PmxLoaderSettings> for PmxLoader {
    fn from(settings: PmxLoaderSettings) -> Self {
        Self::new(settings)
    }
}

impl AssetLoader for PmxLoader {
    type Asset = Pmx;
    type Settings = ();
    type Error = PmxError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> PmxResult<Self::Asset> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        #[cfg(feature = "zip")]
        let asset_path = load_context.path().path();
        #[cfg(feature = "zip")]
        let (document, source) = if is_zip_asset(asset_path) {
            load_zip_document(
                &archive_path_for_load_context(load_context),
                &bytes,
                self.settings.zip_name_encoding,
            )?
        } else {
            (
                PmxDocument::from_bytes(&bytes)?,
                source_for_load_context(load_context),
            )
        };

        #[cfg(not(feature = "zip"))]
        let (document, source) = (
            PmxDocument::from_bytes(&bytes)?,
            source_for_load_context(load_context),
        );

        let import_context = PmxImportContext {
            source: Some(source.clone()),
            resolver: self.settings.resolver.clone(),
            keep_raw_document: self.settings.keep_raw_document,
        };

        let mut model = import_pmx(document, &import_context).model;

        if self.settings.load_textures {
            let texture_paths = model.texture_paths.clone();
            let mut textures = Vec::with_capacity(texture_paths.len());
            for (texture_index, texture) in texture_paths.iter().enumerate() {
                let image = match load_texture_image(&source, texture) {
                    Ok(image) => image,
                    Err(error) => {
                        warn!(
                            "missing texture: {} (resolved to {}) - using magenta placeholder",
                            texture.original,
                            texture.location()
                        );
                        warn!("  read error: {error}");
                        placeholder_texture_image()
                    }
                };

                let texture_handle = load_context
                    .add_labeled_asset(PmxAssetLabel::Texture(texture_index).to_string(), image);
                textures.push(texture_handle);
            }
            model = model.with_textures(textures);
        }

        if self.settings.load_materials {
            let material_records = model.material_records.clone();
            let texture_handles = model.textures.clone();
            let mut material_handles = Vec::with_capacity(material_records.len());

            for (material_index, record) in material_records.iter().enumerate() {
                let material = PmxMaterialAsset::from_record(record, &texture_handles);
                let material_handle = load_context.add_labeled_asset(
                    PmxAssetLabel::Material(material_index).to_string(),
                    material,
                );
                material_handles.push(material_handle);
            }

            model = model.with_material_handles(material_handles);
        }

        if self.settings.load_bones {
            let bone_records = model.bone_records.clone();
            let mut bone_handles = Vec::with_capacity(bone_records.len());

            for (bone_index, record) in bone_records.iter().enumerate() {
                let bone_handle = load_context
                    .add_labeled_asset(PmxAssetLabel::Bone(bone_index).to_string(), record.clone());
                bone_handles.push(bone_handle);
            }

            model = model.with_bone_handles(bone_handles);
        }

        if self.settings.load_morphs {
            let morph_records = model.morph_records.clone();
            let mut morph_handles = Vec::with_capacity(morph_records.len());

            for (morph_index, record) in morph_records.iter().enumerate() {
                let morph_handle = load_context.add_labeled_asset(
                    PmxAssetLabel::Morph(morph_index).to_string(),
                    record.clone(),
                );
                morph_handles.push(morph_handle);
            }

            model = model.with_morph_handles(morph_handles);
        }

        if self.settings.load_physics {
            let rigid_body_records = model.rigid_body_records.clone();
            let mut rigid_body_handles = Vec::with_capacity(rigid_body_records.len());

            for (rigid_body_index, record) in rigid_body_records.iter().enumerate() {
                let rigid_body_handle = load_context.add_labeled_asset(
                    PmxAssetLabel::RigidBody(rigid_body_index).to_string(),
                    record.clone(),
                );
                rigid_body_handles.push(rigid_body_handle);
            }

            model = model.with_rigid_body_handles(rigid_body_handles);

            let joint_records = model.joint_records.clone();
            let mut joint_handles = Vec::with_capacity(joint_records.len());

            for (joint_index, record) in joint_records.iter().enumerate() {
                let joint_handle = load_context.add_labeled_asset(
                    PmxAssetLabel::Joint(joint_index).to_string(),
                    record.clone(),
                );
                joint_handles.push(joint_handle);
            }

            model = model.with_joint_handles(joint_handles);

            let soft_body_records = model.soft_body_records.clone();
            let mut soft_body_handles = Vec::with_capacity(soft_body_records.len());

            for (soft_body_index, record) in soft_body_records.iter().enumerate() {
                let soft_body_handle = load_context.add_labeled_asset(
                    PmxAssetLabel::SoftBody(soft_body_index).to_string(),
                    record.clone(),
                );
                soft_body_handles.push(soft_body_handle);
            }

            model = model.with_soft_body_handles(soft_body_handles);
        }

        if self.settings.load_meshes {
            let mesh = model.geometry.to_mesh();
            let mesh_handle = load_context.add_labeled_asset(PmxAssetLabel::Mesh.to_string(), mesh);
            model = model.with_mesh_handle(mesh_handle);
        }

        Ok(model)
    }

    fn extensions(&self) -> &[&str] {
        #[cfg(feature = "zip")]
        {
            &["pmx", "zip"]
        }

        #[cfg(not(feature = "zip"))]
        {
            &["pmx"]
        }
    }
}

fn source_root_for_load_context(load_context: &LoadContext<'_>) -> PathBuf {
    let model_root = load_context
        .path()
        .path()
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();

    if load_context.path().source().as_str().is_some() {
        model_root
    } else {
        PathBuf::from("assets").join(model_root)
    }
}

fn source_for_load_context(load_context: &LoadContext<'_>) -> PmxSource {
    #[cfg(feature = "zip")]
    {
        if load_context.path().source().as_str() == Some("zip") {
            if let Some(source) = zip_source_for_load_context(load_context) {
                return source;
            }
        }
    }

    PmxSource::folder(source_root_for_load_context(load_context))
}

#[cfg(feature = "zip")]
fn zip_source_for_load_context(load_context: &LoadContext<'_>) -> Option<PmxSource> {
    let (archive, root) = split_zip_asset_path(load_context.path().path())?;
    Some(PmxSource::zip(archive, root))
}

#[cfg(feature = "zip")]
fn archive_path_for_load_context(load_context: &LoadContext<'_>) -> PathBuf {
    source_root_for_load_context(load_context)
        .join(load_context.path().path().file_name().unwrap_or_default())
}

#[cfg(feature = "zip")]
fn is_zip_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("zip"))
}

#[cfg(feature = "zip")]
fn split_zip_asset_path(path: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut archive = PathBuf::new();
    let mut entry = PathBuf::new();
    let mut found_archive = false;

    for component in path.components() {
        if found_archive {
            entry.push(component.as_os_str());
        } else {
            archive.push(component.as_os_str());
            if component
                .as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .ends_with(".zip")
            {
                found_archive = true;
            }
        }
    }

    if found_archive && !entry.as_os_str().is_empty() {
        let root = entry.parent().map(Path::to_path_buf).unwrap_or_default();
        Some((archive, root))
    } else {
        None
    }
}

#[cfg(feature = "zip")]
fn load_zip_document(
    archive_path: &Path,
    archive_bytes: &[u8],
    name_encoding: ZipNameEncoding,
) -> PmxResult<(PmxDocument, PmxSource)> {
    let (index, _entry, root) = find_first_pmx_zip_entry_root(archive_bytes, name_encoding)?
        .ok_or(PmxError::InvalidFormat(
            "zip archive does not contain a PMX file",
        ))?;
    let source = PmxSource::zip_with_encoding(archive_path.to_path_buf(), root, name_encoding);

    let pmx_bytes = read_zip_entry_bytes(archive_bytes, index)?;
    let document = PmxDocument::from_bytes(&pmx_bytes)?;
    Ok((document, source))
}

#[cfg(feature = "zip")]
fn read_zip_entry_bytes(archive_bytes: &[u8], index: usize) -> io::Result<Vec<u8>> {
    let cursor = Cursor::new(archive_bytes);
    let mut archive = ZipArchive::new(cursor).map_err(zip_error_to_io)?;
    let mut file = archive.by_index(index).map_err(zip_error_to_io)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn load_texture_image(source: &PmxSource, texture: &PmxResolvedPath) -> Result<Image, io::Error> {
    let location = texture.location();
    let bytes = source.read_bytes(location).map_err(|error| {
        io::Error::other(format!(
            "failed to read texture {} (resolved to {}) through the source abstraction: {error}",
            texture.original, location,
        ))
    })?;
    decode_texture(location, &bytes)
}

fn decode_texture(location: &PmxSourceLocation, bytes: &[u8]) -> Result<Image, io::Error> {
    let extension = location.extension().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("texture {location} does not have a file extension"),
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
            format!("failed to decode texture {location}: {error}"),
        )
    })
}

#[cfg(feature = "zip")]
fn zip_error_to_io(error: zip::result::ZipError) -> io::Error {
    io::Error::other(error)
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

#[cfg(test)]
mod tests {
    use crate::asset::{PmxMaterialAsset, PmxMaterialRecord};
    use bevy::prelude::Image;

    #[test]
    fn material_assets_bind_loaded_texture_handles_in_document_order() {
        let record = PmxMaterialRecord::new(crate::format::PmxMaterial {
            name: "mat".to_owned(),
            name_english: "mat".to_owned(),
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [0.0, 0.0, 0.0],
            specular_strength: 1.0,
            ambient: [0.0, 0.0, 0.0],
            flags: crate::format::PmxMaterialFlags::default(),
            edge_color: [0.0, 0.0, 0.0, 0.0],
            edge_size: 1.0,
            texture_index: 0,
            sphere_texture_index: 1,
            sphere_mode: crate::format::PmxSphereMode::Multiply,
            toon_sharing: false,
            toon_texture_index: 2,
            comment: String::new(),
            surface_count: 3,
        });
        let textures = vec![
            bevy::asset::Handle::<Image>::from(bevy::asset::uuid::Uuid::from_u128(1)),
            bevy::asset::Handle::<Image>::from(bevy::asset::uuid::Uuid::from_u128(2)),
            bevy::asset::Handle::<Image>::from(bevy::asset::uuid::Uuid::from_u128(3)),
        ];

        let material = PmxMaterialAsset::from_record(&record, &textures);

        assert_eq!(material.material.name, "mat");
        assert_eq!(material.diffuse_texture.as_ref(), Some(&textures[0]));
        assert_eq!(material.sphere_texture.as_ref(), Some(&textures[1]));
        assert_eq!(material.toon_texture.as_ref(), Some(&textures[2]));
        assert_eq!(material.shared_toon_index, None);
    }

    #[test]
    fn shared_toon_materials_keep_the_shared_toon_index() {
        let record = PmxMaterialRecord::new(crate::format::PmxMaterial {
            name: "toon".to_owned(),
            name_english: "toon".to_owned(),
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [0.0, 0.0, 0.0],
            specular_strength: 1.0,
            ambient: [0.0, 0.0, 0.0],
            flags: crate::format::PmxMaterialFlags::default(),
            edge_color: [0.0, 0.0, 0.0, 0.0],
            edge_size: 1.0,
            texture_index: -1,
            sphere_texture_index: -1,
            sphere_mode: crate::format::PmxSphereMode::Disabled,
            toon_sharing: true,
            toon_texture_index: 4,
            comment: String::new(),
            surface_count: 3,
        });
        let textures: Vec<bevy::asset::Handle<Image>> = Vec::new();

        let material = PmxMaterialAsset::from_record(&record, &textures);

        assert!(material.diffuse_texture.is_none());
        assert!(material.sphere_texture.is_none());
        assert!(material.toon_texture.is_none());
        assert_eq!(material.shared_toon_index, Some(4));
    }
}
