#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PmxAssetLabel {
    Root,
    Texture(usize),
    Material(usize),
    Bone(usize),
    Morph(usize),
    Primitive(usize),
}
