#[derive(Debug, Clone)]
pub struct PmxLoaderSettings {
    pub load_textures: bool,
    pub load_meshes: bool,
    pub load_bones: bool,
    pub load_morphs: bool,
    pub load_physics: bool,
    pub keep_raw_document: bool,
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
        }
    }
}

#[derive(Debug, Clone)]
pub struct PmxLoader {
    pub settings: PmxLoaderSettings,
}

impl Default for PmxLoader {
    fn default() -> Self {
        Self {
            settings: PmxLoaderSettings::default(),
        }
    }
}
