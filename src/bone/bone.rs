use bevy::{asset::Asset, reflect::TypePath};

use crate::format::{PmxBone, PmxBoneAxes, PmxBoneFlags, PmxBoneInheritance, PmxBoneTail, PmxIk};

/// Runtime PMX bone data.
///
/// The raw PMX bone fields are preserved, and the resolved parent/child relationship is stored on
/// top so callers can traverse the hierarchy without relying on the raw document.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxBoneRecord {
    pub name: String,
    pub name_english: String,
    pub position: [f32; 3],
    /// Raw PMX parent index preserved from the source document.
    pub parent_bone: i32,
    /// Resolved parent index inside `Pmx::bone_records`.
    pub parent_index: Option<usize>,
    pub layer: i32,
    pub flags: PmxBoneFlags,
    pub tail: PmxBoneTail,
    pub inheritance: Option<PmxBoneInheritance>,
    pub fixed_axis: Option<[f32; 3]>,
    pub local_axes: Option<PmxBoneAxes>,
    pub external_parent: i32,
    pub ik: Option<PmxIk>,
    /// Child bone indices in document order.
    pub children: Vec<usize>,
}

impl PmxBoneRecord {
    /// Build runtime bone records from the raw PMX bone list.
    ///
    /// The returned vector keeps the original document order, resolves `parent_index`, and fills
    /// each bone's `children` list.
    pub fn from_document(bones: &[PmxBone]) -> Vec<Self> {
        let mut records = bones
            .iter()
            .cloned()
            .map(Self::from_bone)
            .collect::<Vec<_>>();
        let bone_count = records.len();

        for index in 0..bone_count {
            if let Some(parent_index) =
                resolve_parent_index(records[index].parent_bone, index, bone_count)
            {
                records[index].parent_index = Some(parent_index);
                records[parent_index].children.push(index);
            }
        }

        records
    }

    /// True when the bone has no resolved parent.
    pub fn is_root(&self) -> bool {
        self.parent_index.is_none()
    }

    fn from_bone(bone: PmxBone) -> Self {
        Self {
            name: bone.name,
            name_english: bone.name_english,
            position: bone.position,
            parent_bone: bone.parent_bone,
            parent_index: None,
            layer: bone.layer,
            flags: bone.flags,
            tail: bone.tail,
            inheritance: bone.inheritance,
            fixed_axis: bone.fixed_axis,
            local_axes: bone.local_axes,
            external_parent: bone.external_parent,
            ik: bone.ik,
            children: Vec::new(),
        }
    }
}

fn resolve_parent_index(raw_parent: i32, bone_index: usize, bone_count: usize) -> Option<usize> {
    if raw_parent < 0 {
        return None;
    }

    let parent_index = raw_parent as usize;
    if parent_index >= bone_count || parent_index == bone_index {
        None
    } else {
        Some(parent_index)
    }
}

#[cfg(test)]
mod tests {
    use super::PmxBoneRecord;
    use crate::format::{PmxBone, PmxBoneFlags, PmxBoneTail};

    fn bone(name: &str, parent_bone: i32, position: [f32; 3], tail: PmxBoneTail) -> PmxBone {
        PmxBone {
            name: name.to_owned(),
            name_english: name.to_owned(),
            position,
            parent_bone,
            layer: 0,
            flags: PmxBoneFlags::default(),
            tail,
            inheritance: None,
            fixed_axis: None,
            local_axes: None,
            external_parent: -1,
            ik: None,
        }
    }

    #[test]
    fn builds_document_order_hierarchy_from_raw_bones() {
        let bones = vec![
            bone(
                "root",
                -1,
                [0.0, 0.0, 0.0],
                PmxBoneTail::Offset([0.0, 1.0, 0.0]),
            ),
            bone(
                "child",
                0,
                [0.0, 1.0, 0.0],
                PmxBoneTail::Offset([0.0, 1.0, 0.0]),
            ),
        ];

        let records = PmxBoneRecord::from_document(&bones);

        assert_eq!(records.len(), 2);
        assert!(records[0].is_root());
        assert_eq!(records[0].parent_bone, -1);
        assert_eq!(records[0].children, vec![1]);
        assert_eq!(records[1].parent_bone, 0);
        assert_eq!(records[1].parent_index, Some(0));
        assert!(records[1].children.is_empty());
        assert!(!records[1].is_root());
    }
}
