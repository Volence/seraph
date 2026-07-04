# S2 — Export Verification Gate Implementation Plan (incl. spec §)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. FOREGROUND rule: oracle MCP steps run in the controller session only. Depends: S0+S1 executed. Repos: aeon (harness) + seraph (render export).

## Spec § (the design, in brief)

The permanent A/B gate behind the guarantee until (and alongside) S3: the SAME
project rendered two ways — (A) Seraph's chip emulation (Nuked-OPN2 + Rust
SN76489) rendering the COMPILED MEV blob's intent, and (B) the real Memra
driver playing that blob in oracle, captured as VGM and rendered via vgm2wav —
compared by energy envelope + spectrum correlation (per repo memory: verify
REAL rendered output, never register proxies). Because oracle VGM capture is
realtime-only and FOREGROUND, this is an operator runbook + scripted compare,
not unattended CI: run per-song before a song ships into the game, and on any
compiler/driver change touching the corpus. Thresholds start at the values
proven in the MT/HCZ2 fidelity work (onset alignment via `tools/vgm_onsets.py`,
band-energy correlation ≥0.9) and are tightened empirically.

**Side A precision note:** Seraph's side-A render must be driven by the
COMPILED blob re-decoded through the manifest (not the pre-compile model), so
side A already reflects compiler output; then B-vs-A differences isolate
DRIVER semantics (tempo gate timing, envelope frames, fill) — exactly the gap
S3 closes. Divergences get triaged: compiler bug / Seraph-render semantic gap
(logged, expected until S3) / driver bug (escalate to aeon).

**Goal:** `seraph render --blob` + aeon capture runbook + `tools/ab_compare.py` verdicts.

---

### Task 1: Blob-driven render mode in Seraph (side A)
Files: create `src-tauri/src/preview/blob_render.rs`; a `MevDecoder` reading the compiled blob via manifest opcode table into timed chip-register writes at the driver's ~59.92 Hz tick (S3K tempo accumulator per channel, envelope contours per frame — implement the documented semantics from the author-surface inventory §1/§5; where semantics are ambiguous, log a `SemanticGap` marker into the render metadata instead of guessing silently).
- [ ] CLI/dev command: `npm run render -- <project> --out a.wav --meta a.json` (headless: cpal replaced by an offline writer).
- [ ] Unit test: 4-note score renders non-silent WAV; SemanticGap list empty for plain notes. Commit.

### Task 2: aeon capture runbook + injection hook (side B)
Files: aeon `docs/superpowers/2026-07-03-s2-capture-runbook.md` (new); reuse S0's exercise-song include mechanism for injecting an arbitrary packed `.asm` song into a DEBUG build slot (document the exact include + hotkey; if S0 recorded the skip, first do that wiring here — it is in-scope for S2 and still data-only: a song include + song-table entry).
- [ ] Runbook: build w/ `SOUND_DRIVER_ENABLED=1 DEBUG=1 SOUND_DEBUG_HOTKEYS=1` → ONE oracle instance → play slot → `emulator_vgm_start/stop` (realtime) → `vgm2wav` → `b.wav`. FOREGROUND.
- [ ] Verify once end-to-end with a 4-note corpus score (FOREGROUND). Commit.

### Task 3: The comparator
Files: aeon `tools/ab_compare.py` (new; numpy/scipy already used by sound tools), extending the `vgm_onsets.py` approach.
- [ ] Metrics: (1) onset times within ±1 frame; (2) per-band RMS envelope correlation (4 bands: <200 Hz, 200–1k, 1–4k, >4k) ≥0.9; (3) duration match. Output: PASS/FAIL + per-metric table + a `--report out.md`.
- [ ] Unit test with synthetic WAV pairs (identical → PASS; pitch-shifted → FAIL). Commit.

### Task 4: Corpus + gate policy
- [ ] Wire the S1 parity corpus's "everything" score + first real song through the full A/B once (FOREGROUND); file each divergence as compiler-bug / semantic-gap / driver-bug in the report; semantic-gap list becomes S3's acceptance checklist (commit the report to seraph `docs/superpowers/reports/`).
- [ ] Document the standing policy in the queue doc: A/B required before a song ships into the game; required after any compiler/packer/driver change touching corpus behavior. Update queue S2 → DONE. Commit.
