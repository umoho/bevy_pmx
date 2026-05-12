use bevy::{
    asset::{AssetLoader, LoadContext, io::Reader},
    prelude::{FromWorld, Resource, World},
    reflect::TypePath,
};

use crate::{
    asset::Pmx,
    error::{PmxError, PmxResult},
    format::PmxDocument,
    import::{PmxImportContext, import_pmx, resolve_textures},
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

        let source = load_context
            .path()
            .path()
            .parent()
            .map(|path| PmxSource::folder(path.to_path_buf()));
        let import_context = PmxImportContext {
            source,
            resolver: self.settings.resolver.clone(),
            keep_raw_document: self.settings.keep_raw_document,
        };

        if self.settings.load_textures {
            let _ = resolve_textures(&document, &import_context);
        }

        let mut result = import_pmx(document, &import_context);
        if !self.settings.load_meshes {
            result.model.primitives.clear();
        }

        Ok(result.model)
    }

    fn extensions(&self) -> &[&str] {
        &["pmx"]
    }
}
