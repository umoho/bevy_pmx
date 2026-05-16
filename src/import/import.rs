use crate::{
    PmxMorphRecord,
    asset::{Pmx, PmxMaterialRecord, PmxMeshGeometry, PmxPrimitive},
    bone::PmxBoneRecord,
    format::{PmxDocument, PmxJoint, PmxMaterial, PmxRigidBody, PmxSoftBody},
    physics::{PmxJointRecord, PmxRigidBodyRecord, PmxSoftBodyRecord},
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
    let rigid_body_records = build_rigid_body_records(&document.rigid_bodies);
    let joint_records = build_joint_records(&document.joints);
    let soft_body_records = build_soft_body_records(&document.soft_bodies);
    let raw_document = context.keep_raw_document.then(|| document.clone());

    let model = Pmx::new(raw_document, geometry, primitives)
        .with_texture_paths(resolved_textures)
        .with_material_records(material_records)
        .with_morph_records(morph_records)
        .with_bone_records(bone_records)
        .with_rigid_body_records(rigid_body_records)
        .with_joint_records(joint_records)
        .with_soft_body_records(soft_body_records);

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

fn build_rigid_body_records(rigid_bodies: &[PmxRigidBody]) -> Vec<PmxRigidBodyRecord> {
    PmxRigidBodyRecord::from_document(rigid_bodies)
}

fn build_joint_records(joints: &[PmxJoint]) -> Vec<PmxJointRecord> {
    PmxJointRecord::from_document(joints)
}

fn build_soft_body_records(soft_bodies: &[PmxSoftBody]) -> Vec<PmxSoftBodyRecord> {
    PmxSoftBodyRecord::from_document(soft_bodies)
}

#[cfg(test)]
mod tests {
    use super::{PmxImportContext, PmxMeshGeometry, import_pmx};
    use crate::{
        format::{
            PmxBone, PmxBoneFlags, PmxBoneTail, PmxDocument, PmxHeader, PmxJoint, PmxJointKind,
            PmxMaterial, PmxMaterialFlags, PmxRigidBody, PmxRigidBodyMode, PmxRigidBodyShape,
            PmxSoftBody, PmxSoftBodyAeroModel, PmxSoftBodyAnchorRigidBody, PmxSoftBodyCluster,
            PmxSoftBodyConfig, PmxSoftBodyFlags, PmxSoftBodyIteration, PmxSoftBodyMaterial,
            PmxSoftBodyShape, PmxSphereMode, PmxTexture, PmxVertex, PmxVertexWeight,
        },
        source::PmxSource,
    };
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP_ID: AtomicU64 = AtomicU64::new(0);

    fn unique_temp_path(prefix: &str) -> std::path::PathBuf {
        let unique = NEXT_TEMP_ID.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!("{prefix}_{}_{}", std::process::id(), unique))
    }

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
            rigid_bodies: vec![PmxRigidBody {
                name: "rigid".to_owned(),
                name_english: "rigid".to_owned(),
                bone_index: 0,
                group: 1,
                mask: 0xffff,
                shape: PmxRigidBodyShape::Sphere,
                size: [0.5, 0.5, 0.5],
                position: [0.0, 1.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                mass: 1.0,
                linear_damping: 0.5,
                angular_damping: 0.5,
                restitution: 0.3,
                friction: 0.4,
                mode: PmxRigidBodyMode::Physics,
            }],
            joints: vec![PmxJoint {
                name: "joint".to_owned(),
                name_english: "joint".to_owned(),
                joint_type: PmxJointKind::Spring6Dof,
                body_a: 0,
                body_b: 0,
                position: [0.0, 0.0, 0.0],
                rotation: [0.0, 0.0, 0.0],
                translation_limit_min: [-1.0, -1.0, -1.0],
                translation_limit_max: [1.0, 1.0, 1.0],
                rotation_limit_min: [-0.1, -0.1, -0.1],
                rotation_limit_max: [0.1, 0.1, 0.1],
                spring_translation: [0.0, 0.0, 0.0],
                spring_rotation: [0.0, 0.0, 0.0],
            }],
            soft_bodies: vec![PmxSoftBody {
                name: "soft".to_owned(),
                name_english: "soft".to_owned(),
                shape: PmxSoftBodyShape::Rope,
                material_index: 0,
                group: 1,
                mask: 0xffff,
                flags: PmxSoftBodyFlags::BLink,
                b_link_distance: 1,
                num_clusters: 2,
                total_mass: 1.0,
                collision_margin: 0.05,
                aero_model: PmxSoftBodyAeroModel::VertexPoint,
                config: PmxSoftBodyConfig {
                    vcf: 0.1,
                    dp: 0.2,
                    dg: 0.3,
                    lf: 0.4,
                    pr: 0.5,
                    vc: 0.6,
                    df: 0.7,
                    mt: 0.8,
                    chr: 0.9,
                    khr: 1.0,
                    shr: 1.1,
                    ahr: 1.2,
                },
                cluster: PmxSoftBodyCluster {
                    srhr_cl: 1.3,
                    skhr_cl: 1.4,
                    sshr_cl: 1.5,
                    sr_splt_cl: 1.6,
                    sk_splt_cl: 1.7,
                    ss_splt_cl: 1.8,
                },
                iteration: PmxSoftBodyIteration {
                    v_it: 5,
                    p_it: 6,
                    d_it: 7,
                    c_it: 8,
                },
                material: PmxSoftBodyMaterial {
                    lst: 0.2,
                    ast: 0.3,
                    vst: 0.4,
                },
                anchor_rigid_bodies: vec![PmxSoftBodyAnchorRigidBody {
                    rigid_body_index: 0,
                    vertex_index: 1,
                    near_mode: true,
                }],
                pin_vertices: vec![2],
            }],
        }
    }

    #[test]
    fn import_context_defaults_to_retaining_raw_documents() {
        let context = PmxImportContext::default();

        assert!(context.keep_raw_document);
    }

    #[test]
    fn import_pmx_keeps_or_drops_the_raw_document_without_changing_geometry() {
        let source = PmxSource::folder(unique_temp_path("bevy_pmx_import"));
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
        assert_eq!(kept.model.rigid_body_records().len(), 1);
        assert_eq!(kept.model.rigid_body_records()[0].rigid_body.name, "rigid");
        assert_eq!(kept.model.joint_records().len(), 1);
        assert_eq!(kept.model.joint_records()[0].joint.name, "joint");
        assert_eq!(kept.model.soft_body_records().len(), 1);
        assert_eq!(kept.model.soft_body_records()[0].soft_body.name, "soft");
        assert_eq!(
            kept.model.soft_body_records()[0].soft_body.shape,
            PmxSoftBodyShape::Rope
        );
        assert_eq!(
            kept.model.soft_body_records()[0].soft_body.material_index,
            0
        );

        let dropped = import_pmx(document, &drop_context);
        assert!(dropped.model.raw_document().is_none());
        assert_eq!(dropped.model.geometry.indices, vec![0, 1, 2]);
        assert_eq!(dropped.model.material_records.len(), 1);
        assert_eq!(dropped.model.bone_records().len(), 2);
        assert_eq!(dropped.model.bone_records()[0].children, vec![1]);
        assert_eq!(dropped.model.root_bones().count(), 1);
        assert_eq!(dropped.model.rigid_body_records().len(), 1);
        assert_eq!(dropped.model.joint_records().len(), 1);
        assert_eq!(dropped.model.soft_body_records().len(), 1);
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
