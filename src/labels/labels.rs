use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PmxAssetLabel {
    Root,
    Mesh,
    Texture(usize),
    Material(usize),
    Bone(usize),
    Morph(usize),
    RigidBody(usize),
    Joint(usize),
    SoftBody(usize),
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
            Self::RigidBody(index) => write!(f, "RigidBody/{index}"),
            Self::Joint(index) => write!(f, "Joint/{index}"),
            Self::SoftBody(index) => write!(f, "SoftBody/{index}"),
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

    #[test]
    fn material_labels_match_the_actual_material_subasset_name() {
        assert_eq!(PmxAssetLabel::Material(7).to_string(), "Material/7");
    }

    #[test]
    fn bone_labels_match_the_actual_bone_subasset_name() {
        assert_eq!(PmxAssetLabel::Bone(7).to_string(), "Bone/7");
    }

    #[test]
    fn morph_labels_match_the_actual_morph_subasset_name() {
        assert_eq!(PmxAssetLabel::Morph(7).to_string(), "Morph/7");
    }

    #[test]
    fn physics_labels_match_the_actual_physics_subasset_name() {
        assert_eq!(PmxAssetLabel::RigidBody(7).to_string(), "RigidBody/7");
        assert_eq!(PmxAssetLabel::Joint(7).to_string(), "Joint/7");
        assert_eq!(PmxAssetLabel::SoftBody(7).to_string(), "SoftBody/7");
    }
}
