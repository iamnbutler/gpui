use crate::{Bounds, DisplayId, Pixels, PlatformDisplay, Point, px};
use anyhow::Result;
use uuid::Uuid;

/// Represents the browser viewport as a display.
#[derive(Debug)]
pub(crate) struct WebDisplay {
    id: DisplayId,
    uuid: Uuid,
    bounds: Bounds<Pixels>,
}

impl WebDisplay {
    pub fn new() -> Self {
        // Default to a reasonable browser viewport size.
        // In a real WASM build, this would query the actual viewport via web-sys.
        Self {
            id: DisplayId(1),
            uuid: Uuid::new_v4(),
            bounds: Bounds::from_corners(Point::default(), Point::new(px(1920.), px(1080.))),
        }
    }

    pub fn set_bounds(&mut self, bounds: Bounds<Pixels>) {
        self.bounds = bounds;
    }
}

impl PlatformDisplay for WebDisplay {
    fn id(&self) -> DisplayId {
        self.id
    }

    fn uuid(&self) -> Result<Uuid> {
        Ok(self.uuid)
    }

    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }
}
