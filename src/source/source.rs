use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxFolderSource {
    pub root: PathBuf,
}

impl Default for PmxFolderSource {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
        }
    }
}

impl PmxFolderSource {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        let path = path.as_ref();
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }

    pub fn resolve_case_insensitive(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        let path = path.as_ref();
        if path.is_absolute() {
            return if path.exists() {
                Some(path.to_path_buf())
            } else {
                None
            };
        }

        let mut current = self.root.clone();
        for component in path.components() {
            match component {
                Component::CurDir => {}
                Component::ParentDir => {
                    if !current.pop() {
                        return None;
                    }
                }
                Component::Normal(part) => {
                    let direct = current.join(part);
                    if direct.exists() {
                        current = direct;
                        continue;
                    }

                    let target = part.to_string_lossy().to_lowercase();
                    let mut matched = None;
                    let entries = fs::read_dir(&current).ok()?;
                    for entry in entries {
                        let entry = entry.ok()?;
                        if entry.file_name().to_string_lossy().to_lowercase() == target {
                            matched = Some(entry.path());
                            break;
                        }
                    }
                    current = matched?;
                }
                Component::RootDir | Component::Prefix(_) => {
                    return None;
                }
            }
        }

        Some(current)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PmxSource {
    Folder(PmxFolderSource),
}

impl Default for PmxSource {
    fn default() -> Self {
        Self::Folder(PmxFolderSource::default())
    }
}

impl PmxSource {
    pub fn folder(root: impl Into<PathBuf>) -> Self {
        Self::Folder(PmxFolderSource::new(root))
    }

    pub fn as_folder(&self) -> Option<&PmxFolderSource> {
        match self {
            Self::Folder(source) => Some(source),
        }
    }

    pub fn root(&self) -> &Path {
        match self {
            Self::Folder(source) => source.root(),
        }
    }

    pub fn resolve(&self, path: impl AsRef<Path>) -> PathBuf {
        match self {
            Self::Folder(source) => source.resolve(path),
        }
    }

    pub fn resolve_case_insensitive(&self, path: impl AsRef<Path>) -> Option<PathBuf> {
        match self {
            Self::Folder(source) => source.resolve_case_insensitive(path),
        }
    }
}

impl From<PmxFolderSource> for PmxSource {
    fn from(source: PmxFolderSource) -> Self {
        Self::Folder(source)
    }
}
