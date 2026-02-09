//! Extension functions for geometry types that require access to the application context.

use crate::{App, Bounds, DisplayId, Pixels, Size, point, px, size};

/// Generate centered bounds for the given display or primary display if none is provided.
pub fn centered_bounds(
    display_id: Option<DisplayId>,
    size: Size<Pixels>,
    cx: &App,
) -> Bounds<Pixels> {
    let display = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display());

    display
        .map(|display| Bounds::centered_at(display.bounds().center(), size))
        .unwrap_or_else(|| Bounds {
            origin: point(px(0.), px(0.)),
            size,
        })
}

/// Generate maximized bounds for the given display or primary display if none is provided.
pub fn maximized_bounds(display_id: Option<DisplayId>, cx: &App) -> Bounds<Pixels> {
    let display = display_id
        .and_then(|id| cx.find_display(id))
        .or_else(|| cx.primary_display());

    display
        .map(|display| display.bounds())
        .unwrap_or_else(|| Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1024.), px(768.)),
        })
}
