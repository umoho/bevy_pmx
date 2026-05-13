use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PmxAssetLabel {
    Root,
    Mesh,
    Texture(usize),
    Material(usize),
    Bone(usize),
    Morph(usize),
    Primitive(usize),
}

impl fmt::Display for PmxAssetLabel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => f.write_str("Root"),
            Self::Mesh => f.write_str("Mesh"),
            Self::Texture(index) => write!(f, "Texture/{index}"),
            Self::Material(index) => write!(f, "Material/{index}"),
            Self::Bone(index) => write!(f, "Bone/{index}"),
            Self::Morph(index) => write!(f, "Morph/{index}"),
            Self::Primitive(index) => write!(f, "Primitive/{index}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PmxAssetLabel;

    #[test]
    fn mesh_labels_match_the_actual_mesh_subasset_name() {
        assert_eq!(PmxAssetLabel::Mesh.to_string(), "Mesh");
        assert_eq!(PmxAssetLabel::Texture(3).to_string(), "Texture/3");
    }
}
