use crate::loader::PmxLoaderSettings;

#[derive(Debug, Clone)]
pub struct PmxPlugin {
    pub settings: PmxLoaderSettings,
}

impl PmxPlugin {
    pub fn new() -> Self {
        Self {
            settings: PmxLoaderSettings::default(),
        }
    }

    pub fn with_settings(settings: PmxLoaderSettings) -> Self {
        Self { settings }
    }
}

impl Default for PmxPlugin {
    fn default() -> Self {
        Self::new()
    }
}
