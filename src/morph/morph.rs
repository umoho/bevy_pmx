use bevy::{asset::Asset, reflect::TypePath};

use crate::format::{PmxMorph, PmxMorphKind, PmxMorphOffset, PmxMorphPanel};

/// Runtime PMX morph record preserved in document order.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxMorphRecord {
    pub name: String,
    pub name_english: String,
    pub panel: PmxMorphPanel,
    pub kind: PmxMorphKind,
    pub offsets: Vec<PmxMorphOffset>,
}

impl PmxMorphRecord {
    /// Build runtime morph records from the raw PMX morph list.
    pub fn from_document(morphs: &[PmxMorph]) -> Vec<Self> {
        morphs.iter().cloned().map(Self::from_morph).collect()
    }

    fn from_morph(morph: PmxMorph) -> Self {
        Self {
            name: morph.name,
            name_english: morph.name_english,
            panel: morph.panel,
            kind: morph.kind,
            offsets: morph.offsets,
        }
    }
}

impl From<PmxMorph> for PmxMorphRecord {
    fn from(morph: PmxMorph) -> Self {
        Self::from_morph(morph)
    }
}
