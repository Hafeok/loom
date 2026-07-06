# Loom

**The implementation of the [Loom Specification](https://github.com/Hafeok/loom-spec)
— a design system delivered natively on every platform and idiomatically in every
framework, from one shared behavioural core.**

Loom-spec says *what a conforming design system must satisfy*. This repository is
**Loom's core** plus the **architecture** of its platform and framework layers.
Core owns the behavioural truth — the component state machines, reification tables,
token graph, motion semantics, and conformance oracles — defined **once**. Platform
**modes** (`loom-ios`, `loom-android`, `loom-web`) project that truth natively;
framework **adapters** (`loom-ios-swiftui`, `loom-web-react`, …) package it
idiomatically and carry no behaviour of their own.

The architecture — layers, the mode-contract seam, the adapter-fidelity rule, and
the reference gallery — is in
[`docs/loom-implementation.md`](docs/loom-implementation.md).

> **Status: founding draft (Preview).** The mode-contract and its consistency check
> are specified and working for the reference component (`single-select`). Concrete
> modes, adapters, and galleries are built against this architecture.

## The three layers

```
loom (core)   — machines · tables · tokens · motion semantics · oracles   ◀── this repo
  ├── loom-ios / loom-android / loom-web        (modes: platform projections)
  │     └── loom-<platform>-<framework>          (adapters: thin packaging)
  │           └── loom-<platform>-<framework>-gallery  (runnable conformance app)
```

- **Naming rule.** Two segments after `loom` = a **mode** (`loom-ios`). Three = an
  **adapter** (`loom-ios-swiftui`), whose platform segment must name a real mode.
  `loom-swiftui` is ill-formed — the platform (which owns behaviour) is missing.

## The mode-contract

The seam between a mode and its adapters, defined per component. It declares the
only three surfaces an adapter may touch — a **dispatch-only event surface**, an
**observable descriptor**, and a **lifecycle handshake** — so an adapter *cannot*
carry behaviour. It is derived from (and checked against) the component's core
machine.

| Surface | Direction | The adapter may only… |
|---|---|---|
| event surface | adapter → mode | dispatch declared events (map native inputs onto them) |
| descriptor | mode → adapter | render the current descriptor (incl. all a11y, verbatim) |
| lifecycle | adapter → mode | signal mount / unmount |

## What's here

| Path | Contents |
|---|---|
| [`docs/loom-implementation.md`](docs/loom-implementation.md) | The implementation tier's What: layers, mode-contract, gallery. |
| [`docs/architecture.md`](docs/architecture.md) | The implementation tier's How: Rust · RDF · SHACL · Oxigraph — graph substrate, workspace, FFI modes, phased build order. |
| [`schema/mode-contract.schema.json`](schema/mode-contract.schema.json) | Normative shape of a per-component mode-contract. |
| [`reference/single-select/`](reference/single-select/) | `single-select` worked through the mode-contract. |
| [`reference/oracles/contract_consistency.py`](reference/oracles/contract_consistency.py) | Proves a mode-contract is derived from its core machine. |

## License

Spec text CC BY 4.0 ([`LICENSE-docs`](LICENSE-docs)); schemas and code Apache-2.0
([`LICENSE`](LICENSE)).
