# S0 — Memra Contract (Capability Manifest) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> **FOREGROUND rule:** any step marked FOREGROUND uses the oracle emulator MCP and MUST run in the controller session, never a subagent (oracle MCP from background agents deadlocks).

**Goal:** Produce the machine-readable Memra capability manifest (generated core + curated overlay), drift-checked in aeon's build, published to empyrean with a JSON Schema — per the approved spec `../specs/2026-07-03-s0-memra-contract-design.md`.

**Architecture:** Refactor `song_packer.py`'s per-Event imperative `validate()` methods into one declarative `OPCODE_SPECS` table interpreted by a generic validator (behavior-preserving, guarded by a rejection-matrix parity test + ROM byte-identity). A new generator merges that table + parsed `sound_constants.asm` structs + a hand-curated semantics module into `memra-manifest.json`; a drift validator runs in `build.sh`; a release-diff mode enforces breaking-change classes.

**Tech Stack:** Python 3 (stdlib only — the curated overlay is a Python module, NOT YAML, to avoid a PyYAML dependency; `jsonschema` used only in an optional skip-if-missing test), AS assembler build (`./build.sh`), oracle MCP for the conformance smoke.

**Repos touched:** aeon (branch `feat/s0-memra-contract`), empyrean (direct commits, additive only), seraph (queue-doc status update at the end).

**Decisions pinned at plan time (from spec §4.2):** the optional packed-song-header compat byte is **DEFERRED** — zero engine changes in S0. Parity with CURRENT packer behavior wins over spec-idealized behavior everywhere they differ (e.g. `PsgEnv` today is legal on every non-FM route including DAC; the manifest records reality and the curated overlay flags the discrepancy — do NOT silently tighten).

---

### Task 1: Branch + deterministic golden baseline

**Files:** none created (baseline hashes recorded in Task 2's JSON).

- [ ] **Step 1: Create the aeon feature branch**

```bash
cd /home/volence/sonic_hacks/aeon && git checkout master && git pull --ff-only 2>/dev/null; git checkout -b feat/s0-memra-contract
```

- [ ] **Step 2: Verify the sound build is deterministic (two builds, same hash)**

```bash
cd /home/volence/sonic_hacks/aeon
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh && sha256sum s4.bin | tee /tmp/s0_build1.txt
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh && sha256sum s4.bin | tee /tmp/s0_build2.txt
diff /tmp/s0_build1.txt /tmp/s0_build2.txt && echo DETERMINISTIC
```

Expected: `DETERMINISTIC`. If the hashes differ, STOP — find the nondeterminism source before proceeding (the ROM-identity gate in Task 3 depends on it). Record the hash; it is the golden ROM hash for this plan.

- [ ] **Step 3: Confirm the existing packer test suite is green**

Run: `cd /home/volence/sonic_hacks/aeon && python3 -m pytest tools/test_song_packer.py tools/test_sfx_transcode.py -q`
Expected: all pass, 0 failures.

---

### Task 2: Rejection-matrix parity harness (BEFORE any refactor)

Captures the CURRENT packer's accept/reject behavior for every Event × route × boundary operand, so Task 3's refactor is mechanically verifiable.

**Files:**
- Create: `tools/test_packer_parity.py`
- Create (generated, committed): `tools/testdata/packer_rejection_baseline.json`

- [ ] **Step 1: Write the parity harness**

Create `tools/test_packer_parity.py`:

```python
#!/usr/bin/env python3
"""Rejection-matrix parity harness for the S0 OPCODE_SPECS refactor.

Sweeps every Event class x every route (0..10) x boundary operand values
against song_packer's validate(), records accept/reject, and compares to the
committed baseline JSON. Regenerate ONLY when intentionally changing format
rules:  python3 tools/test_packer_parity.py --regen
Run as a test:  python3 -m pytest tools/test_packer_parity.py -q
"""
import json, os, sys, itertools
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import song_packer as sp

BASELINE = os.path.join(os.path.dirname(os.path.abspath(__file__)),
                        "testdata", "packer_rejection_baseline.json")
ROUTES = list(range(sp.CHROUTE_COUNT))

# Boundary probe values per constructor-arg kind. Each case = (label, ctor_args).
# Values chosen to straddle every range edge in the current validate() bodies.
def _cases():
    B = [-129, -128, -1, 0, 1, 3, 4, 5, 0x0F, 0x10, 0x5E, 0x5F, 0x7F, 0x80,
         0xDF, 0xE0, 0xEF, 0xF0, 0xFE, 0xFF, 0x100, 127, 128, 255, 256]
    out = []
    for v in B:
        out += [("SetDur", (v,)), ("Note", (v,)), ("Vol", (v,)),
                ("Patch", (v,)), ("Dac", (v,)), ("NoteFill", (v,)),
                ("Pan", (v,)), ("PsgEnv", (v,)), ("FmEnv", (v,)),
                ("PsgNoise", (v,)), ("Detune", (v,)), ("Porta", (v,)),
                ("Lfo", (v,)), ("Tempo", (v,)), ("RepeatEnd", (v,))]
        out += [("NoteDur", (v, 1)), ("NoteDur", (0, v)),
                ("NoteRaw", (v, 0, 1)), ("NoteRaw", (0, v, 1)), ("NoteRaw", (0, 0, v)),
                ("OpBias", (v, 0)), ("OpBias", (0, v)),
                ("ModSet", (v, 0, 0, 0)), ("ModSet", (0, v, 0, 0)),
                ("ModSet", (0, 0, v, 0)), ("ModSet", (0, 0, 0, v))]
    out += [("Rest", ()), ("SpinRev", ()), ("RepeatStart", ()),
            ("LoopPoint", ()), ("Jump", ()), ("End", ()), ("Macro", ())]
    # RegWrite: full guard surface — parts, DAC door, forbidden regs.
    for part in (-1, 0, 1, 2):
        for reg in (0x22, 0x24, 0x27, 0x28, 0x2A, 0x2B, 0x30, 0xB4, 0xB6, 0xFF, 0x100):
            out.append(("RegWrite", (part, reg, 0)))
    out += [("RegWrite", (0, 0x30, -1)), ("RegWrite", (0, 0x30, 256))]
    # PitchEnv: point-count + point-range edges.
    for pts in ([], [0], [0]*5, [0]*6, [0x83], [0x84], [0, 0x83, 5]):
        out.append(("PitchEnv", (list(pts),)))
    # RegDelta: entry-count + reg_sel/value edges (entries = [(reg_sel, val)]).
    for entries in ([], [(0, 0)], [(0x17, 0xFF)], [(0x18, 0)], [(0xFF, 0)],
                    [(0, -1)], [(0, 256)], [(0, 0)]*255, [(0, 0)]*256):
        out.append(("RegDelta", (list(entries),)))
    return out

def sweep():
    results = {}
    for (cls_name, args), route in itertools.product(_cases(), ROUTES):
        key = f"{cls_name}{args!r}@r{route}"
        try:
            ev = getattr(sp, cls_name)(*args)
            ev.validate(route)
            results[key] = "accept"
        except sp.PackError as e:
            results[key] = "reject"
        except Exception as e:                       # ctor blew up, etc.
            results[key] = f"error:{type(e).__name__}"
    return results

def test_parity_matches_baseline():
    with open(BASELINE) as f:
        base = json.load(f)
    now = sweep()
    diffs = {k: (base.get(k), now.get(k))
             for k in set(base) | set(now) if base.get(k) != now.get(k)}
    assert not diffs, f"{len(diffs)} parity diffs, e.g. {list(diffs.items())[:8]}"

if __name__ == "__main__":
    if "--regen" in sys.argv:
        os.makedirs(os.path.dirname(BASELINE), exist_ok=True)
        with open(BASELINE, "w") as f:
            json.dump(sweep(), f, indent=0, sort_keys=True)
        print(f"baseline regenerated: {len(sweep())} cases -> {BASELINE}")
    else:
        test_parity_matches_baseline(); print("parity OK")
```

- [ ] **Step 2: Generate the baseline against the UNMODIFIED packer**

Run: `cd /home/volence/sonic_hacks/aeon && python3 tools/test_packer_parity.py --regen`
Expected: `baseline regenerated: <N> cases` with N in the tens of thousands (25 probes × ~40 shapes × 11 routes). Sanity-check a few entries: `python3 - <<'EOF'` … load the JSON, assert `"Vol((127,))"`-style keys exist with `accept` on FM routes and `"Vol((128,))"` is `reject`. (Exact key format: `Vol((127,))@r0`.)

- [ ] **Step 3: Verify the test passes against the baseline**

Run: `python3 -m pytest tools/test_packer_parity.py -q`
Expected: 1 passed.

- [ ] **Step 4: Commit (exact paths, never -A)**

```bash
git add tools/test_packer_parity.py tools/testdata/packer_rejection_baseline.json
git commit -m "test(sound): rejection-matrix parity baseline for the S0 packer refactor"
```

---

### Task 3: OPCODE_SPECS refactor (behavior-preserving, golden-guarded)

**Files:**
- Modify: `tools/song_packer.py` (constants block ends ~line 131; Event classes ~lines 140–800)

- [ ] **Step 1: Add the declarative spec table + generic validator**

Insert after the `PackError` class (line ~136) in `tools/song_packer.py`. The table is keyed by Event CLASS NAME; operand entries name the instance attribute they check. `routes` is the LEGAL set (`None` = all 11 routes legal — matching validators that never checked route). Special guard kinds carry their own data.

```python
# --- S0: declarative opcode/operand/route spec (the manifest's generated core).
# THE format authority: Event.validate() interprets THIS TABLE, and
# tools/gen_capability_manifest.py generates the Memra manifest from it, so the
# contract and the enforcer cannot drift. Parity with the pre-refactor
# validators is pinned by tools/test_packer_parity.py — change behavior ONLY
# with a deliberate baseline regen.
ALL_ROUTES = frozenset(range(CHROUTE_COUNT))
_NON_FM_ROUTES = ALL_ROUTES - _FM_ROUTES          # PSG1-3, PSGN, DAC

OPCODE_SPECS = {
    #  name        : dict(opcode=<byte|None>, tick="advance"|"zero",
    #                     routes=<frozenset|None>, ops=[(attr, lo, hi), ...],
    #                     special=<None|str>)  — special guards implemented in
    #                     _SPECIAL_GUARDS below, still driven by data here.
    "SetDur":      dict(opcode=None,           tick="zero",    routes=None,
                        ops=[("ticks", 0, MAX_DUR)]),
    "Rest":        dict(opcode=MEV_REST,       tick="advance", routes=None, ops=[]),
    "Note":        dict(opcode=None,           tick="advance", routes=None,
                        ops=[("pitch", 0, MAX_PITCH)]),
    "NoteDur":     dict(opcode=MEV_NOTE_DUR,   tick="advance", routes=None,
                        ops=[("pitch", 0, MAX_PITCH), ("dur", 0, 0xFF)]),
    "NoteRaw":     dict(opcode=MEV_NOTE_RAW,   tick="advance", routes=_FM_ROUTES,
                        ops=[("a4", 0, 0xFF), ("a0", 0, 0xFF), ("dur", 1, 0xFF)]),
    "PitchEnv":    dict(opcode=MEV_PITCHENV,   tick="advance", routes=_FM_ROUTES,
                        ops=[], special="pitchenv"),
    "Vol":         dict(opcode=MEV_VOL,        tick="zero",    routes=None,
                        ops=[("vol", 0, 127)]),
    "Patch":       dict(opcode=MEV_PATCH,      tick="zero",    routes=_FM_ROUTES, ops=[]),
    "Dac":         dict(opcode=MEV_DAC,        tick="zero",
                        routes=frozenset({CHROUTE_DAC}), ops=[]),
    "Pan":         dict(opcode=MEV_PAN,        tick="zero",    routes=None,
                        ops=[("b4", 0, 0xFF)]),
    "NoteFill":    dict(opcode=MEV_NOTEFILL,   tick="zero",    routes=_FM_ROUTES,
                        ops=[("master", 0, 255)]),
    "PsgEnv":      dict(opcode=MEV_PSGENV,     tick="zero",    routes=_NON_FM_ROUTES,
                        ops=[("env_id", 0, 0xFF)]),
    "FmEnv":       dict(opcode=MEV_FMENV,      tick="zero",    routes=_FM_ROUTES,
                        ops=[("env_id", 0, 0xFF)]),
    "PsgNoise":    dict(opcode=MEV_PSGNOISE,   tick="zero",
                        routes=frozenset({CHROUTE_PSGN}), ops=[("ctrl", 0xE0, 0xEF)]),
    "Detune":      dict(opcode=MEV_DETUNE,     tick="zero",    routes=None,
                        ops=[("detune", -128, 127)]),
    "Porta":       dict(opcode=MEV_PORTA,      tick="zero",    routes=None,
                        ops=[("rate", 0, 0xFF)]),
    "Lfo":         dict(opcode=MEV_LFO,        tick="zero",    routes=None,
                        ops=[("value", 0, 0x0F)]),
    "Tempo":       dict(opcode=MEV_TEMPO,      tick="zero",    routes=None,
                        ops=[("mod", 0, 0xFE)]),
    "ModSet":      dict(opcode=MEV_MODSET,     tick="zero",    routes=None,
                        ops=[("wait", 0, 0xFF), ("speed", 0, 0xFF),
                             ("change", -128, 127), ("step", 0, 0xFF)]),
    "OpBias":      dict(opcode=MEV_OPBIAS,     tick="zero",    routes=_FM_ROUTES,
                        ops=[("op", 0, 3), ("val", -128, 127)]),
    "RegDelta":    dict(opcode=MEV_REGDELTA,   tick="zero",    routes=_FM_ROUTES,
                        ops=[], special="regdelta"),
    "RegWrite":    dict(opcode=MEV_REGWRITE,   tick="zero",    routes=None,
                        ops=[("part", 0, 1), ("reg", 0, 0xFF), ("val", 0, 0xFF)],
                        special="regwrite"),
    "Macro":       dict(opcode=MEV_MACRO,      tick="zero",    routes=_FM_ROUTES, ops=[]),
    "SpinRev":     dict(opcode=MEV_SPINREV,    tick="zero",    routes=None, ops=[]),
    "RepeatStart": dict(opcode=MEV_REPEAT_START, tick="zero",  routes=None, ops=[]),
    "RepeatEnd":   dict(opcode=MEV_REPEAT_END, tick="zero",    routes=None,
                        ops=[("count", 1, 255)]),
    "LoopPoint":   dict(opcode=MEV_LOOP_POINT, tick="zero",    routes=None, ops=[]),
    "Jump":        dict(opcode=MEV_JUMP,       tick="zero",    routes=None, ops=[]),
    "End":         dict(opcode=MEV_END,        tick="zero",    routes=None, ops=[]),
}

# Data for the special guards (kept OUT of code paths so the manifest can
# serialize them — spec rule: every constraint expressible as data).
REGWRITE_GUARD = dict(
    forbidden_regs=(0x2A, 0x2B),          # DAC data/enable
    forbidden_reg_range=(0x24, 0x27),     # YM timer block (frame clock)
    dac_door=(1, 0xB6),                   # DAC route may write ONLY part1 $B6
)
REGDELTA_GUARD = dict(entry_count=(1, 255), reg_sel_max_group=None,  # filled below
                      value_range=(0, 0xFF))
PITCHENV_GUARD = dict(point_count=(1, 5), point_range=(0, PITCHENV_MAX_IDX))


def _spec_validate(ev, route):
    """Generic validator: interprets OPCODE_SPECS + *_GUARD data. Raises PackError
    with messages equivalent in MEANING to the pre-refactor text (parity test
    checks accept/reject only, not message text)."""
    spec = OPCODE_SPECS[type(ev).__name__]
    name = type(ev).__name__
    if spec["routes"] is not None and route not in spec["routes"]:
        # RegWrite has a route EXCEPTION (DAC door) handled in its special guard.
        raise PackError(f"{name} illegal on route {route}")
    for attr, lo, hi in spec["ops"]:
        v = getattr(ev, attr)
        if not (lo <= v <= hi):
            raise PackError(f"{name}.{attr} {v} out of range {lo}..{hi}")
    special = spec.get("special")
    if special:
        _SPECIAL_GUARDS[special](ev, route)
```

- [ ] **Step 2: Implement the three special guards (data-driven)**

Add immediately below, transcribing the CURRENT logic exactly (see the pre-refactor bodies: `RegWrite.validate` lines ~300–327, `RegDelta.validate` ~lines 749–764, `PitchEnv.validate` ~lines 675–683 — read them before deleting):

```python
def _guard_regwrite(ev, route):
    # Route legality with the narrow DAC door — mirrors pre-refactor order:
    # DAC route: only (part,reg) == dac_door; other non-FM routes: reject.
    if route == CHROUTE_DAC:
        if (ev.part, ev.reg) != REGWRITE_GUARD["dac_door"]:
            raise PackError(f"RegWrite on the DAC route is limited to part 1 reg $B6")
    elif route not in _FM_ROUTES:
        raise PackError(f"RegWrite on non-FM route {route}")
    if ev.reg in REGWRITE_GUARD["forbidden_regs"]:
        raise PackError(f"RegWrite reg {ev.reg:#x} is a DAC register — refused")
    lo, hi = REGWRITE_GUARD["forbidden_reg_range"]
    if lo <= ev.reg <= hi:
        raise PackError(f"RegWrite reg {ev.reg:#x} is in the YM timer block — refused")

def _guard_regdelta(ev, route):
    lo, hi = REGDELTA_GUARD["entry_count"]
    if not (lo <= len(ev.entries) <= hi):
        raise PackError(f"RegDelta entry count {len(ev.entries)} out of {lo}..{hi}")
    vlo, vhi = REGDELTA_GUARD["value_range"]
    for rs, val in ev.entries:
        if not (0 <= rs <= 0xFF):
            raise PackError(f"RegDelta reg_sel {rs} out of byte range")
        if (rs >> 2) >= REGDELTA_GROUP_COUNT:
            raise PackError(f"RegDelta reg_sel {rs:#x} group out of range")
        if not (vlo <= val <= vhi):
            raise PackError(f"RegDelta value {val} out of byte range")

def _guard_pitchenv(ev, route):
    lo, hi = PITCHENV_GUARD["point_count"]
    if not (lo <= len(ev.points) <= hi):
        raise PackError(f"PitchEnv point count {len(ev.points)} out of {lo}..{hi}")
    plo, phi = PITCHENV_GUARD["point_range"]
    for p in ev.points:
        if not (plo <= p <= phi):
            raise PackError(f"PitchEnv point {p} out of {plo}..{phi}")

_SPECIAL_GUARDS = {"regwrite": _guard_regwrite, "regdelta": _guard_regdelta,
                   "pitchenv": _guard_pitchenv}
```

IMPORTANT ordering note: the pre-refactor `RegWrite.validate` checks route FIRST, then part, then forbidden regs, then ranges. `_spec_validate` runs `routes` (skipped — RegWrite's spec routes=None so the special guard owns routing), then ops ranges (part/reg/val), then the special guard. That reorders "part out of range" before "DAC door" — the parity test only records accept/reject (never which message), and every reordered case still rejects, so parity holds. Verify with the Step 4 run; if any case flips accept↔reject, the table is wrong — fix the table, never the baseline.

Note `RegDelta`/`PitchEnv` deliberately verify the exact same predicates (`(rs >> 2) >= REGDELTA_GROUP_COUNT` mirrors the current group check — confirm the exact expression against the live code at `reg_sel`/`RegDelta.validate` before deleting it; if the current code validates via the `reg_sel()` helper's assertions instead, transcribe THAT predicate).

- [ ] **Step 3: Replace every Event.validate() body with the generic call**

For EACH class in `OPCODE_SPECS` that currently defines `validate()`, delete its body and rely on the base class; change the BASE `Event.validate` to:

```python
class Event:
    """Base event. `encode()` -> bytes; `validate(route)` raises PackError."""
    def encode(self) -> bytes:
        raise NotImplementedError

    def validate(self, route: int) -> None:
        if type(self).__name__ in OPCODE_SPECS:
            _spec_validate(self, route)
```

Keep every `encode()`, constructor, docstring, and class constant (e.g. `Pan.PAN_LEFT`) EXACTLY as they are. Do NOT touch the `MacEvent` classes, `pack_song`, `emit_asm`, or `_validate_channel` in this task. Since `OPCODE_SPECS`/guards reference `PITCHENV_MAX_IDX` and `REGDELTA_GROUP_COUNT` which are defined lower in the file, place the whole spec block AFTER those constants (or move just those two constants up — prefer moving the spec block down to just above `pack_song`; Python resolves names at call time for the guards but the table literal needs them at import — verify import succeeds).

- [ ] **Step 4: Run the parity test — the refactor gate**

Run: `python3 -m pytest tools/test_packer_parity.py tools/test_song_packer.py tools/test_sfx_transcode.py tools/test_smps_import.py tools/test_zyrinx_port.py -q`
Expected: ALL pass. Any parity diff = the table mis-transcribes a rule; fix `OPCODE_SPECS`/guards (NEVER regen the baseline in this task).

- [ ] **Step 5: ROM byte-identity gate**

```bash
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh && sha256sum s4.bin
```

Expected: hash identical to Task 1 Step 2. (The daemon may plain-rebuild mid-session — if the hash differs, FIRST re-run the exact command and re-compare before debugging; per repo memory, byte-verify ROM-vs-build-flags.)

- [ ] **Step 6: Commit**

```bash
git add tools/song_packer.py
git commit -m "refactor(sound): song_packer validation -> declarative OPCODE_SPECS (parity-pinned, ROM byte-identical)"
```

---

### Task 4: Curated overlay module

**Files:**
- Create: `tools/sound_manifest_curated.py`

- [ ] **Step 1: Write the curated overlay**

A plain Python module (comments beat YAML; zero deps). It carries EVERYTHING the source can't express: semantics, units, channel classes, features, limits, rules. Transcribe semantics from `seraph/docs/superpowers/2026-07-03-memra-author-surface-inventory.md` — the file below is complete except where marked `# …` with an explicit count; fill those from the inventory's matching section (they are enumerations, not designs):

```python
"""Curated overlay for the Memra capability manifest (S0).
Everything here is HAND-MAINTAINED semantic truth that cannot be extracted
from song_packer.py / sound_constants.asm. gen_capability_manifest.py FAILS
if any OPCODE_SPECS entry lacks a curated entry here (no silent surface).
Sources of truth for the prose: the author-surface inventory (seraph repo)
and the banked engine specs. Keep units machine-usable (enum of known
strings), never free prose in `unit`."""

FORMAT_VERSION = 1                # manifest schema shape (append-only rule)
DRIVER_COMPAT = {"major": 0, "minor": 1}   # Music v0; same-major = compatible

# unit vocabulary (schema enum): "linear_0_127", "frames", "ticks",
# "fnum_units_per_frame", "signed_delta", "raw_ym_byte", "table_index",
# "pitch_index", "enum", "s3k_tempo_addend", "boolean"

OPCODES = {
    # name: dict(desc, operand_units=[...], feature=None|str, notes=None|str)
    "SetDur":      dict(desc="Set default note/rest duration", operand_units=["ticks"]),
    "Rest":        dict(desc="Rest for default duration (key-off + advance)", operand_units=[]),
    "Note":        dict(desc="Key note at pitch index for default duration",
                        operand_units=["pitch_index"]),
    "NoteDur":     dict(desc="Key note with explicit duration",
                        operand_units=["pitch_index", "frames"]),
    "NoteRaw":     dict(desc="Key raw-frequency FM note (exact $A4/$A0)",
                        operand_units=["raw_ym_byte", "raw_ym_byte", "frames"]),
    "PitchEnv":    dict(desc="Pitch-envelope note + key-on (trill/arp, 1-5 points)",
                        operand_units=["table_index"]),
    "Vol":         dict(desc="Set channel volume", operand_units=["linear_0_127"]),
    "Patch":       dict(desc="Set FM patch index (applies at next key-on)",
                        operand_units=["table_index"]),
    "Dac":         dict(desc="Trigger DAC sample by id", operand_units=["table_index"]),
    "Pan":         dict(desc="Set pan/AMS/FMS (raw YM $B4 byte)",
                        operand_units=["raw_ym_byte"],
                        notes="bit7=L, bit6=R, bits5-4 AMS, bits2-0 FMS"),
    "NoteFill":    dict(desc="Note-fill gate: frames keyed before early key-off (0=legato)",
                        operand_units=["frames"]),
    "PsgEnv":      dict(desc="Arm PSG volume-envelope id (1-based, 0=off)",
                        operand_units=["table_index"],
                        notes="PACKER accepts any non-FM route incl. DAC; engine "
                              "renders on PSG routes only — recorded discrepancy, "
                              "do not tighten without a baseline regen"),
    "FmEnv":       dict(desc="Arm FM carrier-TL volume-envelope id (1-based, 0=off)",
                        operand_units=["table_index"]),
    "PsgNoise":    dict(desc="Set SN76489 noise control byte (mode+rate; rate-3 "
                             "couples to PSG3 tone and silences it)",
                        operand_units=["enum"]),
    "Detune":      dict(desc="Set fine-pitch detune (applied at note-on)",
                        operand_units=["signed_delta"]),
    "Porta":       dict(desc="Set portamento glide rate (0=off); must follow a prior note",
                        operand_units=["fnum_units_per_frame"]),
    "Lfo":         dict(desc="Write YM $22 global LFO (bit3 enable | bits0-2 rate) — "
                             "GLOBAL: one unit for the whole chip",
                        operand_units=["enum"]),
    "Tempo":       dict(desc="Set GLOBAL tempo mod (S3K TempoWait addend; 0=full speed; "
                             "$FF reserved for the 68k restore mailbox)",
                        operand_units=["s3k_tempo_addend"]),
    "ModSet":      dict(desc="Latch software-vibrato params (wait/speed/change/step; all 0=off)",
                        operand_units=["frames", "frames", "signed_delta", "ticks"]),
    "OpBias":      dict(desc="Per-operator additive TL bias (negative=brighter); "
                             "applied at patch load",
                        operand_units=["enum", "signed_delta"]),
    "RegDelta":    dict(desc="Mid-note minimal register writes (timbre morph; never re-keys)",
                        operand_units=["enum", "raw_ym_byte"]),
    "RegWrite":    dict(desc="Raw YM register write escape hatch ($2A/$2B/$24-$27 refused; "
                             "DAC route limited to part1 $B6)",
                        operand_units=["enum", "raw_ym_byte", "raw_ym_byte"]),
    "Macro":       dict(desc="(Re)arm the slot[1] macro/automation stream",
                        operand_units=["table_index"]),
    "SpinRev":     dict(desc="Add global spindash rev to transpose (SFX feature)",
                        operand_units=[], feature="memra.sfx.spinrev"),
    "RepeatStart": dict(desc="Start of a repeatable body (single-level only)", operand_units=[]),
    "RepeatEnd":   dict(desc="Replay body N times (1-255; 0 ILLEGAL)", operand_units=["ticks"]),
    "LoopPoint":   dict(desc="Loop-target marker", operand_units=[]),
    "Jump":        dict(desc="Jump to loop point (song loops forever)", operand_units=[]),
    "End":         dict(desc="End of stream — channel idles; all channels ended = "
                             "song-finished status", operand_units=[]),
}

CHANNELS = {
    "fm":        dict(routes=[0, 1, 2, 3, 4, 5], pitch_range=[0, 0x5E],
                      steal=["protected", "protected", "stealable", "stealable",
                             "stealable", "adaptive"],
                      notes="FM1/FM2 never SFX-stolen; FM6 = adaptive DAC slot"),
    "psg_tone":  dict(routes=[6, 7, 8], pitch_range=[0, 0x5E],
                      steal=["stealable"] * 3,
                      notes="PSG3 (route 8) is the rate-3 noise clock source"),
    "psg_noise": dict(routes=[9], steal=["stealable"],
                      notes="rate-3 mode silences PSG3 tone (hardware coupling)"),
    "dac":       dict(routes=[10], steal=["protected"],
                      notes="trigger-only lane; FM6 tri-mode governs playback"),
}

FM6_MODES = dict(flag_bits={"SH_F_FM6_FM": 0, "SH_F_STREAM": 1, "SH_F_FM6_ADAPTIVE": 2},
                 modes=["dedicate", "full_fm6", "time_share"],
                 rule="SH_F_FM6_ADAPTIVE requires SH_F_FM6_FM")

FEATURES = {
    # status: shipped | experimental | reserved ; enginePackage: aeon queue #
    "memra.comm":          dict(status="reserved", enginePackage=1, budgetGated=False,
                                desc="MEV_EXT sub-op 0: score-authored cue byte"),
    "memra.ext.pumpset":   dict(status="reserved", enginePackage=5, budgetGated=False,
                                desc="MEV_EXT sub-op 1: kick-sidechain pump"),
    "memra.ext.ghostset":  dict(status="reserved", enginePackage=5, budgetGated=True,
                                desc="MEV_EXT sub-op 2: unified echo/unison ghost voice"),
    "memra.fm3.optracks":  dict(status="reserved", enginePackage=5, budgetGated=True,
                                desc="ExtCh3 per-operator routes (alg-7 chord mode)"),
    "memra.mac.pan":       dict(status="reserved", enginePackage=5, budgetGated=False,
                                desc="slot[1] TAG_MAC_PAN autopan tag"),
    "memra.jingle":        dict(status="reserved", enginePackage=1, budgetGated=False,
                                desc="jingle push/pop with mid-song resume"),
    "memra.sfx.spinrev":   dict(status="shipped", enginePackage=None, budgetGated=False,
                                desc="spindash rev transpose (SFX streams only)"),
    "memra.macro":         dict(status="shipped", enginePackage=None, budgetGated=False,
                                desc="slot[1] macro/automation spine"),
    # Plus one entry per shipped expression feature: memra.portamento,
    # memra.detune, memra.vibrato, memra.pitchenv, memra.fmenv, memra.psgenv,
    # memra.regdelta, memra.notefill, memra.psgnoise, memra.lfo, memra.tempo —
    # all status="shipped", enginePackage=None, budgetGated=False, desc from
    # the matching OPCODES entry. (11 entries; enumerate them all explicitly.)
}

LIMITS = dict(
    blob_max_bytes=0xFFFF, channel_count=[1, 11], repeat_nesting=1,
    fm_patch_bytes=32, dac_sample_descriptor_bytes=9,
    dac_sample_max_bytes=32768, dac_no_bank_straddle=0x8000,
    fm_patch_loads_per_frame=1,
    pitch_table_entries=132, macro_tag_namespace=["0xE0", "0xE3"],
)

RULES = [
    dict(id="init_order_fm", cls="init_order",
         prose="FM channels: Patch AND Vol before the first time-advancing event"),
    dict(id="init_order_psg", cls="init_order",
         prose="PSG/noise channels: Vol before the first time-advancing event"),
    dict(id="stream_termination", cls="structure",
         prose="Every stream non-empty and ends with Jump ($EF) or End ($FF); "
               "Jump requires a prior LoopPoint"),
    dict(id="loop_body_advances", cls="structure",
         prose="Loop and repeat bodies must contain >=1 time-advancing event"),
    dict(id="repeat_single_level", cls="structure",
         prose="Repeats do not nest; count 0 is illegal"),
    dict(id="porta_needs_note", cls="sequence",
         prose="Porta must follow a prior note on the channel (not packer-enforced "
               "today — exporter responsibility)"),
    dict(id="macro_body_rules", cls="macro",
         prose="Macro body non-empty, ends END/LOOP, >=1 NEXT before LOOP, operand "
               "bytes never 0xE0-0xE3, reg writes guarded like RegWrite"),
    dict(id="jingle_class", cls="song_class",
         prose="Jingles: <=3 voices, FM4/FM5+PSG windows, no FM6/DAC, no loop"),
    dict(id="music_illegal", cls="route",
         prose="MEV_SPINREV_RESET ($F1) never appears in a music stream"),
]
```

- [ ] **Step 2: Import-sanity test**

Run: `python3 -c "import sys; sys.path.insert(0,'tools'); import sound_manifest_curated as c; assert len(c.OPCODES)==29, len(c.OPCODES); print('curated OK')"`
Expected: `curated OK` (29 = the exact count of OPCODE_SPECS entries; if OPCODE_SPECS gained/lost entries in Task 3, use its real `len` — the two sets must be EQUAL, which Task 5's generator enforces).

- [ ] **Step 3: Commit**

```bash
git add tools/sound_manifest_curated.py
git commit -m "docs(sound): curated semantic overlay for the Memra manifest"
```

---

### Task 5: Manifest generator

**Files:**
- Create: `tools/gen_capability_manifest.py`
- Create (generated, committed): `games/sonic4/data/generated/memra-manifest.json` (add dir if absent; check `git check-ignore` — if the generated dir is ignored, commit the manifest at `tools/testdata/memra-manifest.json` instead and note it)
- Test: `tools/test_gen_capability_manifest.py`

- [ ] **Step 1: Write the generator**

```python
#!/usr/bin/env python3
"""Generate memra-manifest.json: OPCODE_SPECS (song_packer) + parsed
sound_constants.asm structs + sound_manifest_curated overlay.
Usage: python3 tools/gen_capability_manifest.py [--out PATH] [--check PATH]
--check re-generates and byte-compares against PATH (exit 1 on mismatch)."""
import json, os, re, sys, argparse, subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import song_packer as sp
import sound_manifest_curated as cur

ASM = os.path.join(HERE, "..", "sound_constants.asm")

def _routes_list(routes):
    return sorted(routes) if routes is not None else sorted(sp.ALL_ROUTES)

def _extract_asm_constants():
    """Parse `NAME = value` / `NAME equ value` lines for MEV_*/CHROUTE_*/SH_F_*.
    Values are decimal or $hex / 0xhex. Used ONLY for the drift cross-check."""
    consts = {}
    pat = re.compile(r"^\s*(MEV_\w+|CHROUTE_\w+|SH_F_\w+)\s*(?:=|equ)\s*"
                     r"(\$[0-9A-Fa-f]+|0x[0-9A-Fa-f]+|\d+)(?:\s*<<\s*(\d+))?", re.M)
    with open(ASM) as f:
        for m in pat.finditer(f.read()):
            name, raw, shift = m.groups()
            v = int(raw.replace("$", "0x"), 16) if ("$" in raw or "x" in raw) else int(raw)
            if shift:
                v <<= int(shift)
            consts[name] = v
    return consts

def build_manifest():
    # Cross-check: packer mirrors == asm truth (the two hand-synced sets).
    asm = _extract_asm_constants()
    drift = [n for n, v in asm.items()
             if hasattr(sp, n) and getattr(sp, n) != v]
    if drift:
        raise SystemExit(f"DRIFT packer-vs-asm: {drift}")
    # Completeness: every spec entry has a curated entry and vice versa.
    missing = set(sp.OPCODE_SPECS) ^ set(cur.OPCODES)
    if missing:
        raise SystemExit(f"curated/spec mismatch: {sorted(missing)}")
    git = subprocess.run(["git", "rev-parse", "--short", "HEAD"],
                         capture_output=True, text=True, cwd=HERE).stdout.strip()
    opcodes = []
    for name, spec in sorted(sp.OPCODE_SPECS.items()):
        c = cur.OPCODES[name]
        ops = [dict(attr=a, min=lo, max=hi,
                    unit=(c["operand_units"][i] if i < len(c["operand_units"]) else None))
               for i, (a, lo, hi) in enumerate(spec["ops"])]
        opcodes.append(dict(
            name=name, opcode=spec["opcode"], tickModel=spec["tick"],
            legalRoutes=_routes_list(spec["routes"]), operands=ops,
            special=spec.get("special"), desc=c["desc"],
            feature=c.get("feature"), notes=c.get("notes")))
    return dict(
        meta=dict(formatVersion=cur.FORMAT_VERSION, driverCompat=cur.DRIVER_COMPAT,
                  sourceCommit=git, generator="gen_capability_manifest.py"),
        channels=cur.CHANNELS, fm6Modes=cur.FM6_MODES,
        noteRange=dict(min=0, max=sp.MAX_PITCH),
        opcodes=opcodes,
        guards=dict(regwrite=sp.REGWRITE_GUARD, regdelta=sp.REGDELTA_GUARD,
                    pitchenv=sp.PITCHENV_GUARD),
        musicIllegalOpcodes=sorted(sp._MUSIC_ILLEGAL_OPCODES),
        extRegistry={"0": "COMM", "1": "PUMPSET", "2": "GHOSTSET"},
        macroTags=dict(NEXT=sp.TAG_MAC_NEXT, REG=sp.TAG_MAC_REG,
                       LOOP=sp.TAG_MAC_LOOP, END=sp.TAG_MAC_END),
        features=cur.FEATURES, limits=cur.LIMITS, rules=cur.RULES)

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out"); ap.add_argument("--check")
    a = ap.parse_args()
    text = json.dumps(build_manifest(), indent=2, sort_keys=True) + "\n"
    if a.check:
        with open(a.check) as f:
            if f.read() != text:
                print("MANIFEST DRIFT: regenerate with --out", file=sys.stderr)
                sys.exit(1)
        print("manifest up to date")
    else:
        out = a.out or os.path.join(HERE, "..", "games", "sonic4", "data",
                                    "generated", "memra-manifest.json")
        os.makedirs(os.path.dirname(out), exist_ok=True)
        with open(out, "w") as f:
            f.write(text)
        print(f"wrote {out}")

if __name__ == "__main__":
    main()
```

CAVEAT for the executor: `meta.sourceCommit` makes `--check` fail after ANY new commit. Fix as part of this step: exclude `meta.sourceCommit` from the `--check` comparison (compare parsed JSON with that key popped from both sides, not raw text). Implement that — do not ship the naive text compare.

Note on guard serialization: `REGDELTA_GUARD["reg_sel_max_group"]=None` in Task 3 — set it properly there to `REGDELTA_GROUP_COUNT` once that constant is in scope (the generator serializes whatever the packer table holds; None is a bug to fix in Task 3's placement step).

- [ ] **Step 2: Write the test**

`tools/test_gen_capability_manifest.py`:

```python
import json, os, subprocess, sys, unittest
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_capability_manifest as gen
import song_packer as sp

class TestManifest(unittest.TestCase):
    def setUp(self):
        self.m = gen.build_manifest()

    def test_opcode_byte_space_covered(self):
        # Every command opcode $E0-$FF is a spec opcode, a macro tag namespace
        # overlap, the reserved MEV_EXT, or explicitly music-illegal.
        assigned = {o["opcode"] for o in self.m["opcodes"] if o["opcode"] is not None}
        assigned |= set(self.m["musicIllegalOpcodes"]) | {0xFA}
        missing = [hex(b) for b in range(0xE0, 0x100) if b not in assigned]
        self.assertEqual(missing, [], f"unassigned command bytes: {missing}")

    def test_every_reserved_feature_has_package(self):
        for fid, f in self.m["features"].items():
            if f["status"] == "reserved":
                self.assertIsNotNone(f["enginePackage"], fid)

    def test_routes_are_valid(self):
        for o in self.m["opcodes"]:
            for r in o["legalRoutes"]:
                self.assertTrue(0 <= r < sp.CHROUTE_COUNT)

if __name__ == "__main__":
    unittest.main()
```

- [ ] **Step 3: Run test — expect a REAL failure first**

Run: `python3 -m pytest tools/test_gen_capability_manifest.py -q`
Expected on first run: `test_opcode_byte_space_covered` FAILS listing unassigned bytes (e.g. $E5/$E6 repeats ARE assigned via RepeatStart/End; the genuinely unassigned will be any gap — reconcile: every listed byte must be added to the manifest as an explicit entry or the test's allowed sets. Do NOT weaken the test to pass; extend the manifest until the byte space is fully accounted for.)

- [ ] **Step 4: Generate + all tests green**

```bash
python3 tools/gen_capability_manifest.py
python3 -m pytest tools/test_gen_capability_manifest.py tools/test_packer_parity.py tools/test_song_packer.py -q
```

Expected: manifest written; all pass.

- [ ] **Step 5: Commit**

```bash
git add tools/gen_capability_manifest.py tools/test_gen_capability_manifest.py games/sonic4/data/generated/memra-manifest.json
git commit -m "feat(sound): Memra capability manifest generator + generated manifest"
```

---

### Task 6: JSON Schema in empyrean + published manifest copy

**Files:**
- Create: `/home/volence/sonic_hacks/empyrean/contract/schema/memra-manifest.schema.json`
- Create: `/home/volence/sonic_hacks/empyrean/contract/memra-manifest.json` (copy of the generated instance)
- Modify: `/home/volence/sonic_hacks/empyrean/contract/README.md` (add the Memra entry)
- Test: `tools/test_manifest_schema.py` (aeon; skip-if-no-jsonschema)

- [ ] **Step 1: Write the schema**

Draft 2020-12; strict where the shape is fixed, permissive where the overlay grows (features map = additionalProperties with a fixed value shape). Key excerpts the executor must implement in full (top-level `required`: meta, channels, opcodes, features, limits, rules, guards, macroTags, extRegistry, musicIllegalOpcodes, fm6Modes, noteRange):

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "https://empyrean.dev/contract/memra-manifest.schema.json",
  "title": "Memra capability manifest",
  "type": "object",
  "required": ["meta", "channels", "opcodes", "features", "limits", "rules",
               "guards", "macroTags", "extRegistry", "musicIllegalOpcodes",
               "fm6Modes", "noteRange"],
  "properties": {
    "meta": {
      "type": "object",
      "required": ["formatVersion", "driverCompat", "sourceCommit"],
      "properties": {
        "formatVersion": {"type": "integer", "minimum": 1},
        "driverCompat": {
          "type": "object", "required": ["major", "minor"],
          "properties": {"major": {"type": "integer", "minimum": 0},
                          "minor": {"type": "integer", "minimum": 0}}}
      }
    },
    "opcodes": {
      "type": "array", "minItems": 1,
      "items": {
        "type": "object",
        "required": ["name", "opcode", "tickModel", "legalRoutes", "operands", "desc"],
        "properties": {
          "opcode": {"type": ["integer", "null"], "minimum": 128, "maximum": 255},
          "tickModel": {"enum": ["advance", "zero"]},
          "legalRoutes": {"type": "array",
                           "items": {"type": "integer", "minimum": 0, "maximum": 10}},
          "operands": {"type": "array", "items": {
            "type": "object", "required": ["attr", "min", "max"],
            "properties": {"min": {"type": "integer"}, "max": {"type": "integer"},
                            "unit": {"type": ["string", "null"]}}}},
          "feature": {"type": ["string", "null"], "pattern": "^memra\\."}
        }
      }
    },
    "features": {
      "type": "object",
      "additionalProperties": {
        "type": "object", "required": ["status", "budgetGated", "desc"],
        "properties": {"status": {"enum": ["shipped", "experimental", "reserved"]},
                        "enginePackage": {"type": ["integer", "null"]},
                        "budgetGated": {"type": "boolean"}}}
    }
  }
}
```

Complete the remaining property schemas (channels/fm6Modes/guards/limits/rules/macroTags/extRegistry/noteRange/musicIllegalOpcodes) to match Task 5's generator output exactly — every key the generator emits must validate; unknown top-level keys rejected (`"additionalProperties": false` at the top level only).

- [ ] **Step 2: Schema-validation test in aeon (skip-if-missing dependency)**

`tools/test_manifest_schema.py`:

```python
import json, os, sys, unittest
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_capability_manifest as gen
SCHEMA = "/home/volence/sonic_hacks/empyrean/contract/schema/memra-manifest.schema.json"

class TestSchema(unittest.TestCase):
    def test_instance_validates(self):
        try:
            import jsonschema
        except ImportError:
            self.skipTest("jsonschema not installed")
        if not os.path.exists(SCHEMA):
            self.skipTest("empyrean checkout not present")
        with open(SCHEMA) as f:
            schema = json.load(f)
        jsonschema.Draft202012Validator(schema).validate(gen.build_manifest())

if __name__ == "__main__":
    unittest.main()
```

Run: `python3 -m pytest tools/test_manifest_schema.py -q` — expected: 1 passed (or 1 skipped if jsonschema missing; try `pip install --user jsonschema` first and only accept the skip if the environment refuses).

- [ ] **Step 3: Publish to empyrean + commit both repos**

```bash
cp /home/volence/sonic_hacks/aeon/games/sonic4/data/generated/memra-manifest.json /home/volence/sonic_hacks/empyrean/contract/memra-manifest.json
cd /home/volence/sonic_hacks/empyrean
# README: add under the contract listing —
#   "- memra-manifest.json (+ schema/memra-manifest.schema.json) — the Memra
#    sound-driver capability manifest: opcodes, channel classes, validity
#    rules, feature flags. Generated in aeon (tools/gen_capability_manifest.py);
#    this copy is the released artifact Seraph pins. Same-major = compatible."
git add contract/schema/memra-manifest.schema.json contract/memra-manifest.json contract/README.md
git commit -m "contract: Memra capability manifest v1 (schema + first released instance)"
cd /home/volence/sonic_hacks/aeon
git add tools/test_manifest_schema.py
git commit -m "test(sound): manifest validates against the empyrean schema"
```

---

### Task 7: Build-time drift check

**Files:**
- Create: `tools/validate_manifest_drift.py`
- Modify: `build.sh` (insert after the `sfx_transcode.py generate` line, currently line ~146)

- [ ] **Step 1: Write the drift validator**

```python
#!/usr/bin/env python3
"""Build-time Memra manifest drift check (S0).
1. packer-vs-asm constant sync (gen's cross-check, run always)
2. committed manifest == regenerated manifest (ignoring meta.sourceCommit)
3. --diff-release <path>: breaking-class diff vs the released empyrean copy.
Exit nonzero on any failure — build.sh treats that as a build error."""
import json, os, sys
HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import gen_capability_manifest as gen

COMMITTED = os.path.join(HERE, "..", "games", "sonic4", "data", "generated",
                         "memra-manifest.json")

def _norm(m):
    m = json.loads(json.dumps(m))          # deep copy
    m.get("meta", {}).pop("sourceCommit", None)
    return m

def check_committed():
    fresh = _norm(gen.build_manifest())
    with open(COMMITTED) as f:
        disk = _norm(json.load(f))
    if fresh != disk:
        print("MANIFEST DRIFT: committed memra-manifest.json is stale.\n"
              "  regenerate: python3 tools/gen_capability_manifest.py", file=sys.stderr)
        return False
    return True

BREAKING = []
def diff_release(path):
    """Breaking classes (spec §4.5): opcode byte reuse/removal, operand range
    narrowing, route-legality removal, feature status regression, limit
    tightening. Additions are always fine."""
    with open(path) as f:
        old = _norm(json.load(f))
    new = _norm(gen.build_manifest())
    ob = {o["name"]: o for o in old["opcodes"]}
    nb = {o["name"]: o for o in new["opcodes"]}
    for name, o in ob.items():
        n = nb.get(name)
        if n is None:
            BREAKING.append(f"opcode removed: {name}"); continue
        if o["opcode"] != n["opcode"]:
            BREAKING.append(f"opcode byte changed: {name} {o['opcode']}->{n['opcode']}")
        if set(o["legalRoutes"]) - set(n["legalRoutes"]):
            BREAKING.append(f"route legality removed: {name}")
        for oo, no in zip(o["operands"], n["operands"]):
            if no["min"] > oo["min"] or no["max"] < oo["max"]:
                BREAKING.append(f"operand range narrowed: {name}.{oo['attr']}")
        if len(n["operands"]) != len(o["operands"]):
            BREAKING.append(f"operand count changed: {name}")
    byte_owner = {}
    for o in nb.values():
        if o["opcode"] is not None:
            if o["opcode"] in byte_owner:
                BREAKING.append(f"opcode byte reused: {o['opcode']:#x}")
            byte_owner[o["opcode"]] = o["name"]
    for fid, f in old["features"].items():
        nf = new["features"].get(fid)
        if nf is None or (f["status"] == "shipped" and nf["status"] != "shipped"):
            BREAKING.append(f"feature regressed: {fid}")
    if old["meta"]["driverCompat"]["major"] == new["meta"]["driverCompat"]["major"] \
            and BREAKING:
        print("BREAKING contract changes without a major bump:", file=sys.stderr)
        for b in BREAKING:
            print(f"  - {b}", file=sys.stderr)
        return False
    return True

if __name__ == "__main__":
    ok = check_committed()
    if "--diff-release" in sys.argv:
        ok = diff_release(sys.argv[sys.argv.index("--diff-release") + 1]) and ok
    sys.exit(0 if ok else 1)
```

- [ ] **Step 2: Hook into build.sh**

After the `sfx_transcode.py generate` invocation (line ~146), add:

```bash
python3 "${TOOLS}/validate_manifest_drift.py" || {
    echo "Memra manifest drift — see message above."; exit 1; }
```

Match the surrounding style (the file uses `python3 "${TOOLS}/…"` invocations that abort on failure — mirror exactly how `ojz_block_gen.py`'s failure is handled at line ~140).

- [ ] **Step 3: Negative test — drift MUST fail the build**

```bash
# mutate a range, expect failure, restore:
sed -i 's/("vol", 0, 127)/("vol", 0, 126)/' tools/song_packer.py
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh; echo "exit=$?"     # expect exit=1 + drift message
git checkout tools/song_packer.py
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh && echo BUILD_OK    # expect BUILD_OK
```

Also confirm the release-diff catches a narrowing: with the same temporary mutation, `python3 tools/validate_manifest_drift.py --diff-release /home/volence/sonic_hacks/empyrean/contract/memra-manifest.json` must exit 1 listing `operand range narrowed: Vol.vol`. Restore afterward.

- [ ] **Step 4: ROM hash still golden, then commit**

`sha256sum s4.bin` — must equal Task 1's hash.

```bash
git add tools/validate_manifest_drift.py build.sh
git commit -m "build(sound): Memra manifest drift check wired into build.sh (+ release breaking-diff mode)"
```

---

### Task 8: Conformance exercise song + oracle smoke (FOREGROUND)

**Files:**
- Create: `tools/gen_exercise_song.py`

- [ ] **Step 1: Write the exercise-song generator**

A script that builds, via `song_packer` API, ONE maximal-legal song touching every shipped opcode on a legal route: FM1 lane (Patch, Vol, Pan, OpBias, Detune, Porta, ModSet, NoteFill, PitchEnv, RegDelta, RegWrite, FmEnv, Macro w/ MacReg+MacNext+MacLoop→MacEnd, NoteRaw, Note, NoteDur, RepeatStart/End, Tempo, Lfo), PSG1 lane (Vol, PsgEnv, Detune, Note), PSGN lane (Vol, PsgNoise, Note), DAC lane (Dac id 1), LoopPoint+Jump ending. Respect init-order (Patch+Vol / Vol first) and repeat rules. Emit via `emit_asm()` to `/tmp/s0_exercise_song.asm` and print the packed byte count. (Constructor signatures: see `tools/test_song_packer.py` imports and usage — mirror them; `pack_song(SongDesc(...))`.)

Run: `python3 tools/gen_exercise_song.py`
Expected: prints a byte count, no PackError. This proves the refactored validator accepts a full-surface legal song.

- [ ] **Step 2 (FOREGROUND — controller session only): oracle no-hang smoke**

Wire the exercise song into a DEBUG build in place of an existing song ONLY if a trivial hook exists (`games/sonic4/data/sound/` song includes — check how `song_hcz2.py` output is included). If wiring requires engine-side table edits beyond swapping an include path, SKIP the in-ROM step (S0 promises no engine changes) and instead run the packer-level check only, recording the deferral in the queue doc log. If wired: build, `pgrep -x oracle_gui` (kill stale + launch fresh, ONE instance), reload ROM, play the song via the debug hotkeys (requires `SOUND_DEBUG_HOTKEYS=1 DEBUG=1 SOUND_DRIVER_ENABLED=1`), confirm via `emulator_z80_read` that `STAT_TICK` ($1F13, 0x-prefixed addresses) keeps advancing for 10+ seconds (no Z80 hang) and `STAT_ALIVE` ($1F10) reads $5A. Restore the original song include afterward; the ROM shipped from this plan is byte-identical to golden.

- [ ] **Step 3: Commit**

```bash
git add tools/gen_exercise_song.py
git commit -m "test(sound): maximal-legal exercise song generator (manifest conformance smoke)"
```

---

### Task 9: Memra naming pass (docs only — NO symbol renames)

**Files:**
- Modify: `docs/ENGINE_ARCHITECTURE.md` (sound section header)
- Modify: `CLAUDE.md` (engine summary line)
- Modify: `/home/volence/sonic_hacks/empyrean/docs/ROADMAP.md`

- [ ] **Step 1: aeon edits**

In `docs/ENGINE_ARCHITECTURE.md`, at the sound-driver section heading, add one sentence: *"The driver is named **Memra** (docs/contract/UI-level name, 2026-07-03); `MEV_*` = 'Memra EVent'. Its machine-readable contract is `games/sonic4/data/generated/memra-manifest.json`, published to `empyrean/contract/`."* In `CLAUDE.md`, amend the "From-scratch custom Z80-autonomous sound driver" bullet to "From-scratch custom Z80-autonomous sound driver (**Memra**)".

- [ ] **Step 2: empyrean ROADMAP mention**

Add a line under the suite components: *"Memra — the Aeon sound driver (name is docs/contract-level; contract at `contract/memra-manifest.json`)."*

- [ ] **Step 3: Commit (both repos, exact paths)**

```bash
cd /home/volence/sonic_hacks/aeon && git add docs/ENGINE_ARCHITECTURE.md CLAUDE.md && git commit -m "docs: name the sound driver Memra (docs-level; MEV = Memra EVent)"
cd /home/volence/sonic_hacks/empyrean && git add docs/ROADMAP.md && git commit -m "docs: Memra in the suite roster"
```

---

### Task 10: Merge + queue-doc closeout

- [ ] **Step 1: Full test + build gate on the branch**

```bash
cd /home/volence/sonic_hacks/aeon
python3 -m pytest tools/ -q            # entire tools suite
SOUND_DRIVER_ENABLED=1 DEBUG=1 ./build.sh && sha256sum s4.bin   # golden hash
./build.sh                              # plain build must also stay green
```

Expected: all tests pass; sound-build hash equals Task 1's; plain build succeeds (drift check must not require sound flags — verify it runs identically in both build modes; it must, since it only imports Python tools).

- [ ] **Step 2: Merge to master**

```bash
git checkout master && git merge --no-ff feat/s0-memra-contract -m "merge: S0 Memra contract (manifest + drift check + naming)"
```

- [ ] **Step 3: Update the Seraph queue doc**

In `/home/volence/sonic_hacks/seraph/docs/superpowers/2026-07-03-seraph-banking-queue.md`: set S0's status to **DONE** with the merge commit hash, and append a Log line (date, what shipped, any deferrals recorded — e.g. Task 8 Step 2 skip). Commit:

```bash
cd /home/volence/sonic_hacks/seraph && git add docs/superpowers/2026-07-03-seraph-banking-queue.md && git commit -m "docs: queue — S0 executed (Memra contract shipped)"
```

---

## Self-review notes (spec coverage)

- Spec §1 manifest shape → Tasks 4/5 (all sections emitted) + Task 6 schema.
- Spec §2 generation split + drift → Tasks 3/4/5/7; packer-vs-asm sync = generator cross-check + drift runner.
- Spec §3 packer refactor guarded → Task 2 (parity) + Task 3 Step 5 / Task 7 Step 4 / Task 10 Step 1 (ROM identity).
- Spec §4 versioning → curated FORMAT_VERSION/DRIVER_COMPAT + Task 7 breaking-class diff; header compat byte DEFERRED (pinned above).
- Spec §5 consumption → deliberately NOT here (S1); schema published for it.
- Spec §6 naming → Task 9. Spec §8 verification → Tasks 2/3/5/6/7/8; conformance smoke has an explicit no-engine-change escape hatch.
