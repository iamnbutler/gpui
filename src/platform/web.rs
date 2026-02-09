mod dispatcher;
mod display;
mod platform;
mod renderer;
mod window;

pub(crate) use dispatcher::*;
pub(crate) use display::*;
pub(crate) use platform::*;
pub(crate) use window::*;

/// Web platform does not support screen capture.
pub(crate) type PlatformScreenCaptureFrame = ();
