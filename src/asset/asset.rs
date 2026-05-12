use crate::format::PmxDocument;

#[derive(Debug, Clone)]
pub struct Pmx {
    pub document: PmxDocument,
    pub primitives: Vec<PmxPrimitive>,
}

impl Default for Pmx {
    fn default() -> Self {
        Self {
            document: PmxDocument::default(),
            primitives: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxPrimitive {
    pub material_index: usize,
    pub index_start: usize,
    pub index_count: usize,
}
