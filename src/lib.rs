//! Bevy PMX importer.
//!
//! Module responsibilities:
//! - `format`: only parse PMX binary data, no Bevy dependency.
//! - `source`: locate files on disk or inside zip archives.
//! - `resolver`: turn PMX texture strings into concrete paths.
//! - `import`: convert PMX data into Bevy-friendly structures.
//! - `bone`: runtime bone records and hierarchy helpers.
//! - `morph`: runtime morph records and lightweight helpers.
//! - `physics`: runtime physics records and data-preserving helpers.
//! - `asset`: public asset types such as `Pmx`, `PmxMaterialAsset`, `PmxPrimitive`, and
//!   `PmxMeshGeometry`.
//! - `loader`: Bevy asset loader entry and settings.
//! - `plugin`: Bevy plugin registration and configuration.

#![forbid(unsafe_code)]

pub mod bone;
pub mod error;
pub mod format;
pub mod import;
pub mod labels;
pub mod loader;
pub mod plugin;
pub mod resolver;
pub mod source;

pub mod morph;
pub mod physics;

#[path = "asset.rs"]
mod asset_impl;

pub mod asset {
    pub use super::asset_impl::*;
    pub use crate::physics::{PmxJointRecord, PmxRigidBodyRecord, PmxSoftBodyRecord};
}

pub use asset::{Pmx, PmxMaterialAsset, PmxMaterialRecord, PmxMeshGeometry, PmxPrimitive};
pub use bone::PmxBoneRecord;
pub use error::{PmxError, PmxResult};
pub use format::{
    PmxDocument, PmxHeader, PmxJoint, PmxJointKind, PmxRigidBody, PmxRigidBodyMode,
    PmxRigidBodyShape, PmxSoftBody, PmxSoftBodyShape, parse_pmx,
};
pub use import::{PmxImportContext, PmxImportResult, import_pmx, resolve_textures};
pub use labels::PmxAssetLabel;
pub use loader::{PmxLoader, PmxLoaderSettings};
pub use morph::PmxMorphRecord;
pub use physics::{PmxJointRecord, PmxRigidBodyRecord, PmxSoftBodyRecord};
pub use plugin::PmxPlugin;
pub use resolver::{PmxResolvedPath, PmxResolver, PmxResolverSettings};
pub use source::{PmxFolderSource, PmxSource, PmxSourceLocation, PmxZipSource, ZipNameEncoding};

pub mod prelude {
    pub use crate::asset::{
        Pmx, PmxMaterialAsset, PmxMaterialRecord, PmxMeshGeometry, PmxPrimitive,
    };
    pub use crate::bone::PmxBoneRecord;
    pub use crate::error::{PmxError, PmxResult};
    pub use crate::format::parse_pmx;
    pub use crate::format::{
        PmxBone, PmxBoneAxes, PmxBoneFlags, PmxBoneInheritance, PmxBoneTail, PmxDisplayFrame,
        PmxDisplayFrameItem, PmxDocument, PmxHeader, PmxIk, PmxIkLink, PmxJoint, PmxJointKind,
        PmxMaterial, PmxMaterialFlags, PmxMaterialMorph, PmxMaterialMorphOperation, PmxMorph,
        PmxMorphKind, PmxMorphOffset, PmxRigidBody, PmxRigidBodyMode, PmxRigidBodyShape,
        PmxSoftBody, PmxSoftBodyShape, PmxTextEncoding, PmxTexture, PmxVertex, PmxVertexWeight,
    };
    pub use crate::import::{PmxImportContext, PmxImportResult, import_pmx, resolve_textures};
    pub use crate::labels::PmxAssetLabel;
    pub use crate::loader::{PmxLoader, PmxLoaderSettings};
    pub use crate::morph::PmxMorphRecord;
    pub use crate::physics::{PmxJointRecord, PmxRigidBodyRecord, PmxSoftBodyRecord};
    pub use crate::plugin::PmxPlugin;
    pub use crate::resolver::{PmxResolvedPath, PmxResolver, PmxResolverSettings};
    pub use crate::source::{
        PmxFolderSource, PmxSource, PmxSourceLocation, PmxZipSource, ZipNameEncoding,
    };
}
