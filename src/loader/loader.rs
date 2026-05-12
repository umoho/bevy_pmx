use bevy::prelude::Resource;

use crate::resolver::PmxResolverSettings;

#[derive(Debug, Clone, Resource)]
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

#[derive(Debug, Clone)]
pub struct PmxLoader {
    pub settings: PmxLoaderSettings,
}

impl PmxLoader {
    pub fn new(settings: PmxLoaderSettings) -> Self {
        Self { settings }
    }
}

impl Default for PmxLoader {
    fn default() -> Self {
        Self {
            settings: PmxLoaderSettings::default(),
        }
    }
}

impl From<PmxLoaderSettings> for PmxLoader {
    fn from(settings: PmxLoaderSettings) -> Self {
        Self::new(settings)
    }
}
