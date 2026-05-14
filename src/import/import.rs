use crate::{
    PmxMorphRecord,
    asset::{Pmx, PmxMaterialRecord, PmxMeshGeometry, PmxPrimitive},
    bone::PmxBoneRecord,
    format::{PmxDocument, PmxMaterial},
    resolver::{PmxResolvedPath, PmxResolver, PmxResolverSettings},
    source::PmxSource,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmxImportContext {
    pub source: Option<PmxSource>,
    pub resolver: PmxResolverSettings,
    /// Controls whether the imported `Pmx` keeps the raw `PmxDocument`.
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
}

pub fn import_pmx(document: PmxDocument, context: &PmxImportContext) -> PmxImportResult {
    let resolved_textures = resolve_textures(&document, context);
    let geometry = PmxMeshGeometry::from_document(&document);
    let primitives = build_primitives(&document.materials);
    let material_records = build_material_records(&document.materials);
    let morph_records = PmxMorphRecord::from_document(&document.morphs);
    let bone_records = PmxBoneRecord::from_document(&document.bones);
    let raw_document = context.keep_raw_document.then(|| document.clone());

    let model = Pmx::new(raw_document, geometry, primitives)
        .with_texture_paths(resolved_textures)
        .with_material_records(material_records)
        .with_morph_records(morph_records)
        .with_bone_records(bone_records);

    PmxImportResult { model }
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

fn build_material_records(materials: &[PmxMaterial]) -> Vec<PmxMaterialRecord> {
    materials
        .iter()
        .cloned()
        .map(PmxMaterialRecord::new)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PmxImportContext, PmxMeshGeometry, import_pmx};
    use crate::{
        format::{
            PmxBone, PmxBoneFlags, PmxBoneTail, PmxDocument, PmxHeader, PmxMaterial,
            PmxMaterialFlags, PmxSphereMode, PmxTexture, PmxVertex, PmxVertexWeight,
        },
        source::PmxSource,
    };

    fn sample_document() -> PmxDocument {
        PmxDocument {
            header: PmxHeader::default(),
            vertices: vec![
                PmxVertex {
                    position: [0.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 0.0],
                    additional_uvs: Vec::new(),
                    weight: PmxVertexWeight::Bdef1 { bone: -1 },
                    edge_scale: 1.0,
                },
                PmxVertex {
                    position: [1.0, 0.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [1.0, 0.0],
                    additional_uvs: Vec::new(),
                    weight: PmxVertexWeight::Bdef1 { bone: -1 },
                    edge_scale: 1.0,
                },
                PmxVertex {
                    position: [0.0, 1.0, 0.0],
                    normal: [0.0, 0.0, 1.0],
                    uv: [0.0, 1.0],
                    additional_uvs: Vec::new(),
                    weight: PmxVertexWeight::Bdef1 { bone: -1 },
                    edge_scale: 1.0,
                },
            ],
            indices: vec![0, 1, 2],
            textures: vec![PmxTexture {
                path: "Texture/face.png".to_owned(),
            }],
            materials: vec![PmxMaterial {
                name: "mat".to_owned(),
                name_english: "mat".to_owned(),
                diffuse: [1.0, 1.0, 1.0, 1.0],
                specular: [0.0, 0.0, 0.0],
                specular_strength: 1.0,
                ambient: [0.0, 0.0, 0.0],
                flags: PmxMaterialFlags::default(),
                edge_color: [0.0, 0.0, 0.0, 0.0],
                edge_size: 1.0,
                texture_index: 0,
                sphere_texture_index: -1,
                sphere_mode: PmxSphereMode::Disabled,
                toon_sharing: false,
                toon_texture_index: -1,
                comment: String::new(),
                surface_count: 3,
            }],
            bones: vec![
                PmxBone {
                    name: "root".to_owned(),
                    name_english: "root".to_owned(),
                    position: [0.0, 0.0, 0.0],
                    parent_bone: -1,
                    layer: 0,
                    flags: PmxBoneFlags::default(),
                    tail: PmxBoneTail::Offset([0.0, 1.0, 0.0]),
                    inheritance: None,
                    fixed_axis: None,
                    local_axes: None,
                    external_parent: -1,
                    ik: None,
                },
                PmxBone {
                    name: "child".to_owned(),
                    name_english: "child".to_owned(),
                    position: [0.0, 1.0, 0.0],
                    parent_bone: 0,
                    layer: 0,
                    flags: PmxBoneFlags::default(),
                    tail: PmxBoneTail::Offset([0.0, 1.0, 0.0]),
                    inheritance: None,
                    fixed_axis: None,
                    local_axes: None,
                    external_parent: -1,
                    ik: None,
                },
            ],
            morphs: Vec::new(),
            display_frames: Vec::new(),
            rigid_bodies: Vec::new(),
            joints: Vec::new(),
            soft_bodies: Vec::new(),
        }
    }

    #[test]
    fn import_context_defaults_to_retaining_raw_documents() {
        let context = PmxImportContext::default();

        assert!(context.keep_raw_document);
    }

    #[test]
    fn import_pmx_keeps_or_drops_the_raw_document_without_changing_geometry() {
        let source = PmxSource::folder("assets/private/MMD_派蒙");
        let keep_context = PmxImportContext {
            source: Some(source.clone()),
            keep_raw_document: true,
            ..PmxImportContext::default()
        };
        let drop_context = PmxImportContext {
            source: Some(source),
            keep_raw_document: false,
            ..PmxImportContext::default()
        };
        let document = sample_document();

        let kept = import_pmx(document.clone(), &keep_context);
        assert!(kept.model.raw_document().is_some());
        assert_eq!(
            kept.model.geometry.positions,
            document
                .vertices
                .iter()
                .map(|v| v.position)
                .collect::<Vec<_>>()
        );
        assert_eq!(kept.model.material_records.len(), 1);
        assert_eq!(kept.model.material_records[0].material.name, "mat");
        assert_eq!(kept.model.primitives.len(), 1);
        assert_eq!(kept.model.primitives[0].index_count, 3);
        assert_eq!(kept.model.primitives[0].material_index, 0);
        assert_eq!(kept.model.texture_paths.len(), 1);
        assert_eq!(kept.model.bone_records().len(), 2);
        assert_eq!(kept.model.bone_records()[0].children, vec![1]);
        assert_eq!(kept.model.bone_records()[1].parent_index, Some(0));
        assert_eq!(kept.model.root_bones().count(), 1);

        let dropped = import_pmx(document, &drop_context);
        assert!(dropped.model.raw_document().is_none());
        assert_eq!(dropped.model.geometry.indices, vec![0, 1, 2]);
        assert_eq!(dropped.model.material_records.len(), 1);
        assert_eq!(dropped.model.bone_records().len(), 2);
        assert_eq!(dropped.model.bone_records()[0].children, vec![1]);
        assert_eq!(dropped.model.root_bones().count(), 1);
    }

    #[test]
    fn mesh_geometry_can_be_materialized_into_a_bevy_mesh() {
        let document = sample_document();
        let geometry = PmxMeshGeometry::from_document(&document);
        let mesh = geometry.to_mesh();

        assert!(mesh.contains_attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION));
        assert!(mesh.contains_attribute(bevy::mesh::Mesh::ATTRIBUTE_NORMAL));
        assert!(mesh.contains_attribute(bevy::mesh::Mesh::ATTRIBUTE_UV_0));
        assert_eq!(mesh.count_vertices(), 3);

        let positions = mesh
            .attribute(bevy::mesh::Mesh::ATTRIBUTE_POSITION)
            .and_then(|values| values.as_float3())
            .expect("position attribute should be float3");
        assert_eq!(
            positions,
            &[[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]]
        );

        let indices = mesh.indices().expect("mesh should contain indices");
        match indices {
            bevy::mesh::Indices::U32(values) => assert_eq!(values.as_slice(), &[0, 1, 2]),
            bevy::mesh::Indices::U16(values) => panic!("expected u32 indices, got {values:?}"),
        }
    }
}
