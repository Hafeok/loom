# Loom — implementation architecture

*Rust · RDF · SHACL · Oxigraph. Version 0.2.0 — founding architecture (Preview).*

> **Status.** This document fixes the concrete architecture of the Loom
> implementation: the graph substrate, the Rust workspace, the machine runtime,
> how modes and adapters are produced, and the phased build order. It is the How
> for the implementation tier whose What is
> [`docs/loom-implementation.md`](loom-implementation.md) (layers, mode-contract,
> gallery) and whose conformance target is
> [`loom-spec@0.8.0`](https://github.com/Hafeok/loom-spec).

---

## 1. The two decisions everything else follows from

**1. The graph is the implementation, not a serialization of it.** loom-spec §8
requires one queryable graph over which blast-radius, completeness, machine
equivalence, view legality, and criteria discharge are answerable *by query*.
Loom does not satisfy this by exporting a graph from some other primary store —
the **embedded Oxigraph store is the primary store**. Machines, the reification
table, the token graph, the motion catalog, and the mode-contracts are RDF, loaded
once, queried everywhere. Rust structs are *views over the graph* (SPARQL
CONSTRUCT into typed rows), never a parallel model that could drift. This is the
same graph-as-state discipline as `decision-cli`: one substrate, one write path,
projections derived.

**2. The machine runtime is one compiled artifact on every platform.** The
implementation tier's hardest promise is that `loom-ios`, `loom-android`, and
`loom-web` run *the same machine*. The architecture makes this literal instead of
verified-after-the-fact: the state-machine runtime is a Rust crate compiled to
**wasm** (web) and to **static libraries via UniFFI** (iOS/Android). A mode is
therefore *not* a per-platform reimplementation of behaviour — it is the same
`loom-machine` binary plus a **thin platform shim** (native event capture in, a11y
realisation out). `machine_equivalence` still runs (defence in depth, and it
covers the shim's event mapping), but cross-platform behavioural drift is
prevented by construction, not caught by oracle.

```
                        ┌────────────────────────────────────┐
                        │            loom (core, Rust)        │
                        │  Oxigraph store  ·  SHACL  ·  PROV-O│
                        │  machines · tables · tokens · motion│
                        │  mode-contracts · oracles · runtime │
                        └──────┬──────────────┬───────────────┘
              wasm-bindgen     │              │      UniFFI (staticlib)
                        ┌──────▼─────┐  ┌─────▼──────┐  ┌────────────┐
                        │  loom-web  │  │  loom-ios  │  │loom-android│   modes =
                        │ DOM shim,  │  │VoiceOver / │  │ TalkBack / │   runtime
                        │ ARIA apply │  │gesture shim│  │gesture shim│   + shim
                        └──────┬─────┘  └─────┬──────┘  └─────┬──────┘
                          react/vue      swiftui/uikit   compose/views    adapters
                          (TS, thin)     (Swift, thin)   (Kotlin, thin)
```

---

## 2. The graph substrate

### 2.1 Store and layout

Embedded **Oxigraph** (same crate lineage as `decision-cli`), one store per Loom
distribution, **named graphs per artifact class** so provenance and reload are
per-artifact:

| Named graph | Contents | Loaded from |
|---|---|---|
| `loom:g/tokens` | the three-tier token graph + criteria-discharge records | token bundles / §2.5 ingestion |
| `loom:g/machines` | every component machine (states, transitions, gates, motion refs, keyboard bindings per context) | loom-spec reference bundles + Loom's own catalog |
| `loom:g/reification` | the reification table (AIO-kind × context → component) | catalog bundles |
| `loom:g/motion` | the motion catalog (meaning, duration, easing, reduced-motion fallback) | catalog bundles |
| `loom:g/mode-contracts` | per-component event surface, descriptor shape, lifecycle | `loom` repo mode-contract bundles |
| `loom:g/runs/<id>` | one graph per reification run: the UIIntent ingested, every check verdict, the emitted report — **PROV-O** activities/entities throughout | runtime |

Wire format N-Quads; JSON-LD contexts for every bundle so the existing JSON
artifacts (loom-spec's reference bundles, the canonical seam messages) load
losslessly into RDF and round-trip back out.

### 2.2 Ontology

A small `loom:` vocabulary, deliberately *derived from* loom-spec's terms rather
than invented: `loom:Machine`, `loom:ProjectionState`, `loom:OverlayState`,
`loom:Transition` (with `loom:onEvent`, `loom:to`, `loom:motion`), `loom:Gate`
(`loom:blocks`), `loom:ReificationRow` (`loom:aioKind`, `loom:context`,
`loom:cio`), `loom:Token` (three subclasses by tier — the *one* place a type
distinction is warranted because the tiers have different reference rules),
`loom:binds`, `loom:Motion` (`loom:communicates`, `loom:reducedMotionFallback`),
`loom:EventSurface`, `loom:Descriptor`, `loom:A11yField` (`loom:discharges`).
Context objects (`platform` / `form-factor` / `interaction-class`) are nodes, not
strings, so "every context this catalog covers" is one query.

### 2.3 SHACL — closing the dual-encoding gap

loom-spec ships JSON Schema only (its recorded Planned gap). Loom closes it on the
implementation side, per the house pattern:

- **SHACL shapes** for every artifact class (token graph resolution + no-cycle,
  machine well-formedness + overlay gating, reification-table exactly-once,
  mode-contract surface shapes), validated **on load** — a bundle that fails SHACL
  never enters the store.
- **Encoding-neutral test vectors**: the same positive/negative fixtures that
  drive loom-spec's Python oracles are asserted against the SHACL shapes, holding
  the two encodings equivalent. `MISMATCH` (an instance fails one encoding) is a
  bundle bug; `INCOHERENCE` (the encodings disagree on a vector) is a contract
  bug and blocks release.
- Constraints inexpressible in either (e.g. §7.2 machine equivalence, the view
  join) live in the Rust oracle layer (§4), exactly as the conformance-harness
  pattern prescribes.

### 2.4 The §8 queries, concretely

Each mandated query is a maintained SPARQL asset (in `queries/`, tested against
vectors), not ad-hoc strings:

- **Blast radius**: property-path traversal from a token through `loom:binds*` to
  every `loom:ReificationRow`/component, plus every criteria-discharge record
  whose pairing touches it.
- **Completeness**: the reified/unreifiable partition as a CONSTRUCT over
  `loom:g/reification` joined to the ingested intent's `aios-in-context`.
- **Machine equivalence, view legality, criteria discharge**: SPARQL gathers the
  facts; the decision procedure is Rust (§4). The split rule: *queries fetch,
  oracles judge* — no verdict is computed inside a query where the logic would be
  unreadable.

---

## 3. The Rust workspace

```
loom/
  Cargo.toml                 # workspace
  crates/
    loom-graph/              # Oxigraph store, named-graph mgmt, JSON-LD loaders,
                             # SHACL validation on load, PROV-O run recording
    loom-ontology/           # the loom: vocabulary as consts + JSON-LD contexts
    loom-machine/            # the runtime: interpret a machine FROM THE GRAPH;
                             # dispatch(event) -> transition -> descriptor emit;
                             # gate enforcement; focus post-conditions; the three
                             # mode-contract surfaces (store/events/lifecycle)
    loom-descriptor/         # descriptor assembly (state, slots, a11y fields,
                             # active motion) from graph + machine state
    loom-reify/              # the seam: canonical UIIntent in (serde against the
                             # canonical schemas), checks orchestrated, canonical
                             # ReificationReport out (CONSTRUCT -> JSON)
    loom-oracles/            # §7 checks in Rust: leaf completeness, composition
                             # join, view join, machine equivalence, criteria,
                             # motion; judged against loom-spec's test vectors
    loom-ingest/             # §2.5 token-ingestion gate (WCAG contrast in Rust)
    loom-bind/               # the binding contract: derive bindings from the What,
                             # check them (sourcing / state-map / verdict coverage),
                             # host the ProjectionDriver/CommandDriver interface;
                             # FixtureDriver (gallery) and LiveDriver (product)
                             # are the two conforming implementations
    loom-ffi/                # UniFFI interface (udl): LoomComponent handle with
                             # dispatch / subscribe / mount / unmount ONLY
    loom-wasm/               # wasm-bindgen twin of loom-ffi for the web
    loom-cli/                # `loom` binary: load, validate, ingest, reify,
                             # report, query — the product-cli-facing surface
  queries/                   # maintained SPARQL (§2.4), tested
  shapes/                    # SHACL (§2.3)
  vectors/                   # encoding-neutral fixtures, imported from loom-spec
  contracts/                 # vendored canonical seam schemas (pinned version)
```

Key boundaries:

- **`loom-machine` reads machines from the graph** — a machine is data
  (`loom:g/machines`), the runtime interprets it. Adding a component to the
  catalog is a bundle load, not a code change. (Hot paths may compile a machine
  to a transition table at load; the graph stays the source.)
- **`loom-ffi`/`loom-wasm` expose exactly the mode-contract and nothing else**:
  `dispatch(event)`, a descriptor subscription, `mount`/`unmount`. No
  raw-state getter, no transition method — the "structurally uncallable"
  enforcement from the mode-contract, realised as the FFI surface itself. What
  the adapter *cannot* call, no platform can leak.
- **`loom-reify` speaks only the canonical contracts** — serde types generated
  against the pinned `ai-development-contracts` schemas; the composition-graph
  rides in `producer-extension` per loom-spec 0.8.0 §3.4.

### 3.1 The oracles move to Rust — and stay honest

loom-spec's Python oracles remain the *reference* judges; Loom reimplements each
in `loom-oracles` and proves equivalence by running both against the shared
`vectors/`. A vector on which Rust and Python disagree is treated as
INCOHERENCE (contract under-specification), not as a bug to silently fix in one
side. Once green, the Rust oracles are Loom's runtime verification layer and the
Python ones remain loom-spec's portable proof of decidability.

---

## 4. Modes: the runtime crosses the FFI, the shim stays thin

A **mode** = `loom-machine` (compiled) + a platform shim, and the shim's whole
job is the two things Rust cannot own:

| Shim responsibility | loom-web (TS) | loom-ios (Swift) | loom-android (Kotlin) |
|---|---|---|---|
| **native event capture** → `dispatch(name)` | DOM listeners (click/keydown/focusin) | gesture recognisers, key commands | Compose/View input, key events |
| **a11y realisation** ← descriptor `a11y-fields` | apply ARIA attributes verbatim | apply `accessibilityLabel`/traits/`UIAccessibility` posts | apply semantics/`contentDescription`/live regions |
| **motion realisation** ← descriptor `active-motion` | CSS transitions per catalog entry (+ `prefers-reduced-motion`) | `UIView`/SwiftUI animation per entry (+ Reduce Motion) | animation APIs (+ system reduce-motion) |

The shim contains **no conditional on machine state** other than rendering the
descriptor, holds no shadow state, and maps events by table — the same fidelity
rules as adapters, one level down, and checkable the same way. Everything else
(what an event means, which state follows, which slots are present, which a11y
values hold, which motion fires) is decided inside the compiled core, so it is
*identical bytes* deciding it on every platform.

**Adapters** are unchanged from the What (`loom-implementation.md` §3): thin
framework packaging over the mode's three surfaces. On web, `loom-web-react` wraps
the wasm component handle in a hook; on iOS, `loom-ios-swiftui` wraps the UniFFI
handle in an `ObservableObject`. The adapter-fidelity oracle checks each per its
toolchain.

**Distribution:** `loom-web` ships as an npm package (wasm + TS shim);
`loom-ios` as an XCFramework via SwiftPM; `loom-android` as an AAR via Maven.
Version = the core version; a mode may not version independently of the runtime
it embeds (one number, one behaviour).

---

## 5. The seam, end to end (one reification run)

1. `loom-cli reify intent.json` — the canonical UIIntent is schema-validated,
   loaded into `loom:g/runs/<id>` (PROV-O: the intent is an Entity, the run an
   Activity).
2. The composition-graph is read from `producer-extension`, SHACL-validated.
3. Oracles run in §7 order (leaf completeness → composition → view → machine
   equivalence → criteria → motion); every check's verdict is written to the run
   graph *as it is produced* — the report is never assembled from memory.
4. The canonical **ReificationReport** is a CONSTRUCT over the run graph,
   serialised to JSON, schema-validated against the canonical
   `reification-report.schema.json` before emission (internal `discharged`
   booleans mapped to the canonical enum at this boundary).
5. The run graph persists: "why was this AIO unreifiable last Tuesday" is a
   query, per the Representation Contract.

The **gallery** (per adapter) is generated from the same store: a SPARQL query
over `loom:g/mode-contracts` + the reference composition-graphs yields the page
list and each component's state-driver (which events, valid in which states);
the gallery app renders core's reference data through the adapter and nothing
hand-authored.

---

## 6. Phased build order (the product-cli input)

Each phase ends at a verifiable gate; no phase starts before the prior gate is
green. This is the funnel: constraint density rises toward the leaf, and by
phase 4 the remaining work is small-model-shaped.

| Phase | Deliverable | Gate |
|---|---|---|
| **1. Graph core** | `loom-graph` + `loom-ontology` + loaders + SHACL shapes; loom-spec reference bundles load | every bundle loads; SHACL vectors green; §2.4 queries answer on reference data |
| **2. Oracles** | `loom-oracles` + `loom-ingest` in Rust | Rust ≡ Python on all shared vectors (no INCOHERENCE); `loom-cli validate` works |
| **3. Runtime** | `loom-machine` + `loom-descriptor`; mode-contract surfaces in-process | machine-equivalence green against `loom:g/machines`; contract-consistency green; descriptor snapshots match mode-contract shapes |
| **4. Seam** | `loom-reify` + `loom-cli reify` + `loom-bind` (derive/check + driver interface + FixtureDriver) | canonical intent in → canonical report out, both schema-validated; run graph queryable; bindings derived from the intent and checked; components drivable through the FixtureDriver |
| **5. First mode + adapter + gallery** | `loom-wasm` → `loom-web` (DOM shim) → `loom-web-react` → `loom-web-react-gallery` | gallery renders every reference composition **through the FixtureDriver and the derived bindings** — the same contract the product's LiveDriver implements; drives every machine state through dispatched events only; adapter-fidelity scan green |
| **6. Native modes** | `loom-ffi` → `loom-ios` (XCFramework) and `loom-android` (AAR) + one adapter + gallery each | same gallery checklist per platform; the *same* `single-select` machine demonstrably running from the same core on all three |

Descriptor streaming (snapshot vs diff) is pinned in phase 3 when the runtime
first pulls on it — snapshot-first, diff only if a gallery shows it necessary.

## Planned

- **Phase-1 kickoff bundle** — the JSON-LD contexts for loom-spec's reference
  bundles, so day one is a load, not a modelling debate.
- **Adapter-fidelity scanners** — TS (web) first, as part of phase 5's gate.
- **Effect classes for `loom-cli`** — when Loom runs under Kiln, its work units
  declare `effect:filesystem-write(scope)` per the (pending) effect-class
  vocabulary; noted here so the CLI's surface anticipates it.
