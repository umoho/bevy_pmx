use bevy::{asset::Asset, reflect::TypePath};

use crate::format::PmxDocument;

#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
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

impl Pmx {
    pub fn new(document: PmxDocument, primitives: Vec<PmxPrimitive>) -> Self {
        Self {
            document,
            primitives,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxPrimitive {
    pub material_index: usize,
    pub index_start: usize,
    pub index_count: usize,
}
