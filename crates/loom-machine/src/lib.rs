//! loom-machine — the machine runtime (pat-graph-interpreter).
//!
//! A machine is data in `loom:g/machines`; this crate interprets a transition
//! structure handed over by loom-graph's typed views. One crate, compiled to
//! wasm and to static libraries — a mode is this binary plus a thin shim.

pub mod instance;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// An interpretable transition structure, read from the graph at mount.
/// The graph remains the sole authoring surface (p-machine-from-graph).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineDefinition {
    pub component: String,
    pub initial_state: String,
    /// (from-state, event) → transition.
    pub transitions: Vec<TransitionDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDef {
    pub from_state: String,
    pub on_event: String,
    pub to_state: String,
    #[serde(default)]
    pub carries_motion: Option<String>,
}

/// A mounted machine instance. Dispatch resolves against the interpreted
/// structure; there is no other way to effect a transition.
pub struct Machine {
    definition: MachineDefinition,
    table: HashMap<(String, String), usize>,
    state: String,
}

impl Machine {
    /// Mount: optionally table-compile the definition; the graph stays source.
    pub fn mount(definition: MachineDefinition) -> Self {
        let table = definition
            .transitions
            .iter()
            .enumerate()
            .map(|(i, t)| ((t.from_state.clone(), t.on_event.clone()), i))
            .collect();
        let state = definition.initial_state.clone();
        Self {
            definition,
            table,
            state,
        }
    }

    /// Dispatch a declared event. Undeclared events are ignored, not errors —
    /// the adapter's event surface is validated upstream by the mode-contract.
    pub fn dispatch(&mut self, event: &str) -> Option<&TransitionDef> {
        let idx = *self.table.get(&(self.state.clone(), event.to_string()))?;
        let transition = &self.definition.transitions[idx];
        self.state = transition.to_state.clone();
        Some(transition)
    }

    pub fn state(&self) -> &str {
        &self.state
    }

    pub fn component(&self) -> &str {
        &self.definition.component
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_follows_the_interpreted_table() {
        let mut machine = Machine::mount(MachineDefinition {
            component: "single-select".into(),
            initial_state: "loading".into(),
            transitions: vec![TransitionDef {
                from_state: "loading".into(),
                on_event: "data-arrived".into(),
                to_state: "present".into(),
                carries_motion: None,
            }],
        });
        assert_eq!(machine.state(), "loading");
        assert!(machine.dispatch("data-arrived").is_some());
        assert_eq!(machine.state(), "present");
        assert!(machine.dispatch("data-arrived").is_none());
    }
}
