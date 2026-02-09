//! Platform-independent core types and traits for GPUI.
//!
//! This crate contains the platform abstraction boundary — trait definitions
//! and their dependent types that are shared between native and web backends.
//! It has no native OS dependencies and is designed to compile for any target
//! including `wasm32`.

mod color;
mod geometry;
mod shared_string;
mod shared_uri;

pub use color::*;
pub use geometry::*;
pub use shared_string::*;
pub use shared_uri::*;
