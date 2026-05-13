use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::{FromWorld, Image, Resource, World},
    reflect::TypePath,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    asset::Pmx,
    error::{PmxError, PmxResult},
    format::PmxDocument,
    import::{PmxImportContext, import_pmx},
    resolver::PmxResolverSettings,
    source::PmxSource,
};

#[derive(Debug, Clone, Resource, PartialEq, Eq)]
pub struct PmxLoaderSettings {
    pub load_textures: bool,
    pub load_meshes: bool,
    pub load_bones: bool,
    pub load_morphs: bool,
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

        let model_root = load_context
            .path()
            .path()
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let asset_root = asset_root_for_load_context(load_context);
        let source = asset_root
            .as_ref()
            .map(|root| PmxSource::folder(root.join(&model_root)))
            .unwrap_or_else(|| PmxSource::folder(model_root));
        let import_context = PmxImportContext {
            source: Some(source),
            resolver: self.settings.resolver.clone(),
            keep_raw_document: self.settings.keep_raw_document,
        };

        let mut result = import_pmx(document, &import_context);
        if self.settings.load_textures {
            let mut textures = Vec::with_capacity(result.resolved_textures.len());
            for texture in &result.resolved_textures {
                let asset_path =
                    normalize_load_path(texture.resolved_path(), asset_root.as_deref());
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

fn asset_root_for_load_context(load_context: &LoadContext<'_>) -> Option<PathBuf> {
    if load_context.path().source().as_str().is_some() {
        return None;
    }

    fs::canonicalize("assets").ok()
}

fn normalize_load_path(resolved: &Path, asset_root: Option<&Path>) -> PathBuf {
    if let Some(asset_root) = asset_root {
        if let Ok(stripped) = resolved.strip_prefix(asset_root) {
            return stripped.to_path_buf();
        }
    }

    resolved.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::normalize_load_path;
    use std::path::Path;

    #[test]
    fn strips_assets_root_from_texture_path() {
        let asset_root = Path::new("/Users/umoho/Devs/bevy_pmx/assets");
        let resolved = Path::new("/Users/umoho/Devs/bevy_pmx/assets/private/foo/Texture/bar.png");

        let normalized = normalize_load_path(resolved, Some(asset_root));

        assert_eq!(
            normalized,
            Path::new("private/foo/Texture/bar.png").to_path_buf()
        );
    }

    #[test]
    fn leaves_unmatched_paths_untouched() {
        let resolved = Path::new("/tmp/external/bar.png");

        let normalized = normalize_load_path(resolved, None);

        assert_eq!(normalized, resolved.to_path_buf());
    }
}
