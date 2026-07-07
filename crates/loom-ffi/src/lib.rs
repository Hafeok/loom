//! loom-ffi — the mode-contract surface and nothing else (p-ffi-surface-minimal).
//!
//! `LoomComponent` is declared once in UniFFI udl with a wasm-bindgen twin:
//! dispatch(event), subscribe(descriptor-callback), mount(), unmount() —
//! the complete public surface. No raw-state getter, no transition method.

use std::sync::{Arc, Mutex};

use loom_machine::instance::{assemble_descriptor, Descriptor};
use loom_machine::{Machine, MachineDefinition};

pub type DescriptorCallback = Arc<dyn Fn(&Descriptor) + Send + Sync>;

/// The component handle — the only symbol a platform adapter may hold.
pub struct LoomComponent {
    machine: Mutex<Option<Machine>>,
    definition: MachineDefinition,
    subscribers: Mutex<Vec<DescriptorCallback>>,
}

impl LoomComponent {
    pub fn new(definition: MachineDefinition) -> Self {
        Self {
            machine: Mutex::new(None),
            definition,
            subscribers: Mutex::new(Vec::new()),
        }
    }

    /// Mount: interpret the machine from its graph-read definition.
    pub fn mount(&self) {
        let mut machine = self.machine.lock().unwrap();
        if machine.is_none() {
            *machine = Some(Machine::mount(self.definition.clone()));
        }
        drop(machine);
        self.publish(None);
    }

    /// Unmount: drop the interpreted machine.
    pub fn unmount(&self) {
        *self.machine.lock().unwrap() = None;
    }

    /// Dispatch a declared event; subscribers receive the new descriptor.
    pub fn dispatch(&self, event: &str) {
        let motion = {
            let mut machine = self.machine.lock().unwrap();
            let Some(machine) = machine.as_mut() else {
                return;
            };
            machine.dispatch(event).and_then(|t| t.carries_motion.clone())
        };
        self.publish(motion);
    }

    /// Subscribe to descriptor updates — the only way state leaves the handle.
    pub fn subscribe(&self, callback: DescriptorCallback) {
        self.subscribers.lock().unwrap().push(callback);
    }

    fn publish(&self, active_motion: Option<String>) {
        let machine = self.machine.lock().unwrap();
        let Some(machine) = machine.as_ref() else {
            return;
        };
        let mut descriptor = assemble_descriptor(machine.state(), &[], &[]);
        descriptor.active_motion = active_motion;
        for callback in self.subscribers.lock().unwrap().iter() {
            callback(&descriptor);
        }
    }
}
