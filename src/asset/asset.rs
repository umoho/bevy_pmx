use bevy::{
    asset::Asset,
    prelude::{Handle, Image},
    reflect::TypePath,
};

use crate::{
    format::{PmxBone, PmxDocument},
    resolver::PmxResolvedPath,
};

#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct Pmx {
    pub document: PmxDocument,
    pub textures: Vec<Handle<Image>>,
    pub texture_paths: Vec<PmxResolvedPath>,
    pub primitives: Vec<PmxPrimitive>,
}

impl Default for Pmx {
    fn default() -> Self {
        Self {
            document: PmxDocument::default(),
            textures: Vec::new(),
            texture_paths: Vec::new(),
            primitives: Vec::new(),
        }
    }
}

impl Pmx {
    pub fn new(document: PmxDocument, primitives: Vec<PmxPrimitive>) -> Self {
        Self {
            document,
            textures: Vec::new(),
            texture_paths: Vec::new(),
            primitives,
        }
    }

    pub fn with_textures(
        document: PmxDocument,
        textures: Vec<Handle<Image>>,
        primitives: Vec<PmxPrimitive>,
    ) -> Self {
        Self {
            document,
            textures,
            texture_paths: Vec::new(),
            primitives,
        }
    }

    pub fn with_texture_paths(
        document: PmxDocument,
        texture_paths: Vec<PmxResolvedPath>,
        primitives: Vec<PmxPrimitive>,
    ) -> Self {
        Self {
            document,
            textures: Vec::new(),
            texture_paths,
            primitives,
        }
    }

    pub fn textures(&self) -> &[Handle<Image>] {
        &self.textures
    }

    pub fn texture_paths(&self) -> &[PmxResolvedPath] {
        &self.texture_paths
    }

    pub fn primitives(&self) -> &[PmxPrimitive] {
        &self.primitives
    }

    pub fn bones(&self) -> &[PmxBone] {
        &self.document.bones
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxPrimitive {
    pub material_index: usize,
    pub index_start: usize,
    pub index_count: usize,
}
