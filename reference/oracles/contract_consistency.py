#!/usr/bin/env python3
"""
contract_consistency — proves a mode-contract is a faithful projection of its core
machine, not an independent invention.

Two things must hold (else the contract could drift from behavior):

  1. Event-surface = machine input alphabet.
     Every user/data event the machine consumes (every transition label) MUST
     appear in the contract's event-surface, and every event in the surface MUST
     be a real machine input. No phantom events, no missing events.

  2. Descriptor state range = machine state space.
     Every state the descriptor can report (projection states, and projection:overlay
     composite states) MUST be a reachable machine state, and every reachable
     machine state MUST be reportable. The adapter can render exactly the states
     the machine can be in — no more, no less.

This is the core -> mode-contract equivalence: it makes the seam DERIVED from the
machine, so the contract cannot promise the adapter a surface the behavior doesn't
back.

Run:  python3 contract_consistency.py <mode-contract.json> <reification-bundle.json> <kind>
  or: python3 contract_consistency.py --self-test
"""
import json, sys, pathlib


def load(p): return json.load(open(p))


def machine_inputs(machine):
    """Every transition label in the machine: projection transitions + overlay."""
    inputs = set()
    for src, evs in machine.get("transitions", {}).items():
        inputs.update(evs.keys())
    overlay = machine.get("overlay", {})
    for src, evs in overlay.get("transitions", {}).items():
        inputs.update(evs.keys())
    # focus overlay events are part of the interaction surface but may live only in
    # the contract if the machine models them implicitly; we treat focus-* as allowed
    return inputs


def machine_states(machine):
    """Reachable states: projection states, plus projection:overlay composites for
    the gated overlay."""
    proj = list(machine.get("projection-states", []))
    states = set(proj)
    overlay = machine.get("overlay", {})
    gate = overlay.get("gated-on")
    ostates = overlay.get("states", [])
    if gate and ostates:
        # composite states like present:closed, present:open
        for o in ostates:
            states.add(f"{gate}:{o}")
        # the bare gated state (e.g. 'present') is represented by its composites
        states.discard(gate)
    return states, gate, set(ostates)


def check(contract, machine):
    findings = []

    # ---- 1. event surface vs machine inputs
    surface_events = {e["name"] for e in contract["event-surface"]["events"]}
    m_inputs = machine_inputs(machine)
    FOCUS = {"focus-entered", "focus-left"}  # interaction-surface events allowed in contract

    missing_from_surface = m_inputs - surface_events
    for e in sorted(missing_from_surface):
        findings.append({"reason": "event-missing-from-surface",
                         "detail": f"machine consumes '{e}' but the contract's event-surface omits it"})

    phantom = surface_events - m_inputs - FOCUS
    for e in sorted(phantom):
        findings.append({"reason": "phantom-event",
                         "detail": f"contract declares event '{e}' the machine has no transition for"})

    # ---- 2. descriptor state range vs machine states
    m_states, gate, ostates = machine_states(machine)
    # states the descriptor can report = union of slots' present-in + events' valid-in (minus '*')
    reported = set()
    for slot in contract["descriptor"]["slots"]:
        reported.update(slot["present-in"])
    for ev in contract["event-surface"]["events"]:
        for s in ev["valid-in"]:
            if s != "*":
                reported.add(s)

    stray_states = reported - m_states
    for s in sorted(stray_states):
        findings.append({"reason": "descriptor-stray-state",
                         "detail": f"contract references state '{s}' the machine cannot be in "
                                   f"(machine states: {sorted(m_states)})"})

    unreported = m_states - reported
    for s in sorted(unreported):
        findings.append({"reason": "state-unreported",
                         "detail": f"machine state '{s}' is never referenced by any slot or event valid-in"})

    return (len(findings) == 0), findings


def _self_test():
    here = pathlib.Path(__file__).parent
    root = here / "../.."
    contract = load(root / "reference/single-select/single-select.mode-contract.json")
    bundle = load(root / "loom-spec-ref/single-select.reification.json") if (root/"loom-spec-ref").exists() else None
    # the machine lives in the loom-spec reference bundle; we vendor a copy for the self-test
    machine = load(here / "fixtures/single-select.machine.json")

    passed, total = 0, 0

    def case(name, contract, machine, expect):
        nonlocal passed, total
        total += 1
        ok, findings = check(contract, machine)
        good = ok == expect
        passed += good
        print(f"  [{'PASS' if good else 'FAIL'}] {name}: consistent={ok} (expected {expect})")
        for f in findings:
            print(f"           - {f['reason']}: {f['detail']}")

    # 1. the real contract against the real machine -> consistent
    case("single-select contract matches its machine", contract, machine, True)

    # 2. drop an event from the surface -> event-missing-from-surface
    broken = json.loads(json.dumps(contract))
    broken["event-surface"]["events"] = [e for e in broken["event-surface"]["events"] if e["name"] != "retry"]
    case("contract missing a machine event", broken, machine, False)

    # 3. add a phantom event -> phantom-event
    phantom = json.loads(json.dumps(contract))
    phantom["event-surface"]["events"].append({"name": "teleport", "valid-in": ["present:open"], "origin": "user"})
    case("contract with a phantom event", phantom, machine, False)

    # 4. reference a stray state -> descriptor-stray-state
    stray = json.loads(json.dumps(contract))
    stray["descriptor"]["slots"].append({"name": "ghost", "present-in": ["hovering"]})
    case("contract referencing an impossible state", stray, machine, False)

    print(f"\n{passed}/{total} self-test cases passed")
    return passed == total


def main():
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        sys.exit(0 if _self_test() else 1)
    if len(sys.argv) < 4:
        print(__doc__); sys.exit(2)
    contract = load(sys.argv[1])
    bundle = load(sys.argv[2])
    kind = sys.argv[3]
    machine = bundle["machines"][kind]
    ok, findings = check(contract, machine)
    print(f"contract-consistency: {'CONSISTENT' if ok else 'INCONSISTENT'}")
    for f in findings:
        print(f"  [{f['reason']}] {f['detail']}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
