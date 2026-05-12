#![forbid(unsafe_code)]

pub mod asset;
pub mod error;
pub mod format;
pub mod import;
pub mod labels;
pub mod loader;
pub mod plugin;

pub use asset::{Pmx, PmxPrimitive};
pub use error::{PmxError, PmxResult};
pub use format::{PmxDocument, PmxHeader};
pub use labels::PmxAssetLabel;
pub use loader::{PmxLoader, PmxLoaderSettings};
pub use plugin::PmxPlugin;

pub mod prelude {
    pub use crate::asset::{Pmx, PmxPrimitive};
    pub use crate::error::{PmxError, PmxResult};
    pub use crate::format::{
        PmxBone, PmxBoneAxes, PmxBoneFlags, PmxBoneInheritance, PmxBoneTail, PmxDisplayFrame,
        PmxDisplayFrameItem, PmxDocument, PmxHeader, PmxIk, PmxIkLink, PmxJoint, PmxJointKind,
        PmxMaterial, PmxMaterialFlags, PmxMaterialMorph, PmxMaterialMorphOperation, PmxMorph,
        PmxMorphKind, PmxMorphOffset, PmxRigidBody, PmxRigidBodyMode, PmxRigidBodyShape,
        PmxSoftBody, PmxSoftBodyShape, PmxTextEncoding, PmxTexture, PmxVertex, PmxVertexWeight,
    };
    pub use crate::labels::PmxAssetLabel;
    pub use crate::loader::{PmxLoader, PmxLoaderSettings};
    pub use crate::plugin::PmxPlugin;
}
