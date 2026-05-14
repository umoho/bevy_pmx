use bevy::{
    asset::Asset,
    asset::RenderAssetUsages,
    mesh::{Indices, Mesh, PrimitiveTopology},
    prelude::{Handle, Image},
    reflect::TypePath,
};

use crate::{
    bone::PmxBoneRecord,
    format::{PmxBone, PmxDocument, PmxMaterial},
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
        self.mesh_with_indices(&self.indices)
    }

    pub fn to_mesh_for_primitive(&self, primitive: PmxPrimitive) -> Mesh {
        let index_end = primitive
            .index_start
            .checked_add(primitive.index_count)
            .expect("PMX primitive index range overflowed");
        let indices = self
            .indices
            .get(primitive.index_start..index_end)
            .unwrap_or_else(|| {
                panic!(
                    "PMX primitive index range {}..{} exceeds geometry index buffer length {}",
                    primitive.index_start,
                    index_end,
                    self.indices.len()
                )
            });

        self.mesh_with_indices(indices)
    }

    fn mesh_with_indices(&self, indices: &[u32]) -> Mesh {
        debug_assert_eq!(self.positions.len(), self.normals.len());
        debug_assert_eq!(self.positions.len(), self.uvs.len());

        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals.clone());
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, self.uvs.clone());
        mesh.insert_indices(Indices::U32(indices.to_vec()));
        mesh
    }
}

/// Imported PMX material data kept on the root asset.
///
/// This preserves the original `format::PmxMaterial` information even when the raw document is
/// dropped, so the loader can later materialize a runtime material subasset.
#[derive(Debug, Clone, PartialEq)]
pub struct PmxMaterialRecord {
    pub material: PmxMaterial,
}

impl PmxMaterialRecord {
    pub fn new(material: PmxMaterial) -> Self {
        Self { material }
    }
}

/// Runtime PMX material subasset produced by the loader.
///
/// The raw PMX material is preserved in `material`, while the optional texture handles point to
/// already loaded Bevy images. `shared_toon_index` is used when the PMX material references one of
/// the shared toon textures instead of a document texture.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxMaterialAsset {
    pub material: PmxMaterial,
    #[dependency]
    pub diffuse_texture: Option<Handle<Image>>,
    #[dependency]
    pub sphere_texture: Option<Handle<Image>>,
    #[dependency]
    pub toon_texture: Option<Handle<Image>>,
    pub shared_toon_index: Option<usize>,
}

impl PmxMaterialAsset {
    pub fn from_record(record: &PmxMaterialRecord, textures: &[Handle<Image>]) -> Self {
        let material = record.material.clone();
        let diffuse_texture = texture_handle(textures, material.texture_index);
        let sphere_texture = texture_handle(textures, material.sphere_texture_index);
        let (toon_texture, shared_toon_index) = if material.toon_sharing {
            (
                None,
                (material.toon_texture_index >= 0).then_some(material.toon_texture_index as usize),
            )
        } else {
            (texture_handle(textures, material.toon_texture_index), None)
        };

        Self {
            material,
            diffuse_texture,
            sphere_texture,
            toon_texture,
            shared_toon_index,
        }
    }
}

fn texture_handle(textures: &[Handle<Image>], texture_index: i32) -> Option<Handle<Image>> {
    (texture_index >= 0)
        .then(|| textures.get(texture_index as usize).cloned())
        .flatten()
}

#[cfg(test)]
mod tests {
    use super::{PmxMeshGeometry, PmxPrimitive};

    #[test]
    fn primitive_mesh_uses_only_its_index_range() {
        let geometry = PmxMeshGeometry {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
            ],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            indices: vec![0, 1, 2, 0, 2, 3],
        };
        let primitive = PmxPrimitive {
            material_index: 0,
            index_start: 3,
            index_count: 3,
        };

        let mesh = geometry.to_mesh_for_primitive(primitive);

        assert_eq!(mesh.count_vertices(), 4);

        let indices = mesh.indices().expect("mesh should contain indices");
        match indices {
            bevy::mesh::Indices::U32(values) => assert_eq!(values, &vec![0, 2, 3]),
            bevy::mesh::Indices::U16(values) => panic!("expected u32 indices, got {values:?}"),
        }
    }
}

/// Root PMX asset produced by the loader.
///
/// `raw_document` is present only when `keep_raw_document` is enabled. `geometry`,
/// `primitives`, `material_records`, and `bone_records` are always kept so the model can be
/// rendered or re-materialized later, and `mesh_handle` / `material_handles` /
/// `bone_handles` are filled when the loader materializes Bevy subassets.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct Pmx {
    pub raw_document: Option<PmxDocument>,
    pub geometry: PmxMeshGeometry,
    pub textures: Vec<Handle<Image>>,
    pub texture_paths: Vec<PmxResolvedPath>,
    /// Imported PMX material records kept so the loader can materialize subassets.
    pub material_records: Vec<PmxMaterialRecord>,
    /// PMX material subasset handles in the same order as `material_records`.
    pub material_handles: Vec<Handle<PmxMaterialAsset>>,
    /// Imported PMX bone records kept so the loader can materialize subassets.
    pub bone_records: Vec<PmxBoneRecord>,
    /// PMX bone subasset handles in the same order as `bone_records`.
    pub bone_handles: Vec<Handle<PmxBoneRecord>>,
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
            material_records: Vec::new(),
            material_handles: Vec::new(),
            bone_records: Vec::new(),
            bone_handles: Vec::new(),
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
            material_records: Vec::new(),
            material_handles: Vec::new(),
            bone_records: Vec::new(),
            bone_handles: Vec::new(),
            mesh_handle: None,
            primitives,
        }
    }

    pub fn with_texture_paths(mut self, texture_paths: Vec<PmxResolvedPath>) -> Self {
        self.texture_paths = texture_paths;
        self
    }

    pub fn with_material_records(mut self, material_records: Vec<PmxMaterialRecord>) -> Self {
        self.material_records = material_records;
        self
    }

    pub fn with_material_handles(
        mut self,
        material_handles: Vec<Handle<PmxMaterialAsset>>,
    ) -> Self {
        self.material_handles = material_handles;
        self
    }

    pub fn with_bone_records(mut self, bone_records: Vec<PmxBoneRecord>) -> Self {
        self.bone_records = bone_records;
        self
    }

    pub fn with_bone_handles(mut self, bone_handles: Vec<Handle<PmxBoneRecord>>) -> Self {
        self.bone_handles = bone_handles;
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

    pub fn material_records(&self) -> &[PmxMaterialRecord] {
        &self.material_records
    }

    pub fn material_handles(&self) -> &[Handle<PmxMaterialAsset>] {
        &self.material_handles
    }

    pub fn bone_records(&self) -> &[PmxBoneRecord] {
        &self.bone_records
    }

    pub fn bone_handles(&self) -> &[Handle<PmxBoneRecord>] {
        &self.bone_handles
    }

    pub fn mesh_handle(&self) -> Option<&Handle<Mesh>> {
        self.mesh_handle.as_ref()
    }

    pub fn primitives(&self) -> &[PmxPrimitive] {
        &self.primitives
    }

    pub fn root_bones(&self) -> impl Iterator<Item = &PmxBoneRecord> + '_ {
        self.bone_records.iter().filter(|bone| bone.is_root())
    }

    /// Raw bone definitions from `raw_document`, if the raw document is still retained.
    pub fn bones(&self) -> &[PmxBone] {
        self.raw_document
            .as_ref()
            .map_or(&[], |document| document.bones.as_slice())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PmxPrimitive {
    /// Index into `Pmx::material_records` and `Pmx::material_handles`.
    pub material_index: usize,
    pub index_start: usize,
    pub index_count: usize,
}
