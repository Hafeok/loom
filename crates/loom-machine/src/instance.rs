use serde::{Deserialize, Serialize};

/// The mode contract parsed from the design-system catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeContract {
    pub event_surface: Vec<String>,
    pub descriptor_states: Vec<String>,
    pub lifecycle: Vec<String>,
}

/// The serializable descriptor pushed to subscribers on state changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Descriptor {
    pub state: String,
    pub slots: Vec<String>,
    pub a11y_fields: Vec<String>,
    pub active_motion: Option<String>,
}

/// Assembles a Descriptor based on the current state and mode contract.
/// This function is pure and deterministic, mapping graph-interpreted state to a serializable descriptor.
pub fn assemble_descriptor(
    state_name: &str,
    _descriptor_states: &[String],
    overlay_states: &[String],
) -> Descriptor {
    Descriptor {
        state: state_name.to_string(),
        // In a full implementation, slots would be resolved from descriptor_states mapping
        slots: vec![], 
        // Platform-realised a11y fields are derived from overlay states
        a11y_fields: overlay_states.to_vec(),
        // Active motion is determined by the transition that fired, handled at call site
        active_motion: None,
    }
}
