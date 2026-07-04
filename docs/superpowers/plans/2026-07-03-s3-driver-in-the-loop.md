# S3 — Driver-in-the-Loop Preview Implementation Plan (incl. spec §)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. Repo: seraph, branch `feat/s3-driver-loop`. Depends: S1 (compiler). FOREGROUND rule for oracle steps.

## Spec § (design)

Seraph embeds a cycle-stepped Z80 core running the REAL Memra driver blob
against its existing Nuked-OPN2 + Rust SN76489; "play" = compile the project
to a MEV blob, load it exactly as the 68k loader would, and let the driver
play it. The preview then IS the driver — tempo gates, envelope frames,
note-fill, macro timing, DAC streaming all real; Z80 overruns audible/visible.

**Core (research 2026-07-03):** vendor **floooh/chips `z80.h`** (zlib, header-
only C, cycle-stepped via `z80_tick(cpu,pins)` — the same vendoring pattern as
Nuked-OPN2). Runner-up if integration stalls: the pure-Rust `z80` crate (MIT,
instruction-stepped, ZEXALL-verified). **The tick loop + Genesis memory map
live in a C shim** (one FFI call per cycle is too slow from Rust): decode MREQ
writes to YM ($4000–$4003), PSG ($7F11), bank reg ($6000), banked-ROM window
reads ($8000+ via a host callback), Z80 RAM ($0000–$1FFF = the blob + driver
state). The shim returns per-batch ring buffers of `(cycle, chip, addr, val)`
events; Rust replays them into the chip cores with cycle-exact timestamps.
No tracker has done this (Furnace/DefleMask/SMPSPlay all reimplement); it's
novel but strictly simpler than a full console emulator — no 68k, no VDP.

**Clocking:** Z80 at 3579545 Hz; audio callback pulls N samples → run
N × (3579545/44100 ≈ 81.2) ticks (fixed-point accumulator). The driver's frame
clock is YM Timer A — which the YM core itself raises; the shim must route the
YM busy/status reads and Timer A overflow flag reads back to the Z80 bus
(the driver polls/gets ticked via the YM status port — verify the exact
mechanism in `aeon/engine/sound/z80_sound_driver.asm` before implementing;
NO vblank INT is needed if the driver is Timer-A-clocked, which it is).

**Blob + song loading:** aeon's build emits the driver blob (even-sized, per
repo memory) — add an aeon build artifact copy step (blob + s4.lst-derived
symbol map for the mailbox/status addresses, generated not hand-coded). Seraph
loads blob at $0000, writes the compiled song via the same mailbox/param
protocol the 68k uses (SND_REQ/param block per the command API spec), sets the
bank window to a host buffer holding the song+samples ("virtual cartridge").

**68k-command simulation:** play/stop/pause/tempo/SFX = mailbox writes with
the documented hold-bus protocol reduced to atomic host writes between ticks.

**Budget meter (real tier):** the shim counts ticks-per-Timer-A-frame the
driver spends outside its idle loop (idle PC range from the symbol map);
report per-frame % to the UI; overrun (frame overshoot) flags red.

**Acceptance:** S2's semantic-gap list closes to empty — A/B (S2) re-run on
the corpus must show side A ≡ side B within capture tolerance, because side A
is now the same driver.

---

### Task 1: Vendor + shim
Files: `src-tauri/vendor/chips/z80.h` (vendored, zlib header retained), `src-tauri/vendor/z80shim/{shim.c,shim.h}`, `src-tauri/build.rs` (cc entries), `src-tauri/src/z80/mod.rs` (FFI).
- [ ] Shim API: `shim_new(rom_cb, ctx)`, `shim_load_ram(addr,buf,len)`, `shim_write_mailbox(addr,val)`, `shim_run(ticks, evbuf) -> n_events`, `shim_read_ram(addr,len)`, idle-PC-range setter for the budget counter. Event = packed `(u32 cycle, u8 chip, u16 addr, u8 val)`.
- [ ] Unit: load a 20-byte test program (copy loop writing YM port), run, assert event sequence + RAM state. Commit.

### Task 2: Chip-core wiring + audio graph
Files: `src-tauri/src/preview/driver_engine.rs` (new), modify `src-tauri/src/audio/` engine to host it as an alternative source alongside the existing sequencer preview.
- [ ] Replay event ring into Nuked-OPN2/SN76489 with cycle→sample-time conversion; YM status/Timer-A read-back path wired (read the driver source FIRST — cite the mechanism in code comments).
- [ ] Golden boot test: load the real blob (path configurable; from the aeon checkout `s4` build artifacts), run 2 emulated seconds, assert STAT_ALIVE==0x5A and STAT_TICK advancing (addresses from the generated symbol map, not literals). Commit.

### Task 3: aeon artifact export
Files: aeon `build.sh` (+ ~5 lines) or `tools/export_driver_artifact.py` (new): copy the assembled Z80 blob + emit `memra-driver-symbols.json` (mailbox/status/idle-range addresses parsed from `s4.lst`) into `games/sonic4/data/generated/`. Seraph pins copies under `src-tauri/assets/` like the manifest.
- [ ] Drift guard: symbols file carries driverCompat; Seraph refuses a blob whose major ≠ manifest major. Commit (aeon).

### Task 4: Compile-and-play + transport
Files: `src-tauri/src/preview/mod.rs`, IPC commands (`preview_play/stop/pause/seek?`), UI transport binding.
- [ ] Play = S1 compile → load via mailbox protocol → run; stop/pause/tempo via mailbox; seek = restart+fast-forward (run ticks with audio muted) — document the latency; SFX audition = SFX-tier command (S6 uses this).
- [ ] Budget meter real tier wired to the UI (S4's meter). Commit.

### Task 5: Validation (FOREGROUND finale)
- [ ] Corpus A/B: S2 pipeline side-A replaced by driver-in-the-loop render; corpus + one real song must PASS; the S2 semantic-gap list must be empty — any survivor is a shim/timing bug.
- [ ] By-ear check by the user (queue-doc gate note). Merge → main; queue S3 → DONE (+log). Commit.
