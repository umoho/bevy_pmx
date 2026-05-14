use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, io::Reader},
    prelude::{FromWorld, Image, Resource, World},
    reflect::TypePath,
};
use std::path::{Path, PathBuf};

use crate::{
    asset::{Pmx, PmxMaterialAsset},
    error::{PmxError, PmxResult},
    format::PmxDocument,
    import::{PmxImportContext, import_pmx},
    labels::PmxAssetLabel,
    resolver::{PmxResolvedPath, PmxResolverSettings},
    source::PmxSource,
};

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
    /// Reserved for later stages. Currently a no-op because physics runtime assets are not built
    /// yet.
    pub load_physics: bool,
    /// Retains the original parsed PMX document in `Pmx::raw_document`.
    pub keep_raw_document: bool,
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
        let document = PmxDocument::from_bytes(&bytes)?;

        let source = PmxSource::folder(source_root_for_load_context(load_context));
        let import_context = PmxImportContext {
            source: Some(source.clone()),
            resolver: self.settings.resolver.clone(),
            keep_raw_document: self.settings.keep_raw_document,
        };

        let mut model = import_pmx(document, &import_context).model;

        if self.settings.load_textures {
            let texture_paths = model.texture_paths.clone();
            let mut textures = Vec::with_capacity(texture_paths.len());
            for texture in &texture_paths {
                let asset_path = texture_asset_path(load_context.path(), &source, texture)?;
                textures.push(load_context.load::<Image>(asset_path));
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

        if self.settings.load_meshes {
            let mesh = model.geometry.to_mesh();
            let mesh_handle = load_context.add_labeled_asset(PmxAssetLabel::Mesh.to_string(), mesh);
            model = model.with_mesh_handle(mesh_handle);
        }

        Ok(model)
    }

    fn extensions(&self) -> &[&str] {
        &["pmx"]
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

fn texture_asset_path(
    model_path: &AssetPath<'static>,
    source: &PmxSource,
    texture: &PmxResolvedPath,
) -> PmxResult<AssetPath<'static>> {
    let relative = texture
        .relative_to(source.root())
        .ok_or(PmxError::InvalidFormat(
            "resolved texture path is not rooted in the PMX source",
        ))?;
    let relative = relative.to_str().ok_or(PmxError::InvalidFormat(
        "resolved texture path is not valid UTF-8",
    ))?;

    model_path.resolve_embed(relative).map_err(|_| {
        PmxError::InvalidFormat("resolved texture path cannot be converted to an asset path")
    })
}

#[cfg(test)]
mod tests {
    use super::texture_asset_path;
    use crate::asset::{PmxMaterialAsset, PmxMaterialRecord};
    use crate::resolver::PmxResolvedPath;
    use crate::source::PmxSource;
    use bevy::asset::AssetPath;
    use bevy::prelude::Image;
    use std::path::{Path, PathBuf};

    #[test]
    fn converts_resolved_texture_path_into_a_source_preserving_asset_path() {
        let model_path = AssetPath::parse("remote://private/foo/model.pmx");
        let source = PmxSource::folder(PathBuf::from("assets/private/foo"));
        let texture =
            PmxResolvedPath::new("face.png", Path::new("assets/private/foo/Texture/Face.PNG"));

        let asset_path = texture_asset_path(&model_path, &source, &texture)
            .expect("should convert into an asset path");

        assert_eq!(
            asset_path.to_string(),
            "remote://private/foo/Texture/Face.PNG"
        );
        assert_eq!(asset_path.source().as_str(), Some("remote"));
    }

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
