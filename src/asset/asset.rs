use bevy::{
    asset::Asset,
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::{Handle, Image},
    reflect::TypePath,
};

use crate::{
    format::{PmxBone, PmxDocument},
    resolver::PmxResolvedPath,
};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PmxMeshGeometry {
    /// Triangle list positions extracted from the PMX vertex buffer.
    pub positions: Vec<[f32; 3]>,
    /// Triangle list normals extracted from the PMX vertex buffer.
    pub normals: Vec<[f32; 3]>,
    /// Primary texture coordinates extracted from the PMX vertex buffer.
    pub uvs: Vec<[f32; 2]>,
    /// Triangle indices extracted from the PMX index buffer.
    pub indices: Vec<u32>,
}

impl PmxMeshGeometry {
    pub fn from_document(document: &PmxDocument) -> Self {
        let mut positions = Vec::with_capacity(document.vertices.len());
        let mut normals = Vec::with_capacity(document.vertices.len());
        let mut uvs = Vec::with_capacity(document.vertices.len());

        for vertex in &document.vertices {
            positions.push(vertex.position);
            normals.push(vertex.normal);
            uvs.push(vertex.uv);
        }

        Self {
            positions,
            normals,
            uvs,
            indices: document.indices.clone(),
        }
    }

    pub fn to_mesh(&self) -> Mesh {
        debug_assert_eq!(self.positions.len(), self.normals.len());
        debug_assert_eq!(self.positions.len(), self.uvs.len());

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(self.indices.clone()));
        mesh
    }
}

/// Root PMX asset produced by the loader.
///
/// `raw_document` is present only when `keep_raw_document` is enabled. `geometry` and
/// `primitives` are always kept so the model can be rendered or re-materialized later, and
/// `mesh_handle` is filled when the loader materializes a Bevy `Mesh` subasset.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct Pmx {
    pub raw_document: Option<PmxDocument>,
    pub geometry: PmxMeshGeometry,
    pub textures: Vec<Handle<Image>>,
    pub texture_paths: Vec<PmxResolvedPath>,
    pub mesh_handle: Option<Handle<Mesh>>,
    pub primitives: Vec<PmxPrimitive>,
}

impl Default for Pmx {
    fn default() -> Self {
        Self {
            raw_document: None,
            geometry: PmxMeshGeometry::default(),
            textures: Vec::new(),
            texture_paths: Vec::new(),
            mesh_handle: None,
            primitives: Vec::new(),
        }
    }
}

impl Pmx {
    pub fn new(
        raw_document: Option<PmxDocument>,
        geometry: PmxMeshGeometry,
        primitives: Vec<PmxPrimitive>,
    ) -> Self {
        Self {
            raw_document,
            geometry,
            textures: Vec::new(),
            texture_paths: Vec::new(),
            mesh_handle: None,
            primitives,
        }
    }

    pub fn with_texture_paths(mut self, texture_paths: Vec<PmxResolvedPath>) -> Self {
        self.texture_paths = texture_paths;
        self
    }

    pub fn with_textures(mut self, textures: Vec<Handle<Image>>) -> Self {
        self.textures = textures;
        self
    }

    pub fn with_mesh_handle(mut self, mesh_handle: Handle<Mesh>) -> Self {
        self.mesh_handle = Some(mesh_handle);
        self
    }

    pub fn raw_document(&self) -> Option<&PmxDocument> {
        self.raw_document.as_ref()
    }

    pub fn document(&self) -> Option<&PmxDocument> {
        self.raw_document()
    }

    pub fn geometry(&self) -> &PmxMeshGeometry {
        &self.geometry
    }

    pub fn textures(&self) -> &[Handle<Image>] {
        &self.textures
    }

    pub fn texture_paths(&self) -> &[PmxResolvedPath] {
        &self.texture_paths
    }

    pub fn mesh_handle(&self) -> Option<&Handle<Mesh>> {
        self.mesh_handle.as_ref()
    }

    pub fn primitives(&self) -> &[PmxPrimitive] {
        &self.primitives
    }

    pub fn bones(&self) -> &[PmxBone] {
        self.raw_document
            .as_ref()
            .map_or(&[], |document| document.bones.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxPrimitive {
    pub material_index: usize,
    pub index_start: usize,
    pub index_count: usize,
}
