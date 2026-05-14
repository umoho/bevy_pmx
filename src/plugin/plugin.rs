use bevy::{
    asset::AssetApp,
    prelude::{App, Plugin},
};

use crate::{
    asset::{Pmx, PmxMaterialAsset},
    bone::PmxBoneRecord,
    loader::{PmxLoader, PmxLoaderSettings},
};

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

impl Plugin for PmxPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<Pmx>();
        app.init_asset::<PmxMaterialAsset>();
        app.init_asset::<PmxBoneRecord>();
        app.insert_resource(self.settings.clone());
        app.init_asset_loader::<PmxLoader>();
    }
}
