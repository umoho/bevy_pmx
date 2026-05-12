use std::path::PathBuf;

use crate::{asset::Pmx, format::PmxDocument};

#[derive(Debug, Clone, Default)]
pub struct PmxImportContext {
    pub source_path: Option<PathBuf>,
    pub keep_raw_document: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PmxImportResult {
    pub model: Pmx,
    pub source_document: Option<PmxDocument>,
}
