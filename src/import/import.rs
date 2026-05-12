use crate::{
    asset::{Pmx, PmxPrimitive},
    format::{PmxDocument, PmxMaterial},
    resolver::{PmxResolvedPath, PmxResolver, PmxResolverSettings},
    source::PmxSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl PmxImportContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_source(source: impl Into<PmxSource>) -> Self {
        Self {
            source: Some(source.into()),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct PmxImportResult {
    pub model: Pmx,
    pub source_document: Option<PmxDocument>,
}

pub fn import_pmx(document: PmxDocument, context: &PmxImportContext) -> PmxImportResult {
    let primitives = build_primitives(&document.materials);

    if context.keep_raw_document {
        PmxImportResult {
            model: Pmx::new(document, primitives),
            source_document: None,
        }
    } else {
        PmxImportResult {
            model: Pmx::new(PmxDocument::default(), primitives),
            source_document: Some(document),
        }
    }
}

pub fn resolve_textures(
    document: &PmxDocument,
    context: &PmxImportContext,
) -> Vec<PmxResolvedPath> {
    let resolver = PmxResolver::with_settings(context.resolver.clone());
    document
        .textures
        .iter()
        .map(|texture| resolver.resolve_texture_path(context.source.as_ref(), &texture.path))
        .collect()
}

fn build_primitives(materials: &[PmxMaterial]) -> Vec<PmxPrimitive> {
    let mut primitives = Vec::with_capacity(materials.len());
    let mut index_start = 0usize;

    for (material_index, material) in materials.iter().enumerate() {
        let index_count = material.surface_count as usize;
        primitives.push(PmxPrimitive {
            material_index,
            index_start,
            index_count,
        });
        index_start += index_count;
    }

    primitives
}
