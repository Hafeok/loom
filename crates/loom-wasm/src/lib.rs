//! loom-wasm — the wasm-bindgen twin of the loom-ffi handle
//! (pat-ffi-handle). The exported surface is exactly the mode-contract:
//! dispatch, subscribe, mount, unmount (p-ffi-surface-minimal).

pub use loom_ffi::LoomComponent;

#[cfg(target_arch = "wasm32")]
mod bindings {
    use wasm_bindgen::prelude::*;

    use loom_ffi::LoomComponent;
    use loom_machine::MachineDefinition;

    #[wasm_bindgen]
    pub struct LoomComponentHandle {
        inner: LoomComponent,
    }

    #[wasm_bindgen]
    impl LoomComponentHandle {
        #[wasm_bindgen(constructor)]
        pub fn new(definition: JsValue) -> Result<LoomComponentHandle, JsValue> {
            let definition: MachineDefinition = serde_wasm_bindgen::from_value(definition)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            Ok(Self {
                inner: LoomComponent::new(definition),
            })
        }

        pub fn mount(&self) {
            self.inner.mount();
        }

        pub fn unmount(&self) {
            self.inner.unmount();
        }

        pub fn dispatch(&self, event: &str) {
            self.inner.dispatch(event);
        }

        pub fn subscribe(&self, callback: js_sys::Function) {
            self.inner.subscribe(std::sync::Arc::new(move |descriptor| {
                let value = serde_wasm_bindgen::to_value(descriptor).unwrap_or(JsValue::NULL);
                let _ = callback.call1(&JsValue::NULL, &value);
            }));
        }
    }
}
