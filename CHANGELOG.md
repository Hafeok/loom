# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); Semantic
Versioning adapted for a standard (MAJOR on a change that can invalidate a
previously-conforming mode or adapter).

## [0.3.0] — 2026-07-06

### Added
- **The binding contract** (`schema/binding-contract.schema.json`,
  `docs/loom-implementation.md` §8) — the wiring of reified components to the
  domain, **derived from the What** (the owning UI step's declared projection and
  commands), never invented; ambiguity yields `needs-authoring` instead of a
  guess. Sensor side binds a View with a projection-state → data-event
  `state-map`; effector side binds a Command with a VerdictEvent `verdict-map`.
- **`reference/oracles/binding_check.py`** — derivation + three checks
  (declaration sourcing, state-map coverage, verdict coverage), self-test 10/10;
  catches the two production wiring bugs mechanically: the silent failure (a
  projection that can fail whose screen never learns it) and the unreachable
  rejection.
- **`reference/bindings/`** — the settings screen's canonical intent, its
  composition-graph, and the derived, schema-valid binding contract.
- **The driver pair** — `ProjectionDriver`/`CommandDriver`: the gallery's
  FixtureDriver and the product's LiveDriver are two conforming implementations
  of the *same* contract, making "what we verify is what we build" checkable.
- **`loom-bind` crate** added to the architecture; phase-4/5 gates extended so the
  gallery demonstrably runs through the same bindings the product will.

## [0.2.0] — 2026-07-06

### Added
- **Implementation architecture** ([`docs/architecture.md`](docs/architecture.md)) —
  the concrete How: embedded **Oxigraph as the primary store** (named graphs per
  artifact class, JSON-LD loaders, PROV-O run graphs), **SHACL shapes closing the
  dual-encoding gap** (encoding-neutral vectors shared with loom-spec's Python
  oracles; Rust≡Python disagreement treated as INCOHERENCE), the **Rust workspace**
  (graph/ontology/machine/descriptor/reify/oracles/ingest/ffi/wasm/cli), and the
  decisive runtime choice: **one compiled machine runtime on every platform**
  (wasm for web, UniFFI static libs for iOS/Android), so cross-platform
  behavioural identity is by construction — a mode is the runtime plus a thin
  shim owning only native event capture, a11y realisation, and motion
  realisation. The FFI surface exposes exactly the mode-contract
  (dispatch/subscribe/mount/unmount), making the forbidden calls structurally
  uncallable. Ends with the six-phase gated build order for product-cli.

## [0.1.0] — 2026-07-06

The founding draft of the Loom implementation tier.

### Added
- **Three-layer architecture** (`docs/loom-implementation.md`) — core (behavioural
  truth) → modes (platform projections, headless runtimes) → adapters (thin
  framework packaging), with the naming rule `loom-<platform>` (mode) /
  `loom-<platform>-<framework>` (adapter).
- **The mode-contract** — the per-component mode↔adapter seam declaring the only
  three surfaces an adapter may touch (dispatch-only event surface, observable
  descriptor, lifecycle handshake), making thin adapters a structural property.
  Normative schema in `schema/mode-contract.schema.json`.
- **Core → mode-contract consistency check**
  (`reference/oracles/contract_consistency.py`) — proves a mode-contract is derived
  from its core machine (event surface = input alphabet; descriptor states = machine
  states), catching missing events, phantom events, and impossible states.
- **Reference**: `single-select` worked through the mode-contract.
- **The conformance gallery** — the runnable per-adapter app that renders core's
  reference data through an adapter, as the visual counterpart to the oracles and
  the acceptance test for the mode-contract.

## Planned

- **`command` mode-contract** — the effector seam; `succeeded/failed` descriptor
  states bind the Build seam's VerdictEvent (an `origin: system` event).
- **Adapter-fidelity oracle** reference implementation per adapter toolchain.
- **First mode + adapter + gallery** — `loom-web` / `loom-web-react` /
  `loom-web-react-gallery` as the reference vertical slice.
- **Descriptor streaming shape** — snapshot vs diff, pinned once a real mode pulls
  on it.
