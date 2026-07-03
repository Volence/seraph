# Memra Author-Surface Inventory — 2026-07-03

**What this is:** the exhaustive inventory of every author-facing feature and
constraint of the Aeon engine's sound driver (**Memra**), extracted from the
driver source and the banked engine specs on 2026-07-03. It is the grounding
data for the Seraph Aeon-profile banking queue
(`specs/2026-07-03-aeon-profile-banking-design.md`) — every S0–S6 spec session
must read it. It reflects the engine AS SPEC'D on this date: engine packages
1–6 are banked but not yet executed; items marked with their package are
normative-but-unshipped, and Tier-2 items are budget-gated (may never ship).

**Sources:** `aeon/sound_constants.asm`, `aeon/engine/sound/*`, and aeon specs:
`2026-06-23-music-expression-engine-design.md` (incl. package-0 validity rules),
`2026-06-27-music-expr-macro-spine-design.md`, `2026-06-16-sound-driver-design.md`,
`2026-06-16-sound-z80-ram-map.md`, `2026-06-16-sound-command-api.md`,
`2026-07-03-sound-production-suite-design.md`, `2026-07-03-sound-game-feel-moments-design.md`,
`2026-06-24-dac-drum-format-revision-design.md`.

---

## 1. Core timing & execution model

- **Frame clock:** YM2612 Timer A, reload N=137 (pinned, build-asserted) →
  ~59.92 Hz tick, ~16.67 ms period. No per-song reload.
- **Tick model (S3K TempoWait):** per-channel accumulator
  `sc_tempo_accum += tempo_mod` each frame; CARRY = skip this channel's event
  tick. Rate = `(256 − mod)/256` event-ticks per frame; `mod=0` full speed.
  Base from song header; overridable by `MEV_TEMPO`.
- **Dual stream per channel:** slot[0] = note/command events
  (`Sequencer_Channel`); slot[1] = macro/automation stream (`MacroTick`).
  Frame order: named-contour render (`ModUpdate`) → slot[1] reg-writes →
  slot[0] opcodes.
- **Time-advancing events (consume an event-tick):** `MEV_REST` ($80), notes
  ($81–$DF), `MEV_NOTE_DUR` ($E3), `MEV_NOTE_RAW` ($E7), `MEV_PITCHENV` ($E8).
  **Everything else is zero-tick** (applies immediately within the frame).
- **Per-channel state:** 60 B `SeqChannel` struct (11 channels, $1A08–$1C9B).

## 2. Channel roster & hardware topology

11 sequencer lanes (`CHROUTE_COUNT`, sound_constants.asm:715–731):

| Route | Symbol | Physical | Notes |
|---|---|---|---|
| 0 | FM1 | YM part I ch0 | **Never SFX-stolen** (lead-safe) |
| 1 | FM2 | YM part I ch1 | **Never SFX-stolen** (bass-safe) |
| 2 | FM3 | YM part I ch2 | SFX-stealable; special mode = Tier-2 door (see §9) |
| 3 | FM4 | YM part II ch0 | SFX-stealable |
| 4 | FM5 | YM part II ch1 | SFX-stealable |
| 5 | FM6 | YM part II ch2 / DAC | Adaptive slot (see below) |
| 6–8 | PSG1–3 | SN76489 tone 0–2 | SFX-stealable; PSG3 is the noise clock source |
| 9 | PSGN | SN76489 noise | SFX-stealable |
| 10 | DAC | YM $2A | Trigger-only lane; never stolen |

**FM6 ↔ DAC (hardware mutual exclusion via $2B bit 7), three song-declared modes:**

| Mode | Flags | Behavior |
|---|---|---|
| Dedicate | `SH_F_FM6_FM=0` | DAC owns ch6 all frame; FM6 music notes cannot play |
| Full FM6 | `SH_F_FM6_FM=1, ADAPTIVE=0` | FM6 plays normally; DAC triggers forbidden |
| Time-share | both set | FM6 plays in gaps; each drum key-offs FM6, re-keys at sample exhaust (Echo-style) |

**PSG noise coupling:** rate-3 noise mode clocks from PSG3's tone divisor
(`MEV_PSGNOISE` ctrl $E0–$EF). When coupled, PSG3's tone is SILENCED (atten $F)
— noise carries the pitch, PSG3's lane is unavailable. SFX noise restore does
NOT re-latch PSG3 tone (by design).

**SFX steal/restore:** stealable pool = FM3–5, PSG1–3, PSGN (7 voices). Steal
sets `SCF_SFX_OVERRIDE` on the music channel (music writes suppressed, state
still advances); restore re-keys if `SCF_KEYED`. Priority ≥$C0 SFX also duck
music FM carriers (ramped). Composer guidance: leads/bass on FM1/FM2; FM6
unavailable-or-interrupted per mode.

## 3. Pitch model

- **Note range:** pitch index 0..$5E (95 semitones) as opcodes $81–$DF;
  132-entry 2-page chromatic pitch table (shared FM+PSG), per-song override
  table possible (header offset, 0=default).
- **`MEV_NOTE_RAW` ($E7, FM-only):** exact $A4/$A0 register bytes + duration
  1..255 — sub-C0 bass, microtuning. Duration 0 illegal.
- **`MEV_DETUNE` ($F6):** signed −128..127 fine offset, latched; FM
  block-corrected at note-on, PSG divisor add.
- **`MEV_PORTA` ($F5):** 0=off, 1..255 = fnum/divisor units per frame; linear
  glide with automatic block-boundary correction (fnum >$0508 → halve+block++;
  <$0284 → double+block−−). MUST follow a prior note (exporter responsibility).
- **`MEV_MODSET` ($EC):** software vibrato — onset-delay, speed, step-count;
  per-channel independent phase; FM+PSG. All-zero = off.
- **`MEV_PITCHENV` ($E8, FM-only):** 1–5 point pitch contour (fnum-table
  indices 0..$83), cycles per frame; arms AND keys in one opcode
  (time-advancing).
- Vibrato + portamento coexist additively on `sc_mod_accum`; pitch-envelope vs
  vibrato arbitration is a reserved phase-3 door.

## 4. Duration & articulation

- **Default duration:** bytes $00–$7F set `sc_dur_default` (operandless).
- **`MEV_NOTE_DUR` ($E3):** explicit pitch + duration (0..255).
- **`MEV_NOTEFILL` ($ED, FM-only):** per-channel gate — 0=legato/off, 1..255 =
  frames keyed before early key-off (tracker-style gate time).
- **`MEV_REST` ($80):** key-off + advance.
- **Repeats:** `MEV_REPEAT_START` ($E5) / `MEV_REPEAT_END` ($E6, count 1..255;
  0 ILLEGAL — runs 255). **Single-level only, no nesting.** Body must contain
  ≥1 time-advancing event. Sequential blocks re-arm.
- **Loop/end:** `MEV_LOOP_POINT` ($EE) + `MEV_JUMP` ($EF) for looping songs;
  `MEV_END` ($FF) idles the channel (all channels ended → song-finished
  status). Last stream opcode must be $EF or $FF.

## 5. Volume & timbre

- **`MEV_VOL` ($E0):** linear 0..127. FM → carrier TL fold; PSG →
  `atten = ($7F−vol)>>3` (PSG atten already ~2 dB/step).
- **`MEV_PATCH` ($E1, FM-only):** patch index into song patch table; full
  26-reg load at next key-on. Init rule: FM channels need Patch AND Vol before
  first time-advancing event; PSG needs Vol; DAC exempt.
- **`MEV_OPBIAS` ($E9, FM-only):** per-operator additive TL bias — op 0..3
  (physical S1/S3/S2/S4) + signed val (negative = brighter); applied at patch
  load, not per-frame.
- **`MEV_REGDELTA` ($EA, FM-only):** mid-note minimal register writes — count
  1..255 × (reg_sel, value); reg_sel = (group<<2)|op, groups 0–5 = $30/$40/$50/
  $60/$70/$80. Immediate write; does NOT re-key, NOT a pitch change. THE
  mid-note timbre-morph primitive (patch swap re-keys; this doesn't).
- **Volume envelopes (named contour slot, unified `sc_env`):**
  `MEV_PSGENV` ($EB, PSG) / `MEV_FMENV` ($F7, FM carrier-TL) — 1-based env id
  (0=off) into shared global contour tables (`gen_sound_tables.py`); body =
  per-frame absolute bytes + control codes $80=loop, $81=sustain, $83=end;
  cursor retriggers at key-on. Data bytes must be 0..$7F.
- **Pitch and pan contour slots:** renderers exist, arming RESERVED
  (non-breaking doors).
- **`MEV_PAN` ($E4, FM-only):** raw $B4 byte — L/R bits 7–6, AMS 5–4, FMS 2–0.
  Write-on-change via shadow.
- **SSG-EG:** per-op $90+ bytes at patch load; mid-note via REGWRITE/macros.
  (Runtime RegDelta group 6 for SSG-EG = engine package 4.)

## 6. PSG / noise / DAC lanes

- PSG tone: divisor-based pitch, porta/vibrato/detune supported (no block reg).
- **`MEV_PSGNOISE` ($F2, noise route only):** SN76489 noise ctrl byte $E0..$EF
  (mode + rate; rate-3 = tone-2-coupled, see §2).
- **`MEV_DAC` ($E2, DAC route only):** trigger sample id (descriptor table).
  Async playback, ~18.4 kHz register-resident stream (195 cyc/sample),
  DMA-survival ring (256 B, FILL/DRAIN/DRAINING_TAIL state machine,
  DC-center $80 at edges).
- **DAC register protection:** $2A/$2B (and timer block $24–$27) are refused
  by REGWRITE and macro reg-writes; every escape-hatch write re-parks $2A.

## 7. Global & special opcodes

- **`MEV_TEMPO` ($F3):** global tempo mod 0..$FE ($FF reserved — mailbox
  sentinel). Overrides header until 68k restore command. Load-boundary rule:
  song load snaps tempo to the new header; game re-asserts speed-shoes mod.
- **`MEV_LFO` ($F4):** YM $22 — GLOBAL hardware LFO (bit3 enable, bits0–2
  rate). One unit for the whole chip; per-channel depth = AMS/FMS in $B4.
- **`MEV_SPINREV` ($F0) / `MEV_SPINREV_RESET` ($F1):** SFX-ONLY spindash rev
  transpose (music-illegal; $F1 in music = bad-opcode trap).
- **`MEV_REGWRITE` ($F8):** raw part+reg+val escape hatch (FM routes + one
  narrow DAC door: part1 $B6 only); engine-guarded skips for $2A/$2B/$24–$27.
- **`MEV_MACRO` ($F9):** (re)arm the channel's slot[1] stream; legal only on
  channels with a macro body.
- **`MEV_EXT` ($FA):** extension prefix. Registry: sub-op 0 = **COMM**
  (score-authored cue byte → `SND_STAT_COMM`, game polls; engine package 1);
  1 = **PUMPSET** (+id +depth: kick-sidechain duck, package 5 Tier 1);
  2 = **GHOSTSET** (+src +ghost_route +delay +vol_drop +detune +pan_mode:
  unified echo/unison, package 5 Tier 2, budget-gated ≥172 B).

## 8. Slot[1] macro spine (per-channel automation stream)

Tags (private namespace, values overlap MEV_* deliberately):
`TAG_MAC_NEXT` ($E0) yield-1-frame; `TAG_MAC_REG` ($E1) part+reg+val immediate
write (same guards as REGWRITE); `TAG_MAC_LOOP` ($E2) jump to body start;
`TAG_MAC_END` ($E3) disarm. Package 5 Tier 1 adds `TAG_MAC_PAN` ($E4)
mode∈{$80 L,$40 R,$C0 LR} (autopan). Validity: body non-empty, ends with
END/LOOP, ≥1 NEXT before LOOP (else Z80 hard-spin), operand values must avoid
$E0–$E3, reg guards as above. One stream per channel; no nesting.

## 9. Production-suite features (engine package 5)

**Tier 0 (build-time, zero resident bytes):** drum mastering chain (EQ/comp/
saturate/gated-reverb; seed-deterministic, `tools/master_dac.py`); ladder-aware
level staging (MD1-vs-MD2 reference choice, runbook); TL-filter-sweep generator
(filter_env → modulator-TL curves via TAG_MAC_REG — carrier-only FMENV can't
reach modulators); PSG periodic-noise sub-bass (rate-3 pattern, 4 octaves below
clocking tone); generative variation (`tools/song_variation.py`: humanize_vol,
ghost_notes, alternate sample offsets, flam → pre-baked composites — ALL
seeded, no runtime mixing); SSG-EG timbre vocabulary (gated on package 4);
echo hand-authoring rules (−6 dB, opposite pan, duller patch — precursor to
GHOSTSET).

**Tier 1 (~30–50 B each, unconditional):** kick-sidechain pump (PUMPSET;
MAX-combines with SFX duck level; instant attack, duck ramp = release);
autopan (TAG_MAC_PAN).

**Tier 2 (budget-gated post packages 1–4):** ghost-voice engine (GHOSTSET —
one slot/song; ghost route must be a spare FM channel NOT in the score roster;
delay=0+detune = unison; SFX-steal-aware: ghost key skipped gracefully);
ExtCh3 operator-as-track (routes `CHROUTE_FM3_OP0..OP3`, alg-7 4-note chord
mode only in v1, one shared patch, TL as volume; plain FM3 route forbidden
when op routes present; $27 mode set at load). **FM3 special mode is NOT
implemented today** — door only.

**Design doors (NOT features):** CSM formant mode (Timer-A conflict — door
only), PSG 3-ch volume-register PCM, looped DAC (`ds_loop_ofs` reserved),
block-adaptive DDPCM, 26 kHz DAC, alg-4 dual-voice ExtCh3.

## 10. Game-feel features (engine package 1)

- **Pause/unpause:** music freeze+mute distinct from stop; SFX live; DAC
  unaffected; held notes re-articulate at next musical event (no fake
  re-attack).
- **Jingle push/pop with mid-song resume:** jingle = multi-channel SFX (≤3
  voices, FM4/FM5+PSG windows, NO FM6/DAC, NO loop — validity class); music
  freezes in place (zero-copy snapshot — SFX RAM is separate), auto-pops when
  jingle idles, resume fade-in (~S2 slope).
- **Song-finished contract:** `SND_STAT_SEQ_ACTIVE` mirror (all channels
  MEV_END → cleared) + `SND_STAT_COMM` cue byte (MEV_EXT COMM markers). Poll
  model. Non-looping songs signal end; looping songs never emit MEV_END.
- **Composed fade terminals:** fade-out→STOP / fade-out→PAUSE request codes +
  `SND_STAT_FADE_BUSY` mirror.
- **Tempo scalar load-boundary rule:** header tempo snaps on every load; the
  game re-asserts speed-shoes tempo after any music load. SFX cadence is NOT
  tempo-scaled.

## 11. DAC sample authoring (ratified 2026-06-24 spec + package 3)

Raw 8-bit PCM (DPCM-HQ encoder door) at fixed engine rate (~18–22 kHz target);
9-byte descriptors (`ds_bank`, `ds_rate` reserved, `ds_table`, `ds_ptr`,
`ds_length`, `ds_loop_ofs` reserved, + `ds_vol` insurance riding package 3);
samples <32 KB, must not straddle the $8000 bank window; one voice, one-shot,
NO runtime mixing — simultaneity = pre-mixed offline composites (each owns its
own slot); import pipeline = resample → mastering chain → encode
(`tools/dac_encode.py`); verification = cross-correlation vs same-codec
reference r≥0.9 + rendered spectrum A/B.

## 12. SFX tier (for the S6 workshop)

- 7 SfxChannel slots (64 B each, $1D00 base), transcoded blobs
  (`sfx_transcode.py` today — Seraph compiles directly post-S1).
- Headers: priority (≥$C0 ducks music), and per engine package 2 (Stage B/C):
  `sfh_gain`, `sfh_duck`, `sfh_cap` (instance cap, single-channel-only rule),
  non-latching priority (bit 7), `SHF_CONTINUOUS` class (spindash charge,
  drowning warning; re-ping countdown).
- Channel windows: FM4/FM5 + PSG per window tables; noise SFX drop tone
  coupling (smpsPSGform not supported in SFX).
- Jingles ride this tier (see §10). Testing requires
  `SOUND_DEBUG_HOTKEYS=1` builds (Dbg_Sfx_Sel $FF8A12).

## 13. Budget & scheduling constraints (DAW must surface)

- Resident code ceiling $18F0; headroom 362 B DEBUG / 488 B release
  (2026-07-03); table-banking lever EXHAUSTED — Tier-2 budget gates are real.
- YM write pacing ~1 write / ~33.6 Z80 cycles; **no more than one FM patch
  load per frame** — stagger simultaneous FM key-ons or accept a 1-frame
  stagger inserted by the compiler (surfaced, never silent).
- REGDELTA groups + macro reg-writes + contour renders are additive per-frame
  write cost → budget meter input; driver-in-the-loop preview is ground truth.
- Banked in-frame CODE is unsafe (bus contention) — DATA banking only; song
  bank vs DAC sample bank swap brackets are engine-managed (no author choice).

## 14. Song header (compiler target)

Fixed: `SH_FLAGS` (FM6 mode bits + `SH_F_STREAM` always set), `SH_TEMPO`
(legacy), `SH_TEMPO_MOD`, `SH_CHCOUNT` (1..11), pitch-table offset (BE, 0 =
default). Then per channel: route byte + slot[0] cmd offset (BE) + slot[1] mod
offset (BE, 0 = none). Patch table ptr forwarded 68k-side
(`SND_MUSIC_PARAM_PATCHPTR`). All blob offsets 16-bit BE, blob ≤ $FFFF.

## 15. The exporter contract (normative — Seraph enforces ALL of it)

Memra is TRUST-THE-PACKER: the Z80 never re-validates; a violating blob hangs
or corrupts silently. Seraph is the validator. Full normative list lives in
`aeon/docs/superpowers/specs/2026-06-23-music-expression-engine-design.md`
(package-0 "format validity rules"); summary of rule classes:

1. Stream structure: non-empty, ends $EF/$FF, JUMP requires LOOP_POINT, loop
   body advances time, routes unique, channel count 1..11, flag implications
   (`SH_F_FM6_ADAPTIVE` ⇒ `SH_F_FM6_FM`).
2. Operand bounds: pitch 0..$5E, vol 0..127, durations, counts, signed ranges,
   env ids 1-based, tempo ≠ $FF, PITCHENV 1–5 points ≤ $83.
3. Route legality: FM-only / PSG-only / noise-only / DAC-only opcode gating
   (see §5–§7); music-illegal opcodes ($F1) rejected.
4. Init ordering: FM = Patch+Vol before first time-advancing event; PSG = Vol;
   DAC exempt.
5. Repeats: single-level, matched, count ≠ 0, body advances time.
6. Macros: body termination, ≥1 NEXT before LOOP, no $E0–$E3 operand values,
   reg guards, MEV_MACRO only with a body.
7. Field invariants: porta seeded by a prior note; MEV_EXT only registered
   sub-ops; jingle class (≤3 voices, no FM6/DAC, no loop).
