# Loom — the implementation tier

*The concrete implementation of the [Loom Specification](https://github.com/Hafeok/loom-spec).
Version 0.1.0 — founding draft (Preview).*

> **Status: founding draft.** This document establishes the *architecture* of the
> Loom implementation — the layers, the seams between them, and the mode-contract
> that makes each seam checkable. The reference component (`single-select`) is
> worked through the mode-contract; concrete modes and adapters are built against
> this architecture, not inside it.

Loom-spec says *what a conforming design system must satisfy*. **Loom** is the
conforming design system — but "a design system" spans many platforms (iOS,
Android, Web) and, on each platform, many frameworks (SwiftUI/UIKit/MAUI on iOS;
Compose/Views on Android; React/Vue/Angular on Web). Loom must deliver the **same
behaviour** everywhere while being *native* on each platform and *idiomatic* in each
framework. This document is the layering that makes that possible without drift.

---

## 1. The three layers

```
loom (core)                         — the behavioural truth
  machines · reification tables · token graph · motion semantics · oracles

  ├── loom-ios       (mode)         — iOS platform projection
  │     ├── loom-ios-swiftui (adapter)
  │     ├── loom-ios-uikit   (adapter)
  │     └── loom-ios-maui    (adapter)
  ├── loom-android   (mode)
  │     ├── loom-android-compose (adapter)
  │     └── loom-android-views   (adapter)
  └── loom-web       (mode)
        ├── loom-web-react  (adapter)
        ├── loom-web-vue    (adapter)
        └── loom-web-angular(adapter)
```

**Core** owns behaviour that no platform changes: the component **state machines**,
the reification table's left side (AIO → component concept), the token graph, the
motion catalogue's *semantics*, and the conformance oracles. Defined once.

**A mode** is a **platform projection**: the machine *executed against a platform's
native affordances* — which native input fires each event, how focus/screen-reader
order is realised, how gestures dismiss, how reduced-motion is honoured, and the
platform-specific **accessibility discharge**. A mode is a **headless runtime**:
executable code (Swift for `loom-ios`, Kotlin for `loom-android`, TS/DOM for
`loom-web`) that runs the machine and exposes it through the mode-contract (§3).
The mode is where behaviour is verified against core.

**An adapter** is **thin framework packaging**: it mounts a mode component in a
framework's idiom (a SwiftUI `View`, a React component), wires the framework's
native events to the mode's event surface, and renders the mode's descriptor. It
carries **no behaviour** — it inherits behaviour from its mode.

### Naming rule

`loom-<platform>` is a **mode** (two segments after `loom`).
`loom-<platform>-<framework>` is an **adapter** (three segments), and the platform
segment MUST name an existing mode. `loom-swiftui` is ill-formed — it omits the
platform, and SwiftUI-on-iOS is a different projection than SwiftUI-on-macOS. The
platform is the mode because the platform determines behaviour; the framework is a
leaf because it determines only packaging.

---

## 2. Why the split holds (the anti-drift argument)

The same `single-select` machine — `loading → present → empty → failed` plus a
`closed/open` overlay gated on `present` — is *identical* on every platform and in
every framework. What varies:

| Varies by | Example | Owned by |
|---|---|---|
| nothing (behaviour) | the machine, its states, its transitions | **core** |
| platform | tap vs click vs Enter fires `trigger-activated`; VoiceOver vs TalkBack vs ARIA realises the a11y | **mode** |
| framework | a SwiftUI `View` vs a UIKit `UIViewController` vs a React component | **adapter** |

The test that the split is working: a focus-trap bug fixed once in `loom-ios`
reaches `loom-ios-swiftui` **and** `loom-ios-maui` with no per-adapter change. If
the same behavioural bug is ever fixed in two adapters of one platform, behaviour
has leaked upward and MUST be pushed back down into the mode. Same discipline as the
specification tier: shared truth defined once, variation quarantined to the genuine
leaf, drift caught mechanically.

---

## 3. The mode-contract — the seam that makes adapters provably thin

The mode↔adapter seam is the implementation-tier analogue of loom-spec's Reification
Contract. It is defined **per component** and declares the **only three surfaces an
adapter may touch**. If an adapter touches nothing else, it *cannot* carry
behaviour — faithfulness becomes a structural property, not a hope.

A mode-contract MUST validate against
[`schema/mode-contract.schema.json`](schema/mode-contract.schema.json) and MUST be
**consistent with the component's core machine** (§3.4). The three surfaces:

### 3.1 The event surface (adapter → mode, dispatch-only)

The mode declares the set of platform-neutral events it accepts — the machine's
**input alphabet**. The adapter's only write path is `dispatch(event)` where the
event is one of these names. The adapter *maps* native inputs onto them
(`.onTapGesture → dispatch("trigger-activated")`, `onClick → dispatch(...)`), and
**cannot dispatch anything else** — the mode rejects undeclared events. No
transition API is exposed: the adapter can *offer input*, never *effect a
transition*. The mode decides what an event means in the current state.

Events carry an `origin`: `user` events are the ones an adapter wires to native
input; `data`/`system` events are the mode's own (e.g. `data-arrived` fires from the
mode's data binding, not from the adapter).

### 3.2 The descriptor (mode → adapter, observable read-only)

On every state change the mode emits a **descriptor**: the current state, which
composition **slots** are present, the **accessibility fields** (name/role/value/
state, realised for this platform), and the **active motion** (a catalogue name +
phase, never an animation). The adapter **renders the descriptor and nothing more**
— it chooses *how to paint*, never *what to paint*. Crucially, **every accessibility
attribute the adapter applies MUST come from the descriptor**; an a11y attribute the
adapter authors itself is a fidelity violation. This is how the mode's
platform-specific a11y discharge (§3.3 of the spec's criteria) reaches the screen
without the adapter inventing it.

### 3.3 The lifecycle handshake (mount/unmount)

The adapter signals `mount` and `unmount` so the mode can start and dispose the
machine and its subscriptions. The mode tells the adapter nothing about *how* to
mount. This is the thinnest surface; it exists so focus post-conditions and
subscriptions have a defined lifecycle.

### 3.4 Consistency with the core machine (checkable)

A mode-contract is not invented — it is **derived** from the component's core
machine, and that derivation is checked (`reference/oracles/contract_consistency.py`):

- the **event surface equals the machine's input alphabet** — every transition
  label appears as an event, and no event lacks a transition (no phantom events,
  no missing events);
- the **descriptor's state range equals the machine's state space** — every state a
  slot or event references is a reachable machine state, and every reachable state
  is referenced (no stray states, no unreported states).

So the contract cannot promise the adapter a surface the behaviour doesn't back.

---

## 4. Conformance across the tier

Each seam is verified by an oracle, most already shipped by loom-spec:

| Seam | Obligation | Oracle |
|---|---|---|
| core → mode-contract | contract derived from the machine | `contract_consistency.py` (this repo) |
| core → mode | the mode's running machine ≡ core's machine | loom-spec `machine_equivalence.py` |
| mode → platform | criteria discharged with a platform basis | loom-spec criteria check; mode supplies the basis |
| **adapter → mode** | the adapter touches only the three surfaces | **adapter-fidelity oracle** (§5) |

The adapter never re-proves behaviour; it proves only that it *binds* to the mode
without adding logic. Behaviour is proven once, at the mode, against core.

---

## 5. The adapter-fidelity oracle

"The adapter is thin" is made both **structurally impossible to violate** (for the
egregious cases) and **checked** (for the residue):

**Structural (by API shape).** The mode exposes `dispatch(event)` and an observable
`descriptor` — and *no* method returning raw machine state, and *no* method
effecting a transition. A whole class of violations is simply uncallable: the
adapter has no transition function to invoke.

**Checked (by the oracle).** A per-framework structural scan of the adapter source
asserts that every reference to the mode instance is one of {subscribe/observe,
render(descriptor), dispatch(event), mount/unmount}, that every accessibility
attribute set traces to a descriptor `a11y-field`, and that the adapter holds no
local state shadowing a machine state. A violation — a transition, a state
conditional outside rendering, an un-sourced a11y attribute, a shadow state — fails
the check. This is the adapter-tier analogue of `machine_equivalence.py`: it does
not verify the adapter *does the right thing*, it verifies the adapter *cannot do
the wrong thing*.

*(The fidelity oracle is per-language and is specified here but implemented within
each adapter's toolchain; its checklist is normative, its implementation is not.)*

---

## 6. The reference app (conformance gallery)

Each **adapter** ships a runnable **gallery** — e.g. `loom-ios-swiftui-gallery` — a
thin app that mounts core's reference compositions through the adapter and lets you
click through every component in every state on-device. It is the
**visual/interactive counterpart to the oracles**: the oracles prove correctness on
paper; the gallery lets a human confirm it in the hand (does the picker *feel*
right, does focus actually move, does VoiceOver announce).

Three disciplines keep the gallery a conformance surface rather than a hand-built
demo:

1. **Reference data lives in core, not the app.** The pages are driven by the same
   reference bundles the oracles use — the `single-select` reference, the
   composition-graphs, the ingestion palette. The gallery renders *core's* data
   through the adapter; it invents none. Oracle and gallery thus check the same
   artifacts from two directions (structural and visual), and cannot drift.
2. **Pages are generated from core's composition-graphs and mode-contracts, not
   hand-authored.** The reference composition-graphs *are* the page list; each
   component's mode-contract declares its states and events, so the gallery derives
   a state-driver (dispatch `data-failed` to see the `failed` state) from the
   contract. Adding a component or screen to core extends the gallery automatically.
3. **The gallery drives components only through the mode-contract.** It dispatches
   declared events and renders descriptors — nothing else. So building the gallery
   is itself the **acceptance test for the mode-contract**: if the gallery must
   reach past the three surfaces to show a state, the contract is incomplete.

The gallery is a **build output of the implementation tier**, not of `product-cli`
(which prepares context and verifies, and does not emit apps). Product-cli MAY
*verify* a gallery conforms (renders every reference view, reaches no un-sanctioned
surface); it does not build it.

Seeing `loom-ios-swiftui-gallery` and `loom-web-react-gallery` both correctly
realise the *same* `single-select` machine — a native picker sheet on iOS, a
listbox on web — is the cross-platform behavioural equivalence made visible.

---

## 7. Reference — `single-select` through the mode-contract

[`reference/single-select/single-select.mode-contract.json`](reference/single-select/single-select.mode-contract.json)
works the reference component through the seam:

- **event surface**: `data-arrived/empty/failed` (data), `retry`,
  `trigger-activated`, `option-committed`, `dismissed`, `arrow-key` (user),
  `focus-entered/left` — exactly the machine's input alphabet.
- **descriptor**: `state` over `{loading, present:closed, present:open, empty,
  failed}`; slots `trigger/spinner/option-list/empty-message/error-message` with
  per-state presence; a11y fields `role/accessible-name/expanded/selected/
  live-region/focus-target` each tied to the criterion they discharge; motion in
  `active-motion`.
- **lifecycle**: `mount`, `unmount`.

Its consistency with the core machine is proven by
`reference/oracles/contract_consistency.py --self-test`.

---

## 8. The binding contract — wiring reified components to the domain

The gallery drives components with reference data; the product drives them with
the domain's real Views and Deciders. The **binding contract**
([`schema/binding-contract.schema.json`](../schema/binding-contract.schema.json))
is the layer that makes those the *same thing behind the same interface* — which
is what makes "what we verify is what we build" literal rather than aspirational.

**Derived, not authored.** The owning UI step already names its projection and its
commands, so per composition-graph leaf the binding is *computed from the What*
(`reference/oracles/binding_check.py`):

- **Sensor side** — a machine with data-origin events binds to the step's declared
  **View**; the `state-map` (projection state → machine data event:
  `present → data-arrived`, `empty → data-empty`, `failed → data-failed`) is
  derived from the projection's state space; `slot-data` names which projection
  fields feed which descriptor slots.
- **Effector side** — a machine that binds the Build seam
  (`binds-seam: build-seam:VerdictEvent`) binds to a step-declared **Command**;
  `trigger-activated` issues it, and the `verdict-map` wires the returning
  VerdictEvent (`admitted → verdict-admitted`, `rejected → verdict-rejected`).

Derivation **refuses ambiguity rather than guessing**: a step offering several
commands to one effector leaf yields `needs-authoring` (the binding tier's
annotation floor), and the authored choice is then *checked* against the step's
declarations exactly like a derived one.

**The checks (all MUST).**

1. **Declaration sourcing** — a binding may reference only a View the owning step
   projects and a Command the step declares (`binding-not-declared` otherwise).
   The wiring cannot say what the What did not — the same un-sourced discipline as
   the view contract's clause 5, applied to data flow.
2. **State-map coverage** — every state the projection can exhibit fires a machine
   event (`state-unwired` catches the silent-failure wiring bug: a projection that
   can fail whose screen never learns it failed).
3. **Verdict coverage** — both `admitted` and `rejected` are wired
   (`verdict-unwired` catches the unreachable failed state).

**Two drivers, one contract.** The binding contract's runtime surface is a driver
pair — a `ProjectionDriver` (pushes projection state + data per the `state-map`
and `slot-data`) and a `CommandDriver` (receives the issued command, returns the
verdict per the `verdict-map`). The **gallery** implements them with fixtures; the
**product** implements them with live Views and the Build seam. Nothing else
differs. The demo claim is therefore checkable: the gallery run and the product
run execute the same machines, the same descriptors, the same bindings — only the
drivers behind the interface change.

Reference: [`reference/bindings/`](../reference/bindings/) carries the settings
screen's canonical intent, its composition-graph, and the **derived**
binding-contract (schema-valid, self-test 10/10).

## Planned

- **`command` mode-contract** — the effector's seam (note its `succeeded/failed`
  descriptor states bind to the Build seam's VerdictEvent, an event of `origin:
  system`).
- **Adapter-fidelity oracle reference implementation** — one per adapter toolchain,
  against the normative checklist in §5.
- **First mode** — `loom-web` (DOM headless runtime) as the reference mode, with
  `loom-web-react` as the reference adapter and `loom-web-react-gallery` as the
  reference gallery.
- **Descriptor streaming shape** — pin the observable's concrete form (snapshot vs
  diff) once a real mode pulls on it.
- **Driver interface pinning** — the `ProjectionDriver`/`CommandDriver` surface is
  named here (§8); its concrete signatures pin in `architecture.md` phase 4 when
  `loom-reify` first hosts a fixture driver.
