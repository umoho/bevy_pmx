use bevy::{
    asset::{AssetLoader, AssetPath, LoadContext, io::Reader},
    prelude::{FromWorld, Image, Resource, World},
    reflect::TypePath,
};
use std::path::{Path, PathBuf};

use crate::{
    asset::Pmx,
    error::{PmxError, PmxResult},
    format::PmxDocument,
    import::{PmxImportContext, import_pmx},
    resolver::{PmxResolvedPath, PmxResolverSettings},
    source::PmxSource,
};

#[derive(Debug, Clone, Resource, PartialEq, Eq)]
pub struct PmxLoaderSettings {
    pub load_textures: bool,
    pub load_meshes: bool,
    /// Reserved for later stages. Currently a no-op because the raw PMX document already
    /// retains bone data.
    pub load_bones: bool,
    /// Reserved for later stages. Currently a no-op because the raw PMX document already
    /// retains morph data.
    pub load_morphs: bool,
    /// Reserved for later stages. Currently a no-op because the raw PMX document already
    /// retains physics data.
    pub load_physics: bool,
    pub keep_raw_document: bool,
    pub resolver: PmxResolverSettings,
}

impl Default for PmxLoaderSettings {
    fn default() -> Self {
        Self {
            load_textures: true,
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

        let mut result = import_pmx(document, &import_context);
        if let Some(source_document) = result.source_document.take() {
            result.model.document = source_document;
        }

        result.model.texture_paths = result.resolved_textures.clone();

        if self.settings.load_textures {
            let mut textures = Vec::with_capacity(result.resolved_textures.len());
            for texture in &result.resolved_textures {
                let asset_path = texture_asset_path(load_context.path(), &source, texture)?;
                textures.push(load_context.load::<Image>(asset_path));
            }
            result.model.textures = textures;
        }
        if !self.settings.load_meshes {
            result.model.primitives.clear();
        }

        Ok(result.model)
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
    use crate::resolver::PmxResolvedPath;
    use crate::source::PmxSource;
    use bevy::asset::AssetPath;
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
}
