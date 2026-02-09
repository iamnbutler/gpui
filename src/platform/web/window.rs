use crate::{
    AnyWindowHandle, AtlasKey, AtlasTextureId, AtlasTextureKind, AtlasTile, Bounds,
    DevicePixels, DispatchEventResult, GpuSpecs, Pixels, PlatformAtlas, PlatformDisplay,
    PlatformInput, PlatformInputHandler, PlatformWindow, Point, PromptButton,
    RequestFrameOptions, Scene, Size, TileId, WindowAppearance, WindowBackgroundAppearance,
    WindowBounds, WindowControlArea, WindowParams,
};
use collections::HashMap;
use parking_lot::Mutex;
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use std::{
    rc::Rc,
    sync::Arc,
};

pub(crate) struct WebWindow {
    bounds: Bounds<Pixels>,
    display: Rc<dyn PlatformDisplay>,
    sprite_atlas: Arc<dyn PlatformAtlas>,
    title: Option<String>,
    input_handler: Option<PlatformInputHandler>,
    request_frame_callback: Option<Box<dyn FnMut(RequestFrameOptions)>>,
    input_callback: Option<Box<dyn FnMut(PlatformInput) -> DispatchEventResult>>,
    active_status_change_callback: Option<Box<dyn FnMut(bool)>>,
    hover_status_change_callback: Option<Box<dyn FnMut(bool)>>,
    resize_callback: Option<Box<dyn FnMut(Size<Pixels>, f32)>>,
    moved_callback: Option<Box<dyn FnMut()>>,
    should_close_callback: Option<Box<dyn FnMut() -> bool>>,
    close_callback: Option<Box<dyn FnOnce()>>,
    appearance_changed_callback: Option<Box<dyn FnMut()>>,
}

impl WebWindow {
    pub fn new(
        _handle: AnyWindowHandle,
        params: WindowParams,
        display: Rc<dyn PlatformDisplay>,
    ) -> Self {
        Self {
            bounds: params.bounds,
            display,
            sprite_atlas: Arc::new(WebAtlas::new()),
            title: None,
            input_handler: None,
            request_frame_callback: None,
            input_callback: None,
            active_status_change_callback: None,
            hover_status_change_callback: None,
            resize_callback: None,
            moved_callback: None,
            should_close_callback: None,
            close_callback: None,
            appearance_changed_callback: None,
        }
    }
}

impl HasWindowHandle for WebWindow {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        // In a real WASM build, this would return a WebCanvasWindowHandle.
        Err(raw_window_handle::HandleError::NotSupported)
    }
}

impl HasDisplayHandle for WebWindow {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        // In a real WASM build, this would return a WebDisplayHandle.
        Err(raw_window_handle::HandleError::NotSupported)
    }
}

impl PlatformWindow for WebWindow {
    fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    fn is_maximized(&self) -> bool {
        // Browser windows are always effectively maximized within their viewport.
        true
    }

    fn window_bounds(&self) -> WindowBounds {
        WindowBounds::Windowed(self.bounds)
    }

    fn content_size(&self) -> Size<Pixels> {
        self.bounds.size
    }

    fn resize(&mut self, size: Size<Pixels>) {
        self.bounds.size = size;
    }

    fn scale_factor(&self) -> f32 {
        // In a real WASM build, this would query window.devicePixelRatio.
        1.0
    }

    fn appearance(&self) -> WindowAppearance {
        // In a real WASM build, this would query prefers-color-scheme.
        WindowAppearance::Light
    }

    fn display(&self) -> Option<Rc<dyn PlatformDisplay>> {
        Some(self.display.clone())
    }

    fn mouse_position(&self) -> Point<Pixels> {
        Point::default()
    }

    fn modifiers(&self) -> crate::Modifiers {
        crate::Modifiers::default()
    }

    fn capslock(&self) -> crate::Capslock {
        crate::Capslock::default()
    }

    fn set_input_handler(&mut self, input_handler: PlatformInputHandler) {
        self.input_handler = Some(input_handler);
    }

    fn take_input_handler(&mut self) -> Option<PlatformInputHandler> {
        self.input_handler.take()
    }

    fn prompt(
        &self,
        _level: crate::PromptLevel,
        _msg: &str,
        _detail: Option<&str>,
        _answers: &[PromptButton],
    ) -> Option<futures::channel::oneshot::Receiver<usize>> {
        // In a real WASM build, this could use window.confirm() or a custom dialog.
        None
    }

    fn activate(&self) {
        // In a browser, the window is always active.
    }

    fn is_active(&self) -> bool {
        true
    }

    fn is_hovered(&self) -> bool {
        false
    }

    fn set_title(&mut self, title: &str) {
        self.title = Some(title.to_owned());
        // In a real WASM build, this would set document.title.
    }

    fn set_background_appearance(&self, _background: WindowBackgroundAppearance) {}

    fn minimize(&self) {
        // No-op in browser context.
    }

    fn zoom(&self) {
        // No-op in browser context.
    }

    fn toggle_fullscreen(&self) {
        // In a real WASM build, this would use the Fullscreen API.
    }

    fn is_fullscreen(&self) -> bool {
        false
    }

    fn on_request_frame(&self, callback: Box<dyn FnMut(RequestFrameOptions)>) {
        // SAFETY: WebWindow is single-threaded (not Send/Sync),
        // so interior mutability via raw pointer is safe here.
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).request_frame_callback = Some(callback) };
    }

    fn on_input(&self, callback: Box<dyn FnMut(PlatformInput) -> DispatchEventResult>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).input_callback = Some(callback) };
    }

    fn on_active_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).active_status_change_callback = Some(callback) };
    }

    fn on_hover_status_change(&self, callback: Box<dyn FnMut(bool)>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).hover_status_change_callback = Some(callback) };
    }

    fn on_resize(&self, callback: Box<dyn FnMut(Size<Pixels>, f32)>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).resize_callback = Some(callback) };
    }

    fn on_moved(&self, callback: Box<dyn FnMut()>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).moved_callback = Some(callback) };
    }

    fn on_should_close(&self, callback: Box<dyn FnMut() -> bool>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).should_close_callback = Some(callback) };
    }

    fn on_close(&self, callback: Box<dyn FnOnce()>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).close_callback = Some(callback) };
    }

    fn on_hit_test_window_control(
        &self,
        _callback: Box<dyn FnMut() -> Option<WindowControlArea>>,
    ) {
        // No native window controls in browser.
    }

    fn on_appearance_changed(&self, callback: Box<dyn FnMut()>) {
        let self_ptr = self as *const Self as *mut Self;
        unsafe { (*self_ptr).appearance_changed_callback = Some(callback) };
    }

    fn draw(&self, _scene: &Scene) {
        // TODO: Implement wgpu-based rendering.
        // This is where the Scene will be rendered to the WebGPU surface.
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.sprite_atlas.clone()
    }

    fn gpu_specs(&self) -> Option<GpuSpecs> {
        None
    }

    fn update_ime_position(&self, _bounds: Bounds<Pixels>) {}
}

// Simple atlas implementation for the web platform.
// This will be replaced with a wgpu texture atlas once the renderer is built.
struct WebAtlasState {
    next_id: u32,
    tiles: HashMap<AtlasKey, AtlasTile>,
}

struct WebAtlas(Mutex<WebAtlasState>);

impl WebAtlas {
    fn new() -> Self {
        Self(Mutex::new(WebAtlasState {
            next_id: 0,
            tiles: HashMap::default(),
        }))
    }
}

impl PlatformAtlas for WebAtlas {
    fn get_or_insert_with<'a>(
        &self,
        key: &AtlasKey,
        build: &mut dyn FnMut() -> anyhow::Result<
            Option<(Size<DevicePixels>, std::borrow::Cow<'a, [u8]>)>,
        >,
    ) -> anyhow::Result<Option<AtlasTile>> {
        let mut state = self.0.lock();
        if let Some(tile) = state.tiles.get(key) {
            return Ok(Some(tile.clone()));
        }
        drop(state);

        let Some((size, _bytes)) = build()? else {
            return Ok(None);
        };

        let mut state = self.0.lock();
        state.next_id += 1;
        let texture_id = state.next_id;
        state.next_id += 1;
        let tile_id = state.next_id;

        let tile = AtlasTile {
            texture_id: AtlasTextureId {
                index: texture_id,
                kind: AtlasTextureKind::Monochrome,
            },
            tile_id: TileId(tile_id),
            padding: 0,
            bounds: Bounds {
                origin: Point::default(),
                size,
            },
        };
        state.tiles.insert(key.clone(), tile.clone());
        Ok(Some(tile))
    }

    fn remove(&self, key: &AtlasKey) {
        self.0.lock().tiles.remove(key);
    }
}
