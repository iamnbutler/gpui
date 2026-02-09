use crate::{PlatformDispatcher, Priority, RealtimePriority, RunnableVariant, TaskLabel, TaskTiming, ThreadTaskTimings};
use std::time::Duration;

pub(crate) struct WebDispatcher;

impl WebDispatcher {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformDispatcher for WebDispatcher {
    fn get_all_timings(&self) -> Vec<ThreadTaskTimings> {
        Vec::new()
    }

    fn get_current_thread_timings(&self) -> Vec<TaskTiming> {
        Vec::new()
    }

    fn is_main_thread(&self) -> bool {
        // WASM is single-threaded; everything runs on the main thread.
        true
    }

    fn dispatch(&self, runnable: RunnableVariant, _label: Option<TaskLabel>, _priority: Priority) {
        // In WASM, "background" work is just queued on the microtask queue.
        // wasm_bindgen_futures::spawn_local would be used in a real WASM build;
        // for now we run synchronously since we're single-threaded.
        execute_runnable(runnable);
    }

    fn dispatch_on_main_thread(&self, runnable: RunnableVariant, _priority: Priority) {
        // Everything is already on the main thread in WASM.
        execute_runnable(runnable);
    }

    fn dispatch_after(&self, _duration: Duration, runnable: RunnableVariant) {
        // In a real WASM build, this would use setTimeout.
        // For now, execute immediately.
        execute_runnable(runnable);
    }

    fn spawn_realtime(&self, _priority: RealtimePriority, f: Box<dyn FnOnce() + Send>) {
        // No real-time threads in WASM; just run it.
        f();
    }
}

fn execute_runnable(runnable: RunnableVariant) {
    match runnable {
        RunnableVariant::Meta(runnable) => { runnable.run(); }
        RunnableVariant::Compat(runnable) => { runnable.run(); }
    }
}
