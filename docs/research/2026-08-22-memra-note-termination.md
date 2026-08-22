# Memra note termination vs. Seraph's preview — grounding report

**Date:** 2026-08-22 · **Status:** research, read-only. No behaviour changed.

## Grounding record

| repo | ref | SHA |
|---|---|---|
| aeon | `origin/master` (aeon's default branch is **master**, not `main` — `git fetch origin main` fails with `couldn't find remote ref main`) | `1ee8f8e68d826b18023639ab32a8f7c82f238e62` |
| empyrean | `origin/main` | `8f3fbf1cb61c8305705224c284d9f543a853feb8` |
| seraph | this worktree, branch `research/memra-note-termination`, parent | `ede8990b2222bddf99ca15058c0838a0b01d0c24` |

Every aeon and empyrean line number below is from `git show <sha>:<path>` at those
SHAs — never from a working tree. Nothing was read from a dirty tree, so there are no
UNCOMMITTED claims in this report. Seraph citations are from this worktree's committed
`HEAD` (`ede8990b`).

No emulator was touched. No build was run.

---

## 1. What the driver does when a note's duration expires while a later note sounds

**It cannot happen. The state has no representation, because the driver has no note
identity and no per-note timer — it has a per-CHANNEL wait counter.**

`Sequencer_Channel` is the whole of the per-tick model
(`aeon/engine/sound/sound_sequencer.emp:1010-1015`):

```
pub proc Sequencer_Channel () clobbers(af, bc, de, hl) falls_into Sequencer_NextOpcode {
    dec     (ix+sc_dur_count)
    ret     nz                      // note still holding -> no work this tick
    // duration expired -> fetch the next time-advancing event
}
```

`sc_dur_count` is a **single byte per channel** (`sound_constants.emp:694`, "ticks
remaining on the current note/rest"), inside a single 60-byte `SeqChannel`
(`sound_constants.emp:738`, `SeqChannel_len == 60`). There are exactly
`CHROUTE_COUNT == 11` such structs (`sound_constants.emp:465-468`, `:912-913`), one per
physical voice. `sc_note` is likewise one byte (`sound_constants.emp:699`).

So the channel's state is: *one* pitch, *one* countdown, *one* keyed bit. When
`sc_dur_count` hits 0 the driver does not "end a note" — it ends a **wait** and fetches
the next opcode (`sound_sequencer.emp:1023-1104`). Whether that opcode keys off, keys on,
or does neither is what the stream says.

Consequences for the seraph scenario (A: tick 0 len 480, B: tick 240 len 480, one channel):

- There is no byte sequence that means "the note that started at tick 0 has now ended".
  Termination is always *the channel goes quiet* or *the channel gets a new note* — never
  *this particular note stops*.
- Once B keys on at tick 240, A is already gone (see §3). Nothing at tick 480 refers to A,
  because A no longer exists as state — `sc_note` and `sc_base_freq` were overwritten.
- The **only** thing the format could emit at tick 480 is `MEV_REST` ($80), and that would
  silence **B** — i.e. the ROM would truncate B exactly as seraph's preview does. So the
  divergence is not "the driver physically cannot do what seraph does"; it is that no
  monophonic serialization of this input has any reason to emit that rest, and the
  serialization the driver's own semantics imply does not (§Verdict).

This is the honest form of the "not representable" claim: **the seraph event
`NoteOff{tick: 480, pitch: A}` has no counterpart, because the driver's key-off carries no
pitch and no note identity at all.** The concept, not the timing, is what fails to
translate.

Independent corroboration from aeon's own comparative study (a doc, not source):
`aeon/docs/research/2026-08-07-mdsdrv/format-toolchain.md` §2 — *"Single running duration
… No subroutine opcode; no tie; no slur; no transpose opcode."*

---

## 2. How duration and termination are represented

### The opcode space (one flat byte stream per channel)

Range dispatch, `sound_sequencer.emp:1029-1037`:

| bytes | meaning | time |
|---|---|---|
| `$00-$7F` | `.set_dur` — store as `sc_dur_default` (`:1070-1072`) | **zero-tick** |
| `$80` | `MEV_REST` — key OFF (`:1056-1067`) | advances `sc_dur_default` ticks |
| `$81-$DF` | note; pitch = opcode − `MEV_NOTE_BASE` (`:1040-1053`) | advances `sc_dur_default` ticks |
| `$E0-$FF` | coordination, via the 32-entry `SeqOpcodeTable` (`:1079-1100`) | per handler |

Opcode names: `sound_constants.emp:279-319`. The jump table:
`aeon/engine/sound/seq_opcode_tab.emp:43-84`.

**There is no note-off opcode in the table.** All 32 cells are accounted for
(`seq_opcode_tab.emp:44-83`); six are `Seq_BadOpcode` reserves. The only key-off *event*
in the entire grammar is `MEV_REST` ($80), which lives outside the table in the range
ladder.

### The four time-advancing events (all of them key ON or key OFF, none is a bare off)

- `.note` `$81-$DF` — `sound_sequencer.emp:1040-1053`: `set SCF_KEYED`, reload
  `sc_dur_count` from `sc_dur_default`, `call Seq_HookNoteOn`.
- `MEV_REST` `$80` — `:1056-1067`: `res SCF_KEYED`, reload duration, `call Seq_HookNoteOff`.
- `MEV_NOTE_DUR` `$E3` `+ nn dd` — `Seq_Op_NoteDur`, `:1353-1374`: note with an **explicit**
  duration byte. Bit 7 of the pitch operand is `smpsNoAttack` (`:1364-1367`): duration
  counts, no re-key — the driver's only "tie".
- `MEV_NOTE_RAW` `$E7` `+ a4 a0 dd` (`:1379-1410`) and `MEV_PITCHENV` `$E8` (`:1420-1466`,
  paced like a bare note at `:1448-1449`).

### Termination producers (my own enumeration — see §Enumeration below)

Not one of them is a standalone note-off event. Termination is: an explicit `MEV_REST`;
the arrival of the next note-on (last-note priority, §3); a per-frame note-fill countdown;
a PSG vol-env `$83` contour byte; or one of several **chip-level** silencings that bypass
the note abstraction entirely.

### What `MEV_END` does NOT do

`Seq_Op_End` ($FF), `sound_sequencer.emp:1693-1701`, clears `SCF_ACTIVE` and returns. It
does **not** key off. A stream that ends while keyed leaves the note ringing until
something else silences the chip. Preview-relevant: seraph should not assume end-of-song
implies silence.

---

## 3. New note-on with no intervening key-off

**The briefing's premise is inverted for this driver.** The generic YM2612 fact ("key-on
without key-off = no re-attack, envelope continues") does not apply, because Memra keys off
first at its single chokepoint.

`sound_fm.emp:1092-1108`, `Fm_NoteOnFreqExact.do_keyon`:

```
.do_keyon:
    // --- EG RETRIGGER (spec B): the $28 key-on is edge-triggered; keying an
    // already-keyed channel is a chip NO-OP. Key OFF first so EVERY producer
    // funneling through this single chokepoint gets a true 0->1 EG edge.
    bit     SCF_KEYED_B, (ix+sc_flags)
    call    nz, Fm_NoteOff           // keyed -> key OFF first (fresh EG edge)
    ...
    or      SND_FM_KEYON_OPMASK      // $F0 | chsel (all 4 ops on)
```

Note that the gate is `SCF_KEYED`, **not** pitch equality — a same-pitch repeat re-attacks
too, and the header at `sound_sequencer.emp:669-670` says so explicitly ("even a
same-pitch repeat: the real Zyrinx driver re-keys on EVERY pitch command").

Every FM key-on funnels here: `Fm_NoteOn` → `Fm_NoteOnFreq` → `Fm_NoteOnFreqExact`
(`sound_fm.emp:981-1037`, three `falls_into` links), and `Seq_HookNoteOn` tail-jumps
`Fm_NoteOn` (`sound_sequencer.emp:1869-1870`).

**One documented exception** (`sound_fm.emp:1079-1091`): on `CHROUTE_FM6` while
`SND_STAT_DAC_ACTIVE` is set, the chip `$28` key-on is **skipped** — only `SCF_KEYED` is
set, then `jp Snd_ParkDac`. The DAC owns ch6's output, so the key-on would only retrigger a
silenced EG. So an FM6 note-on during a drum produces no audible attack at all.

**PSG half — checked independently, as asked.** `Psg_NoteOn` (`sound_psg.emp:267-363`) has
no key-off/key-on edge because the SN76489 has no envelope generator. It re-latches the
divisor (`:330 call Psg_EmitDivisor`), sets `SCF_KEYED` (`:333`), then
`call Psg_EnvCursorReset` (`:334`) — **that** is the PSG re-attack: the software volume
envelope contour restarts from cursor 0. It then re-arms pitch modulation
(`:339 call Mod_ReArm`) and re-emits volume (`:361-362 jp Psg_SetVolume`). Audibly:
instantaneous pitch + amplitude change with no gap, plus an envelope restart.

**Portamento — checked independently.** Both routes branch on `sc_porta_incr != 0` at
note-on. Armed: the note **attacks at the previous slid pitch** (`sc_porta_accum`) and
glides to the new target, not at the new pitch — FM `sound_fm.emp:1057-1072`, PSG
`sound_psg.emp:340-354`. Unarmed: `sc_porta_accum` snaps to target (FM `:1073-1077`, PSG
`:355-359`). Seraph's preview has no equivalent of this on overlap.

**Net for the overlap case:** in the driver, note B keying on at tick 240 terminates A and
re-attacks cleanly. There is no envelope bleed, no double voice, and no residue of A.

---

## 4. Two logical voices sharing one hardware channel

**No, and the format authority refuses to emit it.**

`aeon/tools/song_packer.py:1010-1015`:

```python
routes = [ch.route for ch in song.channels]
if len(set(routes)) != len(routes):
    dupes = sorted({r for r in routes if routes.count(r) > 1})
    raise PackError(
        f"duplicate channel route(s) {dupes} — two streams would fight "
        f"over one chip channel")
```

Nuance worth recording, because it is the difference between "the format forbids it" and
"the packer forbids it": the **byte format could physically express it**. The loader
assigns `SeqChannel` slots in **declaration order**, not by route
(`z80_sound_driver.emp:1503-1516` — `ld ix, SND_SEQ_CHANNELS`, then `ld a, (iy+SHC_ROUTE)`
/ `ld (ix+sc_route), a` per record), and its only guard is the count clamp
`cp CHROUTE_COUNT+1` (`:1473-1476`). So two records both carrying `CHROUTE_FM1` would load
into two independent `SeqChannel`s, both writing FM1 — an unarbitrated race, which is
exactly why the packer rejects it. **The invariant is enforced producer-side, not by the
engine.**

**Slot[1] is not a second voice.** Each channel descriptor carries a `{cmd_ptr, mod_ptr}`
pair (`song_packer.py:14-22`; loader `z80_sound_driver.emp:1522-1541`), so a channel does
have two streams. But `mod_ptr` feeds `MacroTick`, whose entire vocabulary is
`TAG_MAC_NEXT $E0` / `TAG_MAC_REG $E1` / `TAG_MAC_LOOP $E2` / `TAG_MAC_END $E3`
(`sound_constants.emp:326-332`; reader `sound_sequencer.emp:1734-1755`). Register writes
and frame yields only — **no note opcode exists in that grammar.**

**SFX are the same machine, not a parallel one.** `sound_sfx.emp:298` calls the *shared*
`Sequencer_Channel`, and `SfxChannel` is field-offset-locked to `SeqChannel`
(`sound_constants.emp:752-759`). SFX get polyphony by *stealing* a physical voice, never by
sharing one — see the steal paths in §Enumeration rows 15-18.

---

## 5. What the MEV / packer input format lets a future compiler emit

`song_packer.py` is the format authority (its own words, `:996-999`). Constraints a seraph
S1 compiler inherits:

- **One duration register per channel.** `SetDur` ($00-$7F, `:154-163`) sets it; `Note`
  ($81-$DF, `:171-180`) and `Rest` ($80, `:166-168`) consume it; `NoteDur` ($E3,
  `:651-663`) overrides it for one note.
- **The time-advancing set is fixed:** `(Note, Rest, NoteDur, NoteRaw, PitchEnv)` — used
  identically at `:904`, `:910`, `:947`. Everything else is zero-tick.
- **The engine's `smpsNoAttack` tie is packer-unreachable at this SHA.** The engine honours
  bit 7 of the `NOTE_DUR` pitch operand (`sound_sequencer.emp:1364-1367`), but
  `NoteDur.validate` bounds pitch to `MAX_PITCH == 0x5E` (`song_packer.py:660-661`), so bit
  7 can never be set. A compiler wanting ties would need a packer change first.
- **Init order is mandatory.** FM channels need `Patch` ($E1) **and** `Vol` ($E0) before the
  first time-advancing event; PSG needs `Vol` (`:910-932`).
- **Single-level repeats only** (`:886-893`); loop body must advance time (`:954-960`);
  `LoopPoint` may not sit inside a repeat span (`:933-944`); stream must end in `Jump` or
  `End` (`:973-975`).
- **Duplicate routes rejected** (§4).
- **Nothing in the packer, the format, or the engine expresses note overlap.** There is no
  polyphony construct, no voice-stealing construct, and no note identity to attach one to.

**Seraph's own S1 design already ratified the consequence.** From this worktree's committed
`docs/superpowers/specs/2026-07-03-s1-native-model-compiler-design.md:40-42`:

> `Track { route(s): from manifest channel classes, lanes: Vec<Lane> }` — a Lane =
> sub-voice (instrument + regions); **monophony across a track's lanes is a MODEL
> invariant (validated on edit, not just export).**

and `:78` lists "lane monophony" as a model invariant test. So under S1, a channel with
overlapping notes is an **invalid project**, rejected at edit time — the compiler never
faces the ambiguity.

`empyrean` at `8f3fbf1c` adds nothing normative: `wiki/SOUND.md` is a page plan
(`:11-18` is a boundary rule, not a semantic contract) and `docs/handoffs/seraph.md` is
identity/bus guidance. **No cross-tool contract in empyrean specifies note-overlap
semantics.** Recording that as a gap, not filling it.

---

## Enumeration: everything that can stop a sounding note (second frame)

The aeon overseer enumerated by name (`*NoteOff`) and reported **13 sites, 2 of them the
`Seq_HookNoteOff` route dispatcher**. Per instruction I did **not** re-run that search.

**My method (different frame):** enumerate by *state and chip effect* — grep for
`SCF_KEYED`, `sc_note`, `sc_flags`, `SND_REG_KEY_ONOFF`/`SND_FM_KEYON_OPMASK` (the `$28`
writes), `SND_PSG_ATTEN_SILENT`/`SND_PSG_SILENCE_*`/`ld (SND_Z80_PSG), a`, `$B4` pan-gate
writes, and `$2B` DAC-enable — across **all six** sound files plus the debug module:
`sound_sequencer.emp`, `sound_fm.emp`, `sound_psg.emp`, `sound_sfx.emp`, `sound_api.emp`,
`z80_sound_driver.emp`, `engine/debug/sound_debug.emp`.

**Result: 21 sites in 7 mechanism classes.** Only 12 of them are a `call`/`jp` to
`Fm_NoteOff`/`Psg_NoteOff`; **9 stop a sounding note without going near a routine named
"NoteOff"** — which is precisely what a name-based search cannot see.

| # | site | mechanism | touches `SCF_KEYED`? |
|---|---|---|---|
| **A. FM `$28` key-off emitters** ||||
| 1 | `sound_fm.emp:1118-1126` `Fm_NoteOff` | the one named funnel | yes, `res` (`:1124`) |
| 2 | `sound_sequencer.emp:2002-2015` `Seq_SilenceMusicVoices` | **direct** unrolled `$28`×6, bypasses `Fm_NoteOff` | **no** — deliberately left set (`:1992`) |
| 3 | `z80_sound_driver.emp:1059-1068` `Snd_StartSample` | **direct** `$28` chsel 6, adaptive-FM6 pre-key-off | **no** (`:1057`) |
| **B. PSG attenuation-to-silence** ||||
| 4 | `sound_psg.emp:372-386` `Psg_NoteOff` (tone `:377-380`, noise `:382-384`) | named funnel | yes, `res` (`:373`) |
| 5 | `sound_psg.emp:690-698` `Psg_SilenceAll` | four silence latches `$9F/$BF/$DF/$FF` | **no** |
| 6 | `sound_sequencer.emp:1186-1188` `Seq_Op_PsgNoise` ($F2) | writes `$DF` — **silences a sounding ch2 tone as a side effect of arming noise** | **no** |
| 7 | `sound_sfx.emp:1252-1254` `Sfx_Restore` noise path | same `$DF` re-silence of ch2 | **no** |
| 8 | `sound_psg.emp:530-534`, `:600-604` `Psg_SetVolume` clamps | attenuation clamped to `$0F` — a keyed note rendered inaudible by volume/env alone | **no** |
| **C. FM pan-gate hard mute** ||||
| 9 | `sound_sequencer.emp:2016-2036` | `$B4/$B5/$B6 = 0` both parts. Its own header (`:1989-1990`) calls it *"a pop-free HARD output mute that holds through the EG release tail"*, and `:1994-1997` notes **nothing re-opens a gate by itself** | **no** |
| **D. DAC takeover of FM6** ||||
| 10 | `z80_sound_driver.emp:1069-1077` | `$2B = $80` — DAC output *replaces* FM6; any FM6 note goes inaudible with no key-off | **no** |
| 11 | `sound_fm.emp:1084-1091` | mirror gate: FM6 key-on **skipped** while DAC active; only the bit is set | sets, no chip write |
| **E. Per-frame envelope termination (no stream event)** ||||
| 12 | `sound_sequencer.emp:610-618` note-fill | `sc_fill_count` → 0 → `call Fm_NoteOff`. FM-only (inside `.is_fm`) | via #1 |
| 13 | `sound_sequencer.emp:738-743` `PsgEnvUpdate.rest` | `$83` contour byte → `jp Psg_NoteOff` | via #4 |
| 14 | `sound_sequencer.emp:794-799` `FmEnvUpdate.rest` | `$83` → TL-silence `sc_env_out = $7F`, **explicitly not a key-off** (`:751-752`) | **no** |
| **F. SFX steal / restore / stop** ||||
| 15 | `sound_sfx.emp:592-605` `Sfx_MusicKeyOffKeepKeyed` | `Fm_NoteOff` (`:597`) / `Psg_NoteOff` (`:600`), then **re-asserts** `SCF_KEYED` (`:604`) — kills the sounding music note while the logical bit lies | set back |
| 16 | `sound_sfx.emp:1263` | restore, noise route | via #4 |
| 17 | `sound_sfx.emp:1285`, `:1330` | restore with no music note under the steal | via #1/#4 |
| 18 | `sound_sfx.emp:1346`, `:1351` | `Sfx_StopAll` — FM and PSG SFX voices | via #1/#4 |
| **G. Stream / global** ||||
| 19 | `sound_sequencer.emp:1056-1067` `MEV_REST` ($80) | the only key-off **event** in the grammar | yes, `res` (`:1057`) |
| 20 | `sound_fm.emp:1098-1099` | next note-on's implicit off-then-on (§3) | via #1 |
| 21 | `sound_sequencer.emp:332-333` `Fade_Ramp` terminals; `z80_sound_driver.emp:1370` `Snd_LoadSong` | `Sequencer_StopAll` / `Snd_PauseMusic` → #2 + #5 + #9 | no |

**Reconciliation with their 13.** My grep for `call`/`jp` to `Fm_NoteOff`/`Psg_NoteOff` in
all seven files yields exactly **12**: `sound_fm.emp:1099`; `sound_sfx.emp:597, 600, 1263,
1285, 1330, 1346, 1351`; `sound_sequencer.emp:618, 743, 1885, 1888` — of which
`:1885`/`:1888` are the `Seq_HookNoteOff` dispatcher, matching their "2". Their 13th is
almost certainly `sound_sequencer.emp:1066` `call Seq_HookNoteOff` (the `MEV_REST` call
*into* the dispatcher), which my `call Fm_/Psg_NoteOff` regex excludes. **12 + that call
site = 13. The counts agree, and the agreement is not an artefact of a shared method** —
mine started from `SCF_KEYED`/`$28`/attenuation, not from the string "NoteOff".

**Where the frames disagree, and it matters.** A name-based enumeration reports 13 sites
that all take `ix` and none that take a note argument, and concludes note identity is
absent. That conclusion is right, but the enumeration **misses 9 of the 21 ways a sounding
note actually stops** — rows 2, 3, 5, 6, 7, 8, 9, 10, 14. Three of those (2, 3, 9) write
`$28`/`$B4` directly and never call `Fm_NoteOff` at all; one (9) silences through the EG
release tail and is **sticky until a patch reload or unpause re-opens the gate**; one (6) is
an ordinary music opcode, `MEV_PSGNOISE`, that silences a neighbouring channel's tone as a
documented side effect.

Also confirmed by absence: **`sound_api.emp` (558 lines) and `engine/debug/sound_debug.emp`
(110 lines) contain zero chip writes and zero `SCF_KEYED`/`sc_note` touches** — grep for
`SND_Z80_PSG|SND_Z80_YM|NoteOff|StopAll|Silence|$28` across both returns nothing. Neither
can terminate a note. That is a real negative, not an untested assumption.

**Preview-fidelity items this frame surfaces that the name-based one does not:**

- `MEV_PSGNOISE` ($F2) silences PSG tone ch2 (`$DF`) whenever it fires
  (`sound_sequencer.emp:1187-1188`). A seraph preview that models the noise channel
  independently of PSG3/ch2 will diverge here.
- `MEV_END` does **not** silence (`sound_sequencer.emp:1693-1701`) — a keyed note at
  end-of-stream rings on.
- `FmEnvUpdate`'s `$83` is TL-silence, **not** key-off, so the YM EG release still runs
  (`:794-799`). PSG's `$83` *is* a key-off (`:738-743`). The two `$83`s are not symmetric.
- Pan-gate mute (row 9) is not modelled by seraph's `key_off_channel` at all
  (`src-tauri/src/sequencer/mod.rs:539-559` writes `$28` and PSG attenuation only).

---

## Verdict: **DIVERGES**, with the authoritative rule owned by S1 rather than by the sequencer

Not "under-determined". The driver's behaviour on a monophonic channel is fully determined
once the stream is fixed, and the stream that any faithful serialization of "A(0,480),
B(240,480) on one channel" produces contains **no event at tick 480**:

```
SetDur 240 ; Note A          <- keys A, waits 240
SetDur 480 ; Note B          <- Fm_NoteOff then key-on (sound_fm.emp:1098-1099); waits 480
Rest                         <- tick 720
```

Seraph's stream is `On(A,0) On(B,240) Off(A,480) Off(B,720)` and `process_event`
(`src-tauri/src/sequencer/mod.rs:388-391`) matches `NoteOff { .. }` and keys off
unconditionally. **B is cut at 480 instead of 720 — 240 ticks early.** That is the defect.

Two honest caveats that keep this from being a simple "the hardware can't do that":

1. The driver's key-off primitive **is** pitch-blind, exactly like seraph's. `Seq_HookNoteOff`
   (`sound_sequencer.emp:1881-1889`) branches only on route bits; `Fm_NoteOff`
   (`sound_fm.emp:1118-1126`) and `Psg_NoteOff` (`sound_psg.emp:372-386`) read neither
   `sc_note` nor any argument. So `process_event`'s unconditional key-off is *correct*
   modelling of the primitive. The bug is upstream, in `build_snapshot` emitting an event
   that no serialization would emit.
2. A perverse compiler *could* emit `Rest` at tick 480 and the ROM would then truncate B
   too. So the ROM is not physically incapable of the sound seraph produces. What is absent
   is any reason to emit it — and S1's model invariant
   (`docs/superpowers/specs/2026-07-03-s1-native-model-compiler-design.md:40-42`, lane
   monophony) makes the input itself invalid, so the question never reaches the compiler.

**On the pitch-equality guard.** Rejected — and the driver-faithful answer does *not* share
its weakness:

- `active_notes[ch] == Some(pitch)` would make seraph's key-off pitch-aware, which is a
  step **away** from the driver (whose key-off is pitch-blind, and whose re-key gate at
  `sound_fm.emp:1098` is `bit SCF_KEYED_B`, not a pitch compare).
- It fixes nothing when the two overlapping notes share a pitch, as the briefing notes.
- The driver-faithful fix is at **event construction**, not at dispatch, and is therefore
  pitch-independent: the stale `NoteOff` is never created, so its pitch is irrelevant. Two
  overlapping same-pitch notes reduce to `Note P (dur = gap) ; Note P (dur = len)` — a clean
  re-attack, which is what the chip does (`sound_sequencer.emp:669-670`).

---

## Driver-faithful preview behaviour, concretely

Fix in `build_snapshot` (`src-tauri/src/project/manager.rs:903-972`), leaving
`process_event` alone.

Today, every note unconditionally pushes both a `NoteOn` at `abs_tick` and a `NoteOff` at
`end_tick` (`manager.rs:938-952`). Instead, after collecting the channel's note spans,
apply **last-note-priority monophonic reduction** — the same reduction the MEV serializer
must apply:

1. Collect spans `(start_i, end_i)` for the channel (the data is already gathered in
   `overlap_sources`, `manager.rs:953`).
2. Sort by `start`, ties broken however `sort_by` at `:959-972` already does.
3. Emit `NoteOn` for every span, unchanged.
4. Emit `NoteOff{tick: end_i}` **only if** there is no next span, or `end_i < start_{i+1}`.
   When `end_i >= start_{i+1}` the successor's key-on is the terminator — matching
   `sound_fm.emp:1098-1099` — and the stale off must not exist.
   (At exact equality `end_i == start_{i+1}` the off is redundant, not harmful: seraph's
   `process_event` keys off on note-on anyway at `mod.rs:332-334`, and the driver's `.do_keyon`
   does the same. Dropping it is the closer match to what the packer emits — no `Rest`.)
5. Keep the `OverlapWarning`s exactly as they are (`:974-982`). They remain the correct
   authoring signal, and under S1 they become a hard model-invariant violation.

Already correct, no change needed: seraph's `process_event` keys off before key-on when a
note is active (`mod.rs:332-334`), matching `.do_keyon` — including for a same-pitch repeat,
since both gate on "is something sounding", not on pitch.

Verification without an emulator: a unit test over `build_snapshot` asserting that for
A(0,480)+B(240,480) on one channel the event list is exactly
`[On(A,0), On(B,240), Off(720)]`, and that A(0,240)+B(360,240) (a real gap) still yields
four events.

---

## Open / BLOCKED

- **Not established: which reduction S1 will actually implement.** S1's design makes
  overlap a model-invariant *violation*
  (`docs/superpowers/specs/2026-07-03-s1-native-model-compiler-design.md:40-42`) but does
  not say what the compiler does if one slips through. Last-note-priority is the only rule
  consistent with `sound_fm.emp:1098-1099`, but the spec **does not state it**. This is a
  gap to close in the S1 spec, and it is the one place where "under-determined" is the true
  answer.
- **Not established: any cross-tool contract for this in empyrean.** At `8f3fbf1c` there is
  none. `wiki/SOUND.md` and `docs/handoffs/seraph.md` are the only seraph/sound documents
  and neither carries note semantics.
- **Deliberately not done:** no build (`./build.sh`, `SIGIL_*`), no emulator MCP call. The
  claim that a stale `Off` truncates the later note is a source-level reading of
  `mod.rs:388-391` plus `manager.rs:938-952`; if runtime confirmation on real hardware is
  wanted, **TAG: foreground follow-up for the controller** — capture VGM of a packed
  two-overlapping-notes song via `emulator_vgm_start` and compare against the preview. Not
  attempted here.
- **Line-number caveat.** `aeon/docs/research/2026-08-07-mdsdrv/format-toolchain.md` cites
  `sound_sequencer.emp:982-983, 996-997, 1358-1359` for the duration reloads; at
  `1ee8f8e6` those reloads are at `:1046-1047`, `:1060-1061`, `:1360`. That doc was written
  against an earlier SHA. Every line number in *this* report is verified at `1ee8f8e6`.
