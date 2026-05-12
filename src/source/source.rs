use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxFolderSource {
    pub root: PathBuf,
}

impl PmxFolderSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PmxSource {
    Folder(PmxFolderSource),
}

impl PmxSource {
    pub fn folder(root: impl Into<PathBuf>) -> Self {
        Self::Folder(PmxFolderSource::new(root))
    }
}

impl From<PmxFolderSource> for PmxSource {
    fn from(source: PmxFolderSource) -> Self {
        Self::Folder(source)
    }
}
