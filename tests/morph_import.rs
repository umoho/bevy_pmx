use bevy_pmx::{
    format::{PmxMorphPanel, PmxSphereMode},
    prelude::*,
};

fn sample_document() -> PmxDocument {
    PmxDocument {
        header: PmxHeader {
            model_name: "morph sample".to_owned(),
            model_name_english: "morph sample".to_owned(),
            ..PmxHeader::default()
        },
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
        textures: Vec::new(),
        materials: vec![PmxMaterial {
            name: "material".to_owned(),
            name_english: "material".to_owned(),
            diffuse: [1.0, 1.0, 1.0, 1.0],
            specular: [0.0, 0.0, 0.0],
            specular_strength: 1.0,
            ambient: [0.0, 0.0, 0.0],
            flags: PmxMaterialFlags::default(),
            edge_color: [0.0, 0.0, 0.0, 0.0],
            edge_size: 1.0,
            texture_index: -1,
            sphere_texture_index: -1,
            sphere_mode: PmxSphereMode::Disabled,
            toon_sharing: false,
            toon_texture_index: -1,
            comment: String::new(),
            surface_count: 3,
        }],
        bones: Vec::new(),
        morphs: vec![
            PmxMorph {
                name: "vertex_morph".to_owned(),
                name_english: "vertex_morph".to_owned(),
                panel: PmxMorphPanel::Eye,
                kind: PmxMorphKind::Vertex,
                offsets: vec![PmxMorphOffset::Vertex {
                    vertex_index: 1,
                    offset: [0.25, 0.5, 0.75],
                }],
            },
            PmxMorph {
                name: "flip_morph".to_owned(),
                name_english: "flip_morph".to_owned(),
                panel: PmxMorphPanel::Other,
                kind: PmxMorphKind::Flip,
                offsets: vec![PmxMorphOffset::Flip {
                    morph_index: 0,
                    influence: 0.75,
                }],
            },
        ],
        display_frames: Vec::new(),
        rigid_bodies: Vec::new(),
        joints: Vec::new(),
        soft_bodies: Vec::new(),
    }
}

fn import_model(document: PmxDocument, keep_raw_document: bool) -> Pmx {
    import_pmx(
        document,
        &PmxImportContext {
            keep_raw_document,
            ..PmxImportContext::default()
        },
    )
    .model
}

#[test]
fn morph_records_keep_document_order_with_or_without_raw_document() {
    let document = sample_document();
    let expected_records = PmxMorphRecord::from_document(&document.morphs);

    let kept = import_model(document.clone(), true);
    let dropped = import_model(document, false);

    assert!(kept.raw_document().is_some());
    assert!(dropped.raw_document().is_none());

    assert_eq!(kept.morph_records(), expected_records.as_slice());
    assert_eq!(dropped.morph_records(), expected_records.as_slice());
    assert_eq!(kept.morph_records(), dropped.morph_records());

    assert!(kept.morph_handles().is_empty());
    assert!(dropped.morph_handles().is_empty());
}

#[test]
fn morph_record_from_document_preserves_kind_and_offsets() {
    let expected_offsets = vec![PmxMorphOffset::Flip {
        morph_index: 0,
        influence: 0.5,
    }];
    let morph = PmxMorph {
        name: "flip_morph".to_owned(),
        name_english: "flip_morph".to_owned(),
        panel: PmxMorphPanel::Other,
        kind: PmxMorphKind::Flip,
        offsets: expected_offsets.clone(),
    };

    let records = PmxMorphRecord::from_document(&[morph.clone()]);

    assert_eq!(records.len(), 1);
    assert_eq!(records[0].kind, morph.kind);
    assert_eq!(records[0].offsets, expected_offsets);
}
