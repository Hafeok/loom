# Build Context: d1-graph-core — feature `ft-graph-core`

⟦Ω:SPMC⟧ Frozen context. Produce one artifact; reference the What/How by pointer; do not re-decide them.

---

## What — the feature to realise

# Domain Context Bundle: f-bundle-load

⟦Ω:WhatBundle⟧{
  product≜loom:Product
  focus≜f-bundle-load:Flow
  depth≜2
  nodes≜13
}

---

## Focus

### flow `f-bundle-load` — Bundle Load

- steps: t-operator-load, cmd-load-bundle, ev-bundle-loaded, rm-catalog-inventory
- system: sys-loom-cli

---

## Bounded contexts

### context `ctx-catalog` — Design-System Catalog

- glossary: bundle — a loadable artifact set per artifact class, validated on load, token graph — primitive → semantic → component, resolution acyclic, machine — a component's projection layer + gated interaction overlay, reification row — (AIO-kind, platform, form-factor, interaction-class) → component, exactly-once, motion entry — named meaning + duration + easing + reduced-motion fallback, mode-contract — a component's three adapter surfaces, derived from its machine

---

## Entities

### entity `e-bundle` — Artifact Bundle

- attributes: {"name":"artifact-class","type":"string"}, {"name":"named-graph","type":"string"}, {"name":"format","type":"string"}, {"name":"shacl-status","type":"string"}, {"name":"source","type":"string"}, {"name":"version","type":"string"}
- context: ctx-catalog
- definition: A loadable artifact set for one artifact class (token bundle, machine catalog, reification table, motion catalog, mode-contracts), targeting one named graph. It enters the store only after SHACL validation on load — fail-closed; its load status and provenance are part of the catalog's record.
- identity: bundle id (artifact class + source + version)
- is_aggregate_root: true

### entity `e-machine` — Component Machine

- attributes: {"name":"component","type":"string"}, {"name":"projection-states","type":"string"}, {"name":"overlay-states","type":"string"}, {"name":"gates","type":"string"}, {"name":"side","type":"string"}
- context: ctx-catalog
- definition: A component's declared state machine: a projection layer over the UI step's projection-states plus an interaction overlay whose transitions are gated on declared projection states. Every interaction event has a keyboard binding in every keyboarded context (2.1.1 structural); transitions opening transient surfaces declare focus post-conditions; every transition carrying motion names a catalog entry or declares none. The machine is data — the runtime interprets it from the graph.
- identity: component name
- is_aggregate_root: true

### entity `e-mode-contract` — Mode-Contract

- attributes: {"name":"component","type":"string"}, {"name":"event-surface","type":"string"}, {"name":"descriptor-states","type":"string"}, {"name":"lifecycle","type":"string"}, {"name":"consistency-status","type":"string"}
- context: ctx-catalog
- definition: Per component, the only three surfaces an adapter may touch: the event surface (the machine's input alphabet, dispatch-only, each event carrying an origin user|data|system), the descriptor shape (state range, slots, a11y fields each tied to the criterion it discharges, active motion), and the mount/unmount lifecycle. Derived from — and checked consistent with — the component's core machine: no phantom events, no unreported states (loom-implementation §3.4).
- identity: component name
- is_aggregate_root: true

### entity `e-motion-entry` — Motion Catalog Entry

- attributes: {"name":"name","type":"string"}, {"name":"communicates","type":"string"}, {"name":"duration","type":"string"}, {"name":"easing","type":"string"}, {"name":"reduced-motion-fallback","type":"string"}
- context: ctx-catalog
- definition: One named entry of the motion catalog: what it communicates, duration, easing, and a mandatory reduced-motion fallback satisfying WCAG 2.3.3. Machine transitions reference entries by name only — never ad-hoc animation; an entry without a fallback fails validation (loom-spec §5).
- identity: catalog name
- is_aggregate_root: true

### entity `e-reification-row` — Reification Row

- attributes: {"name":"aio-kind","type":"string"}, {"name":"platform","type":"string"}, {"name":"form-factor","type":"string"}, {"name":"interaction-class","type":"string"}, {"name":"cio","type":"string"}
- context: ctx-catalog
- definition: One row of the reification table: (AIO-kind, platform, form-factor, interaction-class) → a component (CIO). Each AIO×context resolves to exactly one component — two rows matching the same pair is an ambiguity and fails validation; multiple rows for the same AIO kind across contexts is how one composition adapts responsively without forking (§3.1–3.2).
- identity: (aio-kind, platform, form-factor, interaction-class)
- is_aggregate_root: true

### entity `e-token-graph` — Token Graph

- attributes: {"name":"version","type":"string"}, {"name":"criteria-discharges","type":"string"}, {"name":"resolution-status","type":"string"}
- context: ctx-catalog
- definition: The three-tier token graph (primitive → semantic → component) with its resolution edges and criteria-discharge records. Every component token must resolve through the semantic tier to a primitive with no cycle; a cycle or dangling reference is an incoherence that fails validation (loom-spec §2).
- identity: distribution version
- is_aggregate_root: true

---

## Commands

### command `cmd-load-bundle` — Load Bundle

- context: ctx-catalog
- emits: ev-bundle-loaded, ev-bundle-rejected
- fields: {"name":"bundle-path","type":"string"}, {"name":"artifact-class","type":"string"}
- targets: e-bundle

---

## Events

### event `ev-bundle-loaded` — Bundle Loaded

- changes: e-bundle
- context: ctx-catalog
- fields: {"name":"bundle-id","type":"string"}, {"name":"artifact-class","type":"string"}, {"name":"named-graph","type":"string"}, {"name":"triple-count","type":"integer"}

### event `ev-bundle-rejected` — Bundle Rejected

- changes: e-bundle
- context: ctx-catalog
- fields: {"name":"bundle-id","type":"string"}, {"name":"artifact-class","type":"string"}, {"name":"shacl-violations","type":"string"}

---

## Read models

### read-model `rm-catalog-inventory` — Catalog Inventory

- projects: e-bundle, e-token-graph, e-machine, e-reification-row, e-motion-entry, e-mode-contract
- states: loading, present, empty, failed

---

## Triggers

### trigger `t-operator-load` — Operator loads a bundle

- issues: cmd-load-bundle
- source: user

---


---

## How — apply these by pointer

**Principles** (obey):
- p-views-over-graph: Rust structs are views over the graph — SPARQL CONSTRUCT into typed rows. No parallel domain model exists that could drift from the store.
- p-queries-fetch-oracles-judge: Queries fetch, oracles judge: no verdict is computed inside a SPARQL query where the logic would be unreadable — the decision procedure is a pure Rust function over queried facts.
- p-run-graph-append: Every check verdict is written to the run graph as it is produced; a report is a CONSTRUCT over the run graph and is never assembled from memory.
- p-fail-closed-load: A bundle that fails SHACL validation never enters the store, and a palette that fails a token-tier criterion never enters the semantic tier — every gate is fail-closed.
- p-vectors-equivalence: Rust and Python oracles run the same encoding-neutral vectors; any disagreement is INCOHERENCE (contract under-specification) and blocks release — never a bug silently fixed on one side.
- p-ffi-surface-minimal: loom-ffi and loom-wasm expose exactly the mode-contract — dispatch(event), a descriptor subscription, mount/unmount — and nothing else: no raw-state getter, no transition method. What the adapter cannot call, no platform can leak.
- p-shim-no-state: Shims and adapters hold no shadow state and contain no conditional on machine state other than rendering the descriptor; native events map to declared events by table. The same fidelity rules apply one level down as one level up.
- p-one-version: A mode's version equals the core version it embeds — one number, one behaviour; a mode may not version independently of the runtime.
- p-machine-from-graph: Adding a component to the catalog is a bundle load, not a code change: the runtime interprets machines from loom:g/machines, and any load-time compilation keeps the graph as the source of truth.
- p-seam-schema-validated: Every message crossing a seam is schema-validated at the boundary against the pinned contract version — UIIntent on entry, ReificationReport on exit, ingestion mapping on submission.

**Patterns** (apply):
- pat-typed-row-view: A maintained SPARQL CONSTRUCT under queries/ + a serde row struct + a typed view built From&lt;Row&gt; — the only way domain data leaves the store. The query file is vector-tested; no inline SPARQL literals in crate code.
- pat-load-gate: Loader function: parse JSON-LD against the pinned context → SHACL-validate against shapes/ → atomic insert into the target named graph. On violation, return the shape report and write nothing.
- pat-oracle-pair: Each §7 oracle = a SPARQL fact-gatherer + a pure Rust judge function + a paired vector test that runs both the Rust judge and loom-spec's Python reference over the shared vectors/ and diffs verdicts.
- pat-ffi-handle: LoomComponent handle, declared once in UniFFI udl with a wasm-bindgen twin: dispatch(event), subscribe(descriptor-callback), mount(), unmount() — the complete public surface; no other symbol exported.
- pat-shim-table: Per-platform shim = a static event-mapping table (native input → declared event name) + descriptor-apply functions (a11y fields verbatim, motion per catalog entry with the platform's reduce-motion honoured). Zero branches on machine state; zero local state.
- pat-run-recorder: PROV-O writer: the run is an Activity, the intent an Entity; each oracle verdict is appended to loom:g/runs/&lt;id&gt; as produced. Report emission = CONSTRUCT over the run graph → map to canonical enums → schema-validate → serialise.
- pat-graph-interpreter: Machine interpreter: at mount, read the component's machine from loom:g/machines into an interpretable transition structure (optionally table-compiled); every dispatch resolves against that structure; the graph remains the sole authoring surface.

**Application contract**: loom-app (Rust (2021 edition workspace); TS/Swift/Kotlin only in shims and adapters)

---

## Behaviour — the Decider oracle (your code must compute the same)

### Decider `e-bundle-decider` (decides for e-bundle)
- a conformant token bundle loads into its named graph: given [], when cmd-load-bundle, then emit ["ev-bundle-loaded"]
- an unknown artifact class is rejected fail-closed: given [], when cmd-load-bundle, then reject inv-known-artifact-class
- a bundle without a source path is rejected: given [], when cmd-load-bundle, then reject inv-bundle-source-present
- a previously rejected bundle may be re-loaded once fixed: given ["ev-bundle-rejected"], when cmd-load-bundle, then emit ["ev-bundle-loaded"]
- re-loading the already-resident bundle is a rejected no-op: given ["ev-bundle-loaded"], when cmd-load-bundle, then reject inv-bundle-already-resident

---

## Acceptance — what makes this done

- [pending] bundles-load: every loom-spec reference bundle (single-select, command, token-graph, composition containers, motion) loads through the SHACL gate into its named graph with zero violations
- [pending] shacl-vectors-green: shapes/ assertions pass on all encoding-neutral vectors — negative fixtures rejected, positive fixtures admitted, zero MISMATCH
- [pending] queries-answer: the blast-radius and completeness queries under queries/ return correct results on the loaded reference data (blast radius of surface.interactive includes every component whose tokens bind it)
