//! Acceptance: queries-answer — the blast-radius and completeness queries
//! under queries/ return correct results on the loaded reference data.

mod common;

use std::collections::BTreeSet;

use loom_graph::mode_contracts::{contract_views, judge_contract_consistency, machine_surfaces};
use loom_graph::reification_table::{
    judge_leaf_completeness, rows_from_results, NodeRef, UnreifiableReason,
};
use loom_graph::sparql::SparqlBuilder;
use loom_graph::tokens::{judge_token_resolution, token_rows};
use loom_graph::{LoomStore, NamedGraph};
use serde::Deserialize;

const BLAST_RADIUS_QUERY: &str = include_str!("../queries/blast_radius.sparql");

#[derive(Debug, Deserialize)]
struct ComponentRow {
    component: String,
}

fn blast_radius(store: &LoomStore, token: &str) -> BTreeSet<String> {
    let query = SparqlBuilder::from_query_file(BLAST_RADIUS_QUERY)
        .bind_graph("graph", &NamedGraph::Tokens)
        .bind_iri("token", &format!("{}{token}", loom_ontology::TOKEN_NS))
        .build();
    let rows: Vec<ComponentRow> = store.fetch_rows_bound(&query).unwrap();
    rows.into_iter().map(|r| r.component).collect()
}

#[test]
fn queries_answer_blast_radius() {
    let store = LoomStore::new().unwrap();
    common::load_reference_bundles(&store);

    // Blast radius of surface.interactive includes every component whose
    // tokens bind it — directly or through the resolution chain.
    let radius = blast_radius(&store, "surface.interactive");
    let expected: BTreeSet<String> = ["Ref.Button", "Ref.Listbox", "Ref.MenuTUI", "Ref.PickerSheet"]
        .into_iter()
        .map(String::from)
        .collect();
    assert_eq!(radius, expected, "blast radius of surface.interactive");
    assert!(
        !radius.contains("Ref.Form"),
        "Ref.Form binds text.body only — it must not appear"
    );

    // Cross-check against the closure computed in Rust over the token rows:
    // the query and the judge must agree on every component.
    let rows = token_rows(&store, &NamedGraph::Tokens).unwrap();
    assert!(
        judge_token_resolution(&rows).is_empty(),
        "reference token graph must be coherent"
    );
    let target = format!("{}surface.interactive", loom_ontology::TOKEN_NS);
    let mut closure: BTreeSet<String> = BTreeSet::new();
    for row in rows.iter().filter(|r| r.tier == "component") {
        let mut cursor = row.binds.clone();
        while let Some(current) = cursor {
            if current == target {
                closure.extend(row.component.clone());
                break;
            }
            cursor = rows
                .iter()
                .find(|r| r.token == current)
                .and_then(|r| r.binds.clone());
        }
    }
    assert_eq!(radius, closure, "query and Rust closure disagree");

    // A narrower radius behaves too.
    let body_radius = blast_radius(&store, "text.body");
    assert_eq!(
        body_radius,
        BTreeSet::from(["Ref.Form".to_string()]),
        "blast radius of text.body"
    );
}

#[test]
fn queries_answer_completeness() {
    let store = LoomStore::new().unwrap();
    common::load_reference_bundles(&store);

    let rows = rows_from_results(&store, NamedGraph::Reification).unwrap();
    assert_eq!(rows.len(), 6, "six reference reification rows");
    assert!(
        rows.iter().all(|r| r.platform.label.is_some()),
        "context objects carry labels from the bundle"
    );

    // The sample intent's aios-in-context (web/desktop/gui), plus one kind
    // the catalog does not know.
    let ctx = |name: &str| NodeRef {
        node: format!("{}{name}", loom_ontology::CONTEXT_NS),
        label: None,
    };
    let intent_aios = vec![
        ("single-select".to_string(), ctx("web"), ctx("desktop"), ctx("gui")),
        ("command".to_string(), ctx("web"), ctx("desktop"), ctx("gui")),
        ("date-picker".to_string(), ctx("web"), ctx("desktop"), ctx("gui")),
    ];
    let known_kinds: Vec<String> = vec!["single-select".into(), "command".into()];

    let report = judge_leaf_completeness(&intent_aios, &rows, &known_kinds, &[]);

    assert_eq!(report.reified.len(), 2, "both sample aios reify");
    assert_eq!(report.reified[0].cio, "Ref.Listbox");
    assert_eq!(report.reified[1].cio, "Ref.Button");
    assert_eq!(report.unreifiable.len(), 1);
    assert_eq!(report.unreifiable[0].aio_kind, "date-picker");
    assert_eq!(
        report.unreifiable[0].reason,
        UnreifiableReason::UnknownAioKind
    );
}

#[test]
fn queries_answer_contract_consistency() {
    let store = LoomStore::new().unwrap();
    common::load_reference_bundles(&store);

    let contracts = contract_views(&store, &NamedGraph::ModeContracts).unwrap();
    let machines = machine_surfaces(&store, &NamedGraph::Machines).unwrap();
    assert_eq!(contracts.len(), 4, "four reference mode-contracts");
    assert_eq!(machines.len(), 4, "four reference machines");

    for contract in &contracts {
        let machine = machines
            .iter()
            .find(|m| m.component == contract.component)
            .unwrap_or_else(|| panic!("no machine for contract {}", contract.component));
        let report = judge_contract_consistency(contract, machine);
        assert!(
            report.consistent(),
            "{}: phantom events {:?}, unreported states {:?}",
            contract.component,
            report.phantom_events,
            report.unreported_states
        );
    }
}
