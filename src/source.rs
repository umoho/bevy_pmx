//! PMX resource source abstraction.
//!
//! Start with folder-based resources, then extend to zip-based packages
//! without changing the parser or import pipeline.

pub mod source;

pub use self::source::*;
