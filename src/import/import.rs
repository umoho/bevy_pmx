use crate::{asset::Pmx, format::PmxDocument};
use crate::{resolver::PmxResolverSettings, source::PmxSource};

#[derive(Debug, Clone)]
pub struct PmxImportContext {
    pub source: Option<PmxSource>,
    pub resolver: PmxResolverSettings,
    pub keep_raw_document: bool,
}

impl Default for PmxImportContext {
    fn default() -> Self {
        Self {
            source: None,
            resolver: PmxResolverSettings::default(),
            keep_raw_document: true,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PmxImportResult {
    pub model: Pmx,
    pub source_document: Option<PmxDocument>,
}
