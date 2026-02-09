//! Platform-independent core types and traits for GPUI.
//!
//! This crate contains the platform abstraction boundary — trait definitions
//! and their dependent types that are shared between native and web backends.
//! It has no native OS dependencies and is designed to compile for any target
//! including `wasm32`.

mod atlas;
pub mod bounds_tree;
mod color;
mod content_mask;
mod geometry;
pub mod scene;
mod shared_string;
mod shared_uri;

pub use atlas::*;
pub use bounds_tree::*;
pub use color::*;
pub use content_mask::*;
pub use geometry::*;
pub use scene::*;
pub use shared_string::*;
pub use shared_uri::*;
