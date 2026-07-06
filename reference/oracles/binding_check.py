#!/usr/bin/env python3
"""
binding_check — derive and verify the binding contract (the wiring of reified
components to the domain).

The binding is DERIVED from the What, never invented: the owning UI step already
names its projection and its commands, so for each composition-graph leaf:

  sensor side   — a machine with data-origin events binds to the step's declared
                  View; the state-map (projection state -> data event) is derived
                  from the intersection of the projection's state space and the
                  machine's data events.
  effector side — a machine that binds the Build seam (binds-seam:
                  build-seam:VerdictEvent) binds to a step-declared Command; the
                  verdict-map is fixed by the machine (admitted/rejected).

Derivation is unambiguous only when the step's declarations make it so. A step
offering several commands to one effector leaf CANNOT be derived — it is surfaced
as needs-authoring (the binding analogue of the annotation floor), and the
authored choice is then CHECKED against the step's declarations.

Checks (all MUST):
  1. declaration sourcing — a binding may reference only a View the owning step
     projects and a Command the owning step declares. Binding to anything else is
     binding-not-declared: the wiring cannot say what the What did not.
  2. state-map coverage — every state the projection can exhibit (beyond the
     initial 'loading') maps to a machine data event, and every mapped event
     exists in the machine. A projection that can fail whose binding never fires
     data-failed is a silent-failure wiring bug caught here, not in production.
  3. verdict coverage — an effector binding maps both 'admitted' and 'rejected';
     a rejected verdict with no wiring is an unreachable failed state.

The same bindings drive TWO conforming drivers: the gallery's FixtureDriver and
the product's LiveDriver (real Views, real VerdictEvents). Same contract, two
drivers — what we verify is what we build.

Run:  python3 binding_check.py --self-test
"""
import json, sys, pathlib


def load(p): return json.load(open(p))


def machine_data_events(machine):
    """Data-origin events = transition labels out of the machine's initial/loading
    state that feed it data (convention: data-*), plus none for effectors."""
    evs = set()
    for src, m in machine.get("transitions", {}).items():
        for e in m:
            if e.startswith("data-"):
                evs.add(e)
    return evs


def is_effector(machine):
    return machine.get("binds-seam") == "build-seam:VerdictEvent"


def derive(ui_intent, composition_graph, machines_by_kind):
    """Derive bindings for every leaf. Returns (bindings, needs_authoring)."""
    kinds = (ui_intent.get("producer-extension") or {}).get("aio-kinds", {})
    steps = ui_intent.get("ui-steps", [])
    bindings, needs_authoring = [], []

    for nid, node in composition_graph.get("nodes", {}).items():
        if "kind" not in node:
            continue
        kind = kinds.get(nid, node["kind"])
        machine = machines_by_kind.get(kind)
        if machine is None:
            continue

        # owning step: the fixture model is one step; with several, ownership
        # comes from the step<->node linkage the intent carries. Derive only
        # when unambiguous.
        if len(steps) != 1:
            needs_authoring.append({"node": nid, "reason": "owning-step-ambiguous",
                                    "detail": f"{len(steps)} steps; node-to-step ownership must be authored"})
            continue
        step = steps[0]
        b = {"node": nid, "step-id": step.get("step-id", "step"), "derivation": "derived"}

        data_evs = machine_data_events(machine)
        if data_evs:
            view = (step.get("projection") or {}).get("view")
            if not view:
                needs_authoring.append({"node": nid, "reason": "no-view-declared",
                                        "detail": "machine has data events but the step's projection names no view"})
                continue
            # state-map: projection states (from the step) -> machine data events
            pstates = [s for s in step.get("projection-states", []) if s != "loading"]
            smap = {}
            for s in pstates:
                ev = f"data-{'arrived' if s == 'present' else s}"
                if ev in data_evs:
                    smap[s] = ev
            b["sensor"] = {"view": view, "state-map": smap,
                           "slot-data": {"option-list": "rows", "trigger": "selected"} if kind == "single-select" else {}}

        if is_effector(machine):
            cmds = step.get("commands", [])
            if len(cmds) == 1:
                b["effector"] = {"command": cmds[0], "on-event": "activated",
                                 "verdict-map": {"admitted": "verdict-admitted",
                                                 "rejected": "verdict-rejected"}}
            elif len(cmds) == 0:
                needs_authoring.append({"node": nid, "reason": "no-command-declared",
                                        "detail": "effector leaf but the step declares no command"})
                continue
            else:
                needs_authoring.append({"node": nid, "reason": "command-ambiguous",
                                        "detail": f"step declares {len(cmds)} commands; the choice must be authored"})
                continue

        if "sensor" in b or "effector" in b:
            bindings.append(b)

    return bindings, needs_authoring


def check(bindings, ui_intent, machines_by_kind, composition_graph):
    """Verify (derived or authored) bindings against the What. Returns findings."""
    kinds = (ui_intent.get("producer-extension") or {}).get("aio-kinds", {})
    steps = {s.get("step-id", "step"): s for s in ui_intent.get("ui-steps", [])}
    findings = []

    for b in bindings:
        step = steps.get(b["step-id"])
        if step is None:
            findings.append({"reason": "binding-not-declared", "node": b["node"],
                             "detail": f"step '{b['step-id']}' does not exist in the intent"})
            continue
        kind = kinds.get(b["node"])
        machine = machines_by_kind.get(kind, {})

        # 1. declaration sourcing
        if "sensor" in b:
            declared_view = (step.get("projection") or {}).get("view")
            if b["sensor"]["view"] != declared_view:
                findings.append({"reason": "binding-not-declared", "node": b["node"],
                                 "detail": f"sensor binds view '{b['sensor']['view']}' but the step projects '{declared_view}'"})
            # 2. state-map coverage
            exhibitable = [s for s in step.get("projection-states", []) if s != "loading"]
            data_evs = machine_data_events(machine)
            for s in exhibitable:
                if s not in b["sensor"]["state-map"]:
                    findings.append({"reason": "state-unwired", "node": b["node"],
                                     "detail": f"projection can be '{s}' but the state-map never fires an event for it (silent-failure wiring)"})
            for s, ev in b["sensor"]["state-map"].items():
                if ev not in data_evs:
                    findings.append({"reason": "event-unknown", "node": b["node"],
                                     "detail": f"state-map fires '{ev}' which the machine has no transition for"})

        if "effector" in b:
            if b["effector"]["command"] not in step.get("commands", []):
                findings.append({"reason": "binding-not-declared", "node": b["node"],
                                 "detail": f"effector binds command '{b['effector']['command']}' the step does not declare"})
            # 3. verdict coverage
            vm = b["effector"].get("verdict-map", {})
            for outcome in ("admitted", "rejected"):
                if outcome not in vm:
                    findings.append({"reason": "verdict-unwired", "node": b["node"],
                                     "detail": f"no wiring for '{outcome}' — that verdict would be unreachable in the UI"})

    return (len(findings) == 0), findings


# ---------------------------------------------------------------- self-test
def _self_test():
    here = pathlib.Path(__file__).parent
    machines = {
        "single-select": load(here / "fixtures/single-select.machine.json"),
        "command": load(here / "fixtures/command.machine.json"),
    }
    intent = load(here / "../bindings/sample.ui-intent.json")
    comp = load(here / "../bindings/sample.composition-graph.json")

    passed = total = 0
    def case(name, cond):
        nonlocal passed, total
        total += 1; passed += bool(cond)
        print(f"  [{'PASS' if cond else 'FAIL'}] {name}")

    # derivation on the reference settings screen
    bindings, na = derive(intent, comp, machines)
    case("derives 2 bindings, nothing needs authoring", len(bindings) == 2 and not na)
    tz = next(b for b in bindings if b["node"] == "tz-field")
    save = next(b for b in bindings if b["node"] == "save-cmd")
    case("tz-field: sensor bound to the step's declared view",
         tz["sensor"]["view"] == "timezone-options")
    case("tz-field: state-map covers present/empty/failed",
         set(tz["sensor"]["state-map"]) == {"present", "empty", "failed"})
    case("save-cmd: effector bound to the single declared command",
         save["effector"]["command"] == "set-timezone")
    case("save-cmd: both verdicts wired",
         set(save["effector"]["verdict-map"]) == {"admitted", "rejected"})

    # derived bindings check clean
    ok, f = check(bindings, intent, machines, comp)
    case("derived bindings pass their own check", ok)

    # negative: authored binding to an undeclared command
    bad = json.loads(json.dumps(bindings))
    next(b for b in bad if b["node"] == "save-cmd")["effector"]["command"] = "delete-account"
    ok, f = check(bad, intent, machines, comp)
    case("binding to undeclared command: caught",
         not ok and any(x["reason"] == "binding-not-declared" for x in f))

    # negative: silent failure — drop the failed mapping
    bad2 = json.loads(json.dumps(bindings))
    del next(b for b in bad2 if b["node"] == "tz-field")["sensor"]["state-map"]["failed"]
    ok, f = check(bad2, intent, machines, comp)
    case("projection-can-fail but failed unwired: caught",
         not ok and any(x["reason"] == "state-unwired" for x in f))

    # negative: unwired rejection verdict
    bad3 = json.loads(json.dumps(bindings))
    del next(b for b in bad3 if b["node"] == "save-cmd")["effector"]["verdict-map"]["rejected"]
    ok, f = check(bad3, intent, machines, comp)
    case("rejected verdict unwired: caught",
         not ok and any(x["reason"] == "verdict-unwired" for x in f))

    # ambiguity: a second command makes derivation refuse, not guess
    intent2 = json.loads(json.dumps(intent))
    intent2["ui-steps"][0]["commands"].append("set-locale")
    _, na2 = derive(intent2, comp, machines)
    case("two commands: derivation refuses (needs-authoring), never guesses",
         any(x["reason"] == "command-ambiguous" for x in na2))

    print(f"\n{passed}/{total} self-test cases passed")
    return passed == total


if __name__ == "__main__":
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        sys.exit(0 if _self_test() else 1)
    print(__doc__)
