//! loom-ontology — vocabulary constants and the pinned JSON-LD context.
//!
//! Every IRI the workspace mints lives here; every bundle is parsed against
//! [`PINNED_CONTEXT`] (pat-load-gate), never against a context it carries itself.

/// Core vocabulary namespace.
pub const LOOM_NS: &str = "http://loom.dev/ns#";
/// Named-graph namespace: `loom:g/<artifact-class>` display names resolve here.
pub const GRAPH_NS: &str = "http://loom.dev/g/";
/// Token individuals: `token:<path>`.
pub const TOKEN_NS: &str = "http://loom.dev/token/";
/// Motion catalog entries: `motion:<name>`.
pub const MOTION_NS: &str = "http://loom.dev/motion/";
/// Component machines: `machine:<component>`.
pub const MACHINE_NS: &str = "http://loom.dev/machine/";
/// Reification rows: `row:<key>`.
pub const ROW_NS: &str = "http://loom.dev/row/";
/// Mode-contracts: `contract:<component>`.
pub const CONTRACT_NS: &str = "http://loom.dev/contract/";
/// Context objects (platforms, form factors, interaction classes): `ctx:<name>`.
pub const CONTEXT_NS: &str = "http://loom.dev/context/";
/// Run activities and entities minted by the PROV-O recorder.
pub const RUN_NS: &str = "http://loom.dev/run/";

/// W3C namespaces.
pub const PROV_NS: &str = "http://www.w3.org/ns/prov#";
pub const SHACL_NS: &str = "http://www.w3.org/ns/shacl#";
pub const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// The artifact classes a bundle may declare (e-bundle-decider: inv-known-artifact-class).
pub const ARTIFACT_CLASSES: [&str; 5] = [
    "tokens",
    "machines",
    "reification",
    "motion",
    "mode-contracts",
];

/// The pinned JSON-LD context document. Bundles are expanded against this and
/// nothing else; a context shipped inside a bundle is ignored on load.
pub const PINNED_CONTEXT: &str = include_str!("../contexts/loom.context.json");

/// The pinned context as a JSON value (the object under `@context`).
pub fn pinned_context_value() -> serde_json::Value {
    let doc: serde_json::Value =
        serde_json::from_str(PINNED_CONTEXT).expect("pinned context is valid JSON");
    doc.get("@context")
        .expect("pinned context has an @context key")
        .clone()
}

#[cfg(test)]
mod tests {
    #[test]
    fn pinned_context_parses() {
        let ctx = super::pinned_context_value();
        assert_eq!(ctx["loom"], super::LOOM_NS);
        assert_eq!(ctx["@vocab"], super::LOOM_NS);
    }
}
