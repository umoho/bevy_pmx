use bevy::{asset::Asset, reflect::TypePath};

use crate::format::{PmxJoint, PmxRigidBody, PmxSoftBody};

/// Runtime PMX rigid body record preserved in document order.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxRigidBodyRecord {
    pub rigid_body: PmxRigidBody,
}

impl PmxRigidBodyRecord {
    /// Build runtime rigid body records from the raw PMX rigid body list.
    pub fn from_document(rigid_bodies: &[PmxRigidBody]) -> Vec<Self> {
        rigid_bodies
            .iter()
            .cloned()
            .map(|rigid_body| Self { rigid_body })
            .collect()
    }
}

impl From<PmxRigidBody> for PmxRigidBodyRecord {
    fn from(rigid_body: PmxRigidBody) -> Self {
        Self { rigid_body }
    }
}

/// Runtime PMX joint record preserved in document order.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxJointRecord {
    pub joint: PmxJoint,
}

impl PmxJointRecord {
    /// Build runtime joint records from the raw PMX joint list.
    pub fn from_document(joints: &[PmxJoint]) -> Vec<Self> {
        joints.iter().cloned().map(|joint| Self { joint }).collect()
    }
}

impl From<PmxJoint> for PmxJointRecord {
    fn from(joint: PmxJoint) -> Self {
        Self { joint }
    }
}

/// Runtime PMX soft body record preserved in document order.
#[derive(Debug, Clone, PartialEq, Asset, TypePath)]
pub struct PmxSoftBodyRecord {
    pub soft_body: PmxSoftBody,
}

impl PmxSoftBodyRecord {
    /// Build runtime soft body records from the raw PMX soft body list.
    pub fn from_document(soft_bodies: &[PmxSoftBody]) -> Vec<Self> {
        soft_bodies
            .iter()
            .cloned()
            .map(|soft_body| Self { soft_body })
            .collect()
    }
}

impl From<PmxSoftBody> for PmxSoftBodyRecord {
    fn from(soft_body: PmxSoftBody) -> Self {
        Self { soft_body }
    }
}
