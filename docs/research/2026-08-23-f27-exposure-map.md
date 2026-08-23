# F27 exposure map — where the FM6/DAC collision shows up in Seraph

**Date:** 2026-08-23 · **Status:** research, read-only. No product code changed.
**Branch:** `research/f27-exposure-map`, forked from `8871d77`
(`docs(status): fresh boot — the two F27 investigations died at rotation with zero commits`).

**Scope.** The driver's exact steal semantics are OUT of scope here (a sibling agent
owns that). This report assumes only *"FM6 and the DAC contend for one hardware
channel"* and maps Seraph's side: every place that touches the data, what each one
does today, and what three candidate fixes would cost.

No emulator was touched. No build and no test run were performed — every claim below
is from reading committed source in this worktree, and every cost is labelled
**measured** or **judged**.

---

## 0. Two things found on the way in that change how you read the rest

### 0.1 `src/components/PianoRoll.tsx` is invisible to `grep`

The file contains a literal NUL byte inside a string constant, so `file` classifies it
as `data` and grep skips it **silently** — no error, no exit code, nothing:

```
$ sed -n '703p' src/components/PianoRoll.tsx | od -c
0000000       c   o   n   s   t       M   I   X   E   D   _   V   O
0000020   I   C   E       =       "  \0   m   i   x   e   d   "   ;  \n
```

```
$ grep -n import src/components/PianoRoll.tsx      # no output, exit 1
$ grep -a -n import src/components/PianoRoll.tsx   # 30+ hits
```

`src/components/PianoRoll.tsx` is 907 lines (measured) and is the *entire* note-editing
surface. **Every prior text search over `src/` that did not pass `-a` silently excluded
it.** It is the only such file in the tree:

```
$ git ls-files -- src | xargs -I{} sh -c 'e=$(file -b --mime-encoding "{}"); [ "$e" = binary ] && echo "{}"'
src/components/PianoRoll.tsx
```

(the same sweep over `src-tauri` returns only the 17 files under `src-tauri/icons/`.)

Every frontend grep in this report was re-run with `-a`. This is not an F27 defect but
it is a **grep-integrity defect** that will keep producing confident false negatives in
this repo until the NUL is replaced with a normal sentinel (e.g. `"<NUL>mixed"` written
as an escape, or just `"__mixed__"`). Recommend booking separately.

### 0.2 Writing `$2B` would NOT, on its own, fix the preview

The obvious fix — "route the DAC through the chip: write `$2B` bit 7 and stream samples
to `$2A`, and Nuked-OPN2 models the steal for free" — **does not work through Seraph's
current read path.** Nuked applies the DAC substitution in the time-multiplexed
`mol`/`mor` output stage, not in the per-channel array:

`src-tauri/vendor/nuked-opn2/ym3438.c`, in `OPN2_Clock`'s channel-output block:

```c
    /* Ch 6 */
    if (((cycles >> 2) == 1 && chip->dacen) || test_dac)
    {
        out = (Bit16s)chip->dacdata;
```

but `AudioEngine::ym_clock_settle` reads through `Ym2612::read_channels` →
`OPN2_ReadChannels`, which is:

```c
void OPN2_ReadChannels(ym3438_t *chip, Bit16s *left, Bit16s *right)
{
    for (i = 0; i < 6; i++)
    {
        Bit16s out = chip->ch_out[i];
```

`ch_out[5]` is the FM channel's output regardless of `dacen`. So a `$2B` write would
change nothing audible in Seraph. This is load-bearing for the option (a) pricing in
§5 — treat any estimate that assumes "just write `$2B`" as wrong.

---

## 1. The preview / audio path

### 1.1 How the DAC is mixed today

The DAC is a **fourth independent stream**, parallel to the YM2612, the SN76489 and the
PSG-envelope players. It lives entirely in three fields of `AudioEngine`
(`src-tauri/src/audio/engine.rs`): `dac_samples: Option<Arc<Vec<u8>>>`, `dac_position`,
`dac_step`.

`AudioEngine::render` reads one byte per output frame and sums it flat into both
channels, after the FM path has already been fully mixed:

```rust
let dac_mix = dac_sample as f32 * 48.0 / 65536.0;
let psg_mix = psg_sample as f32 * 9.0 / (16.0 * 65536.0);
let pre_l = (fm_l + dac_mix + psg_mix) * self.master_volume;
```

`fm_l` at that point is the sum of **all six** FM channels including channel 6. Nothing
anywhere reduces one when the other is live.

### 1.2 Where a steal would have to be modelled — and it is more than one place

The "is the DAC sounding" predicate is `self.dac_samples.is_some()`. That state is
touched at exactly eight sites, all in `engine.rs`, established by:

```
$ grep -rn 'dac_samples' src-tauri/src --include='*.rs'
src-tauri/src/audio/engine.rs:123    (field declaration)
src-tauri/src/audio/engine.rs:192    (AudioEngine::new — init None)
src-tauri/src/audio/engine.rs:314    (process_command, AudioCommand::DacPlayback — SET)
src-tauri/src/audio/engine.rs:331    (process_command, AudioCommand::StopPreview — CLEAR)
src-tauri/src/audio/engine.rs:351    (process_command, AudioCommand::Panic — CLEAR)
src-tauri/src/audio/engine.rs:428    (apply_sequencer_output, SequencerOutput::DacPlayback — SET)
src-tauri/src/audio/engine.rs:582    (render — READ)
src-tauri/src/audio/engine.rs:588    (render, samples exhausted — CLEAR)
```

That is the complete set (the command above is the check that earns the claim). So there
are **two independent entry points** that start a DAC sample and **three** that end one:

| entry | function | reached from |
|---|---|---|
| SET | `AudioEngine::process_command` → `AudioCommand::DacPlayback` | `ipc::commands::preview_dac` (auditioning a sample from the DAC editor / the piano-roll Sample picker) |
| SET | `AudioEngine::apply_sequencer_output` → `SequencerOutput::DacPlayback` | sequencer playback, emitted by `Sequencer::process_event`'s `ChannelType::Dac(_)` arm |
| CLEAR | `process_command` → `StopPreview` | `library_stop_audition` |
| CLEAR | `process_command` → `Panic` | `stop_all_sound` |
| CLEAR | `render` | sample runs out — **the common case, and the only one that is not a command** |

A "steal FM6 while the DAC is live" model therefore needs **an edge-triggered hook on
five sites, not one**, and the release edge (sample exhaustion) fires from inside the
render loop rather than from the command path. Modelling it purely in `process_command`
would leave FM6 muted forever after the first drum hit.

Additionally, three more functions carry a `Dac` arm that today does nothing and would
have to learn the rule (or be deliberately exempted):

- `Sequencer::process_event`, `ChannelType::Dac(_)` arm — pushes `DacPlayback`, no FM6 awareness.
- `Sequencer::key_off_channel`, `ChannelType::Dac(_) => {}` — empty. A DAC "note-off" is a no-op, so the *authored duration* of a DAC note is not currently modelled at all; the sample simply plays to its end. That matters: if the steal is keyed off "a DAC note is live", **live** has two possible meanings (authored span vs. actual sample length) and they diverge.
- `Sequencer::reprogram_live`, `ChannelType::Dac(_) => {}` — live-edit path, same.

### 1.3 Does `build_snapshot` / the render path diverge from live preview?

**No — they are the same code, which is the one piece of good news here.**

`ProjectManager::build_snapshot` emits one `ChannelSequence` per `channel_key`
(`fm_5` and `dac_0` are different keys, so FM6 and DAC are simply two unrelated
channels). Both `transport_play` and `export_wav` feed that same snapshot into the same
`AudioEngine`:

- `ipc::commands::transport_play` → `mgr.build_snapshot()` → `AudioCommand::LoadSequence` → the cpal-hosted `AudioEngine`.
- `ipc::commands::export_wav` → `mgr.build_snapshot()` → a **freshly constructed** `AudioEngine::new(44100)` → `engine.render(...)` in a loop.

So there is exactly one preview model and one bug, not two. A fix inside
`AudioEngine`/`Sequencer` lands on live preview and WAV export simultaneously.

`build_snapshot` *does* compute overlap diagnostics — but strictly **within** a channel
(`for_each_conflicting_span` over `overlap_sources`, which is built per `channel_key`
group). There is no cross-channel pass, so it can never see FM6-vs-DAC.

---

## 2. The export paths — verdict per path

There are four export entry points. They are **not** uniformly wrong, and the shapes
differ in ways that matter to the user story.

### 2.1 SMPS / driver export (`export_song` → `FlamedriverProfile::export_song` → `export::smps::write_export`) — **WRONG (promises both)**

`export::smps::generate_music_asm` counts FM and DAC tracks and writes a combined header:

```rust
let fm_dac_count = fm_count + dac_count;
...
asm.push_str(&format!("\tsmpsHeaderChan\t\t${fm_dac_count:02X}, ${psg_count:02X}\n"));
```

then unconditionally emits a `smpsHeaderDAC` line for every DAC track **and** a
`smpsHeaderFM` line for every FM track including index 5. `export::smps::validate_for_export`
checks: missing instrument, missing instrument in bank, overlapping *regions* on one
track, pitch range, duration rounding. There is **no cross-channel check of any kind**.

So a project with an FM6 track and a DAC track exports an assembly file that asks the
driver for both. Whatever the driver then does, the export is a faithful transcription
of an unrepresentable request — the same defect class as the preview.

### 2.2 WAV export (`export_wav`) — **WRONG, identically to the preview**

Shares `build_snapshot` and `AudioEngine` (see §1.3). The rendered WAV contains FM6 and
the drum sounding together, byte-for-byte the same lie the speakers tell.

**Interaction with the booked hardcoded-60 s defect:** `src/components/TopBar.tsx` calls
`await ipc.exportWav(path, 60)` — a literal. The backend `export_wav` takes
`duration_seconds` and honours it; only the caller is hardcoded. For F27 this is a
*severity multiplier in one direction only*: the 60 s cap means the exported artefact
that would demonstrate the bug is truncated, but the first 60 s of a song is exactly
where drums and FM6 overlap. Fixing the 60 s cap does not touch F27; fixing F27 does not
touch the cap. **They are independent.**

### 2.3 VGM export (`export_vgm` → `export::vgm::export_vgm_data`) — **WRONG IN THE OPPOSITE DIRECTION**

```rust
let (ch_type, hw_ch) = match track.channel {
    ChannelAssignment::Fm(n) => (ChType::Fm, n),
    ChannelAssignment::Psg(n) => (ChType::Psg, n),
    ChannelAssignment::PsgNoise => (ChType::PsgNoise, 3),
    ChannelAssignment::Dac(_) => continue,
};
```

`ChType` has three variants — `Fm`, `Psg`, `PsgNoise`. **There is no DAC in the VGM
exporter at all.** Every DAC track is dropped, no `$2B` is written, no `$2A` stream is
produced. The resulting VGM is *internally consistent with hardware* (FM6 plays, nothing
contends) but is **not the song** — the drums are missing.

This is a materially different user story from 2.1/2.2 and should be booked as such: it
is not "the export encodes the collision wrongly", it is "the export silently discards a
whole instrument class".

One trap for whoever fixes this: `grep '0x2B'` over the exporter hits

```
src-tauri/src/export/vgm.rs:82:        out[0x2B] = 0;  // flags
```

which is **VGM header byte offset 0x2B (SN76489 shift-register flags)**, not the YM2612
register. Do not mistake it for DAC-enable handling.

**Interaction with the booked dead-`export_vgm`-UI defect:** confirmed dead. The
binding exists (`src/bindings.ts` `exportVgm`), the wrapper exists
(`src/api/ipc.ts::exportVgm`), and **no component calls it**:

```
$ grep -rn -a 'exportVgm\|export_vgm' src
src/bindings.ts:551,553   src/api/ipc.ts:411,412        (definitions only)
```

versus the control, `exportWav`, which additionally hits `src/components/TopBar.tsx:102`.
So today F27's VGM manifestation is **unreachable by a user**. That lowers its urgency
but not its correctness: whoever wires the VGM export button up will ship the
drums-are-missing bug on day one unless 2.3 is fixed first. **Sequence 2.3 before the
README-7 VGM-UI fix.**

### 2.4 The import side, for contrast — **CORRECT, and it is the only place in Seraph that knows the rule**

`src-tauri/src/import/vgm_import.rs` models `$2B` properly. `ImportState` carries
`dac_enabled`; `ImportState::process_ym2612` maintains it:

```rust
if port == 0 && addr == 0x2B {
    let enabled = val & 0x80 != 0;
    if enabled { self.dac_ever_enabled = true; }
    self.dac_enabled = enabled;
    return;
}
```

and `ImportState::process_key_on` **implements the steal**:

```rust
if hw_ch == 5 && self.dac_enabled {
    return;
}
```

That is the *only* line in the whole Rust tree that couples FM channel 5 to DAC state:

```
$ grep -rn 'Fm(5)\|hw_ch == 5\|hw_channel == 5\|== 5\b' src-tauri/src --include='*.rs'
src-tauri/src/import/vgm_import.rs:432:        if hw_ch == 5 && self.dac_enabled {
```

**Consequence:** a project imported from VGM is self-consistent — the importer already
dropped the FM6 key-ons that the DAC stole, so the imported song will never exhibit the
collision. Only **hand-authored** and **SMPS-imported** projects can. This narrows the
blast radius considerably and is worth stating in any user-facing note.

`import::smps_mapper::map_smps_to_project` assigns FM tracks **sequential indices from
0** (`fm_idx += 1`), independent of which hardware channel the driver will place them on,
and always maps a `SmpsChannelKind::Dac` to `ChannelAssignment::Dac(0)`. So an SMPS song
with six FM channels *and* a DAC channel would produce FM6 + DAC in Seraph. Whether such
source songs exist depends on the driver's channel-table convention — **out of scope,
flagged for the driver agent.**

---

## 3. The authoring gate — can the constraint ride on `check_voice_overlap`?

### 3.1 What `check_voice_overlap` actually is

`ProjectManager::check_voice_overlap` is **not** a hardware-ceiling gate. Its doc comment
is explicit: it rejects an edit that would leave an *edited* note overlapping, **on the
same output channel**, a note whose effective voice (note > region > track) is a
**different concrete instrument**. Its very first act is:

```rust
let key = channel_key(&target.channel);
...
for track in self.tracks.iter().filter(|t| channel_key(&t.channel) == key) {
```

`channel_key` yields `fm_5` for FM6 and `dac_0` for the DAC
(`ChannelAssignment::Dac(n) => format!("dac_{n}")`). **The two never meet.** The gate is
single-channel by construction, at the very first line.

The F25 "DAC is one channel" behaviour is an *emergent* property: because all DAC tracks
share `dac_0`, two overlapping DAC notes carrying different samples trip the same
generic voice rule. It is a same-channel/different-voice rejection that happens to read
as a hardware statement.

Callers, enumerated:

```
$ grep -rn 'check_voice_overlap' src-tauri/src --include='*.rs'
project/manager.rs:1418   (add_note — only when an EXPLICIT per-note instrument_id is passed)
project/manager.rs:1459   (set_note_instrument — always)
project/manager.rs:1510   (definition)
```

Note the reachability asymmetry, which matters for option (c): **`add_note` with
`instrument_id: None` bypasses the gate entirely** ("`None` keeps the historical
behavior for every existing caller: no gates"). And `update_note` — which moves a note in
time — runs **no** gate at all. So dragging a note into an overlap is already legal
today. Any FM6/DAC gate hung off this machinery would inherit those holes.

### 3.2 Cost of riding the existing machinery

Two distinct pieces of machinery exist, and the constraint fits one much better:

**(i) `check_voice_overlap` (the rejecting gate).** Poor fit. It would need to become
cross-channel: a second span set built from a *different* `channel_key`, a rule that is
about channel identity rather than voice identity, and an error message in a different
register. Judged: this is not "add a case", it is a second rule sharing a helper
(`for_each_conflicting_span`). Plus the two bypasses above must be closed or the gate is
theatre.

**(ii) `build_snapshot`'s `OverlapWarning` pass (the reporting diagnostic).** Good fit.
It already produces exactly the right payload shape:

```rust
pub struct OverlapWarning {
    pub channel_name: String,
    pub tick_start: u64,
    pub tick_end: u64,
    pub track_ids: Vec<String>,
}
```

and `for_each_conflicting_span` is already generic over a tagged span
(`fn for_each_conflicting_span<T>`). Emitting an FM6-vs-DAC warning means building one
extra span list from the `fm_5` and `dac_0` groups and running the same helper. It is
additive, it changes no existing behaviour, and it reuses the serialisation and the
tauri-specta binding that already exist.

### 3.3 The `get_channel_overlaps` situation

`get_channel_overlaps` is a registered tauri command (`lib.rs:49,141`, `ipc/mod.rs:34`)
backed by `ProjectManager::get_all_overlaps`, exposed in `src/bindings.ts:503` and
wrapped in `src/api/ipc.ts:379`. **No component calls it** — confirmed with the
NUL-safe sweep:

```
$ grep -rn -a 'getChannelOverlaps' src
src/api/ipc.ts:379, 380
src/bindings.ts:503, 505
```

(definitions and the wrapper only; zero call sites). So `build_snapshot` computes
overlap warnings on every snapshot build and throws them away. **This is F27's cheapest
lever:** the reporting half of a "visible warning" fix is already written, already
tested (`test_build_snapshot_detects_overlaps` in `project/manager.rs`), and already
plumbed to the frontend — it just has no consumer. Fixing F27 via (b) simultaneously
retires the booked `get_channel_overlaps` defect.

### 3.4 Existing rejection/notice surface in the UI

Yes, F25 shipped one. `src/components/PianoRoll.tsx`:

- state `const [voiceHint, setVoiceHint] = useState<string | null>(null)` plus
  `showVoiceHint(msg)`, which sets the message and clears it after 5000 ms;
- rendered in the piano-roll header as `{voiceHint && <span className={styles.voiceHint}>{voiceHint}</span>}`;
- styled by `.voiceHint` in `PianoRoll.module.css`, whose comment reads
  *"Non-modal inline notice (drop hints, voice-overlap rejections)."*

It has **six** call sites in that file (paste failure, two "select notes first" hints,
and three backend-rejection paths including `handleDacSamplePick`'s `catch`). The
component's own comment names the constraint: *"the app has no toast system"*.

Its limit is scope: it lives inside `PianoRoll` and is not reachable from the arrangement
view. `ArrangementView.tsx` says so directly, in a `catch` that resorts to `console.error`:

> *"This view has no notice element (the app has no toast system and the piano-roll
> header hint is not reachable from here), so the console is the honest floor."*

So a warning raised at *arrangement* time (dropping a DAC region under an FM6 region)
has **no** surface today. That is a real cost line for option (b).

---

## 4. The frontend — where a user would see it

### 4.1 The F25 tooltip, and whether it generalises

Found. It is on the DAC sample `<select>` in the piano-roll header, and it is
**conditional on both `isDac` and the selection being non-empty**:

```
title={
  selectedNotes.size === 0
    ? "Select notes first, then pick the sample they play. …"
    : "Sets the sample the selected notes play. The DAC channel plays ONE sample at a time, so notes that overlap must share a sample."
}
```

**It does not generalise.** Three reasons: (1) the control is rendered only when
`isDac` — an author sitting on the FM6 lane never sees it; (2) it speaks about *samples*
sharing one channel, not about a *different lane* taking the channel away; (3) it is a
native `title` tooltip on a picker, i.e. discoverable only by hovering a control you
already decided to use. The `voiceHint` element (§3.4) is the generalisable surface, not
this string.

### 4.2 The one place the collision is already named — and where it gets thrown away

`FlamedriverProfile::channel_layout` names FM channel index 5:

```rust
FmChannelInfo { index: 5, name: "FM6/DAC".into(), supports_special_mode: false },
```

That name reaches the UI through `get_driver_info` → `DriverDetail.layout`, and
`AddTrackDialog` renders it verbatim in the channel picker (`{ch.name}`). So **the Add
Track dialog is the only user-visible place in Seraph that hints at the constraint** —
and it hints only, it does not gate.

Everywhere else the name is discarded and reconstructed from the index:

```ts
// src/components/TrackHeader.tsx  and, byte-identically, src/components/ArrangementView.tsx
function channelLabel(track: Track): string {
  if (typeof ch === "object" && "Fm" in ch) return `FM${ch.Fm + 1}`;
  if (typeof ch === "object" && "Dac" in ch) return "DAC";
```

So the track header badge reads plain **"FM6"**, not "FM6/DAC". Two duplicated copies of
this function exist (`TrackHeader.tsx` and `ArrangementView.tsx`); a rename would have to
touch both.

The backend has the same duplication: `ProjectManager::channel_display_name` also builds
`format!("FM{}", n + 1)` from the index and ignores the layout name. That is the string
`OverlapWarning.channel_name` carries — so a warning about FM6 would say "FM6" unless
this is changed too.

### 4.3 Every project starts in the colliding state

`ProjectManager::default_tracks_for_layout` seeds **one instrument-less lane per driver
channel**, iterating `fm_channels`, then `psg_channels`, then `dac_channels`. For
Flamedriver that is 6 + 4 + 1 = 11 lanes, including both an `Fm(5)` lane and a `Dac(0)`
lane. **A brand-new project already contains the two lanes that cannot coexist**, side
by side in the arrangement, with no indication. This is why the finding is not exotic:
the author does not have to do anything unusual to reach it.

### 4.4 The meters lie in the same way the audio does

`AudioEngine::update_channel_levels` maps `ChannelType::Fm(n) => n` and
`ChannelType::Dac(_) => 10` into a 16-slot level array. The frontend mirrors it exactly
in `TrackHeader.channelLevelIndex` (`Fm` → `ch.Fm`, `Dac` → `10`), consumed by
`ArrangementView` at `level={channelLevels[channelLevelIndex(track)] ?? 0}`. So during a
drum hit under an FM6 note, **both meters light**, which is a second, independent
statement of the same false claim. Any fix that changes the audio without changing the
meters leaves a visibly contradictory UI.

Note also that `Sequencer::channel_activity` derives "active" from `active_notes`, which
for the DAC is set on note-on and cleared on note-off — and `key_off_channel`'s DAC arm
is empty, so the DAC meter tracks the *authored* span, while the audio tracks the
*sample length*. Pre-existing minor divergence; relevant if the steal is keyed to the
meter's notion of "live".

### 4.5 Dead component, noted in passing

`src/components/TrackList.tsx` renders a `DAC` badge and is **imported by nothing**
(`grep -rn -a 'TrackList' src --include='*.tsx'` returns only its own `import styles`
line and its `.module.css`). Not an F27 surface; worth booking as dead code.

---

## 5. Pricing the options

**All figures below are judged, not measured**, except where marked. This workspace's
booked cost tables have been wrong by 2.5x, 6x and 33% — read these as shapes, not
budgets. What is measured: file sizes, call-site counts, and the grep outputs quoted
above.

Measured context for scale: `audio/engine.rs` 1533 lines, `sequencer/mod.rs` 990,
`project/manager.rs` 3174, `export/smps.rs` 1056, `export/vgm.rs` 470,
`PianoRoll.tsx` 907. The nearest precedent harness,
`src-tauri/src/audio/overlap_audibility.rs`, is 319 lines and is `#[cfg(test)]`-gated in
`audio/mod.rs`.

### Option (a) — silent steal: model it in preview so it sounds like hardware

**Files.**
1. `src-tauri/src/audio/engine.rs` — the steal itself. Because `$2B` is invisible through `OPN2_ReadChannels` (§0.2), the two workable shapes are:
   - **(a-i) suppress FM6 in the Rust mixer.** Needs per-channel visibility that `read_channels` does not provide (it returns a pre-summed pair), so it needs a new C entry point — e.g. `OPN2_ReadChannelsExcept6` or a `OPN2_ReadChannel(chip, i)` — added to `src-tauri/vendor/nuked-opn2/ym3438.c` + `.h` + one `extern` in `src-tauri/src/ym2612/bindings.rs` + one method in `ym2612/chip.rs`. `OPN2_ReadChannels` sits at the very end of the vendored file, after `OPN2_Read`, and is declared last in `ym3438.h` — which *looks* like a local append (inference; I did not diff against upstream Nuked), so there is likely precedent for editing the vendored file. ~15 lines of C, ~10 of Rust.
   - **(a-ii) key off FM6 at the driver level**: when a DAC sample starts, emit the same `$28` key-off Seraph already emits (`key_off_channel`'s FM arm) for hw_ch 5, and suppress FM6 note-ons while the DAC is live. No new bindings; entirely inside `Sequencer` + `AudioEngine`. Closer to what the hardware/driver actually does, and it makes the meters correct for free.
2. `src-tauri/src/sequencer/mod.rs` — the "DAC is live" state has to be *known to the sequencer* for (a-ii), which today it is not (the sequencer fires `DacPlayback` and forgets). Adds a field + the note-on suppression branch in `process_event`'s `ChannelType::Fm(5)` case.
3. **Five edge sites** (§1.2), of which the release edge lives in `render`'s sample-exhaustion branch.
4. `src-tauri/src/export/smps.rs` and `export/vgm.rs` — *not touched by (a)*. See "does not fix" below.

**Tests that would have to exist.** A rendered-audio harness modelled on
`overlap_audibility.rs`: build a project with an `Fm(5)` track carrying a sustaining
library voice and a `Dac(0)` track carrying a sample, render through
`build_snapshot → Sequencer → AudioEngine`, and assert the FM6 window drops to the level
of a control render with the FM6 note deleted. `ProjectManager::add_dac_instrument`
takes PCM **in memory** (`add_dac_instrument(inst, pcm_data: Vec<u8>)`, caches into
`dac_pcm_cache`), so no file I/O is needed — the harness is genuinely cheap to build.
Judged: ~250-350 lines, one new `#[cfg(test)] pub mod` in `audio/mod.rs`, alongside
2-3 unit tests on the five edge sites.

**What it breaks / annoys.** (a-ii) makes an authored FM6 note vanish with no
explanation — the author hears the bass drop out under every kick and has nothing to
read. Given §4.3 (every new project ships both lanes), this will be reported as a bug on
first contact. (a-i) additionally forks the vendored Nuked file, which the repo will
carry forever.

**What it does NOT fix.** SMPS export still emits both channels (§2.1). VGM export still
drops the DAC (§2.3). The meters still both light unless (a-ii) is chosen or
`update_channel_levels` is separately taught. The author still cannot tell *why* their
note went quiet.

**Blocked on:** the resumption semantics — does FM6 come back after the sample ends, at
what latency, and does it re-key or resume mid-envelope? That is the sibling agent's
question, and (a) cannot be specified without it. **TAGGED for the controller.**

### Option (b) — visible warning: let it be authored, tell the user

**Files.**
1. `src-tauri/src/project/manager.rs` — a second, cross-channel pass in `build_snapshot` after the per-channel one, reusing `for_each_conflicting_span` over spans drawn from the `fm_5` and `dac_0` groups, pushing `OverlapWarning`s with a `channel_name` like `"FM6/DAC"`. Additive; no existing behaviour changes. Judged ~40-60 lines including the group lookup.
2. `src-tauri/src/project/manager.rs::channel_display_name` — teach it the layout name, or accept "FM6" in the warning text (§4.2).
3. `src/api/ipc.ts` / component wiring — call the **already existing, already bound, currently uncalled** `getChannelOverlaps` (§3.3) and surface results. Cheapest landing: reuse `PianoRoll`'s `voiceHint` element for the piano-roll case.
4. `src/components/ArrangementView.tsx` — this is the real cost. The arrangement view has **no notice element at all** (§3.4), and the collision is fundamentally an arrangement-level fact (two lanes), not a note-editing fact. Either a small persistent warning strip in the arrangement, or a per-lane badge on the FM6 and DAC headers, has to be built. Judged: the larger half of this option.

**Tests.** `project/manager.rs` unit tests in the shape of the existing
`test_build_snapshot_detects_overlaps` (measured: it already exists and asserts
`!fm0.overlaps.is_empty()`), plus a vitest for whatever UI element is added. No
rendered-audio harness needed — this option asserts on data, not on samples. Materially
cheaper to test than (a).

**What it breaks / annoys.** Nothing existing. The annoyance is chronic rather than
acute: because §4.3 seeds both lanes into every project, a naive "these two lanes
conflict" warning would fire on projects where the lanes merely *exist*. The warning must
be span-based (actual temporal overlap), which the `for_each_conflicting_span` reuse
gives for free.

**What it does NOT fix.** The preview still lies — the user is told one thing and hears
another, which is arguably worse than either being wrong alone. Both exports still wrong
(§2.1, §2.3). It converts a silent defect into an acknowledged one; it does not make
Seraph honest.

### Option (c) — authoring-time gate: refuse the edit

**Files.**
1. `src-tauri/src/project/manager.rs` — a new cross-channel gate. It cannot be a case inside `check_voice_overlap`, because that function's first line narrows to a single `channel_key` (§3.1); it would be a sibling rule sharing `for_each_conflicting_span`.
2. **The bypasses must be closed or the gate is decorative.** Measured: `check_voice_overlap` has exactly two callers, `add_note` (only when an explicit `instrument_id` is passed — the `None` path is documented as ungated) and `set_note_instrument`. `update_note`, which changes a note's `tick` and `duration_ticks`, runs **no** gate. `move_region`, `duplicate_region`, `update_region` and `add_region` likewise. To actually prevent an FM6/DAC overlap you must gate: `add_note` (both arms), `update_note`, `add_region`, `update_region`, `move_region`, `duplicate_region`, and `update_track` (which can *change a track's channel assignment* — re-pointing a lane at `Fm(5)` can create the overlap with no note edit at all). That is **seven or eight entry points**, most of which have never had a gate.
3. `src/components/PianoRoll.tsx` — reuse `voiceHint` for the rejection message (cheap, the plumbing exists: three of its six call sites are already backend-rejection `catch` blocks).
4. `src/components/ArrangementView.tsx` — same missing-notice problem as (b), and worse: region drags live here, so rejections *originate* here.

**Tests.** One unit test per gated entry point (7-8), plus the rejection-message vitest.
Judged: the largest test surface of the three, and the one most likely to expose existing
untested paths.

**What it breaks / annoys.** Severely. It refuses edits that are legitimate on hardware —
an FM6 note and a drum that merely *abut* are fine, and authors routinely write FM6
parts around drum hits. Getting the boundary condition wrong makes the DAW feel broken.
It also makes imported SMPS projects potentially unopenable-without-error if the importer
ever produces the pair (§2.4). And it is the only option that can make a *previously
valid* project reject a *later* edit.

**What it does NOT fix.** Both exports remain wrong for projects created before the gate
(and for VGM, for all projects). The preview remains wrong for any pre-existing
collision the gate does not retroactively remove — and nothing here retroactively removes
anything.

### Option (d) — the one I would actually book: (a-ii) + (b)'s diagnostic, exports separately

Model the steal in the sequencer so the preview and WAV export sound like hardware
(a-ii), **and** emit the cross-channel `OverlapWarning` from `build_snapshot` (b's
cheap half) so the author is told why their FM6 note went quiet. The two share no code
and can land in either order. This is exactly the shape the overlap last-note-priority
fix already took, and `build_snapshot` says so in a comment that reads like it was
written for this finding:

> *"The overlap diagnostics are UNCHANGED by last-note-priority. The ambiguity is real
> and still the author's to resolve; it now resolves at compile time (deterministically,
> the way the driver resolves it) instead of at playback, which is a reason to keep
> surfacing it, not to stop."*

Booking (b)'s diagnostic also retires the `get_channel_overlaps` defect from README-7 as
a side effect, since the command finally acquires a caller.

The exports (§2.1, §2.3) are **not** fixed by any of a/b/c and should be booked as
separate items. §2.3 in particular should be sequenced before the README-7 VGM-UI wiring.

---

## 6. Summary table — every surface and its verdict

Verdicts: **wrong** = actively states something false; **silent** = has the data and says
nothing (a place a fix must touch); **correct** = models the constraint.

| # | Surface (file · symbol) | Verdict | Note |
|---|---|---|---|
| 1 | `audio/engine.rs` · `AudioEngine::render` | **wrong** | sums `fm_l + dac_mix`; FM6 never reduced |
| 2 | `audio/engine.rs` · `AudioEngine::apply_sequencer_output` (`DacPlayback` arm) | **wrong** | starts the DAC, never touches FM6 |
| 3 | `audio/engine.rs` · `AudioEngine::process_command` (`DacPlayback` arm) | **wrong** | preview/audition entry; same gap |
| 4 | `audio/engine.rs` · `AudioEngine::process_command` (`StopPreview`, `Panic`) | silent | DAC-clear edges; a fix must hook them |
| 5 | `audio/engine.rs` · `render` sample-exhaustion branch | silent | the *common* release edge, outside the command path |
| 6 | `audio/engine.rs` · `AudioEngine::update_channel_levels` | **wrong** | FM6 (idx 5) and DAC (idx 10) both light |
| 7 | `audio/engine.rs` · `AudioEngine::ym_clock_settle` → `OPN2_ReadChannels` | **wrong** | reads `ch_out[5]`; blind to `dacen` by construction |
| 8 | `sequencer/mod.rs` · `Sequencer::process_event` (`ChannelType::Dac`) | silent | emits `DacPlayback`; no cross-channel state |
| 9 | `sequencer/mod.rs` · `Sequencer::key_off_channel` (`Dac` arm, empty) | silent | DAC note-off is a no-op; "live" is ambiguous |
| 10 | `sequencer/mod.rs` · `Sequencer::reprogram_live` (`Dac` arm, empty) | silent | live-edit path |
| 11 | `sequencer/mod.rs` · `Sequencer::channel_activity` | **wrong** | feeds #6 |
| 12 | `project/manager.rs` · `ProjectManager::build_snapshot` | **wrong** | `fm_5` and `dac_0` are unrelated channels; overlap pass is intra-channel only |
| 13 | `project/manager.rs` · `ProjectManager::default_tracks_for_layout` | **wrong** | every new project ships both lanes |
| 14 | `project/manager.rs` · `ProjectManager::check_voice_overlap` | silent | single-`channel_key` by construction (first line) |
| 15 | `project/manager.rs` · `ProjectManager::get_all_overlaps` | silent | correct machinery, wrong scope |
| 16 | `project/manager.rs` · `channel_display_name` | silent | rebuilds "FM6" from the index; drops "FM6/DAC" |
| 17 | `ipc/commands.rs` · `get_channel_overlaps` | silent + **dead** | zero frontend callers (grep in §3.3) |
| 18 | `export/smps.rs` · `generate_music_asm` | **wrong** | emits `smpsHeaderDAC` + `smpsHeaderFM` for index 5 |
| 19 | `export/smps.rs` · `validate_for_export` | silent | no cross-channel rule of any kind |
| 20 | `export/vgm.rs` · `export_vgm_data` | **wrong (other direction)** | `ChannelAssignment::Dac(_) => continue` — DAC dropped entirely |
| 21 | `ipc/commands.rs` · `export_wav` | **wrong** | shares `build_snapshot` + `AudioEngine` with preview |
| 22 | `ipc/commands.rs` · `export_vgm` | **wrong** + dead UI | inherits #20; unreachable today |
| 23 | `driver/flamedriver.rs` · `channel_layout` (`"FM6/DAC"`) | correct-ish | the only place the constraint is *named*; purely cosmetic |
| 24 | `model/driver.rs` · `DriverFeature` | silent | no variant expresses DAC/FM6 exclusivity |
| 25 | `src/components/AddTrackDialog.tsx` · channel `<select>` | silent | shows "FM6/DAC" verbatim; permits both |
| 26 | `src/components/TrackHeader.tsx` · `channelLabel` | **wrong** | renders "FM6", discarding the layout name |
| 27 | `src/components/ArrangementView.tsx` · `channelLabel` (duplicate of #26) | **wrong** | second copy of the same function |
| 28 | `src/components/TrackHeader.tsx` · `channelLevelIndex` + meter | **wrong** | mirrors #6 into the UI |
| 29 | `src/components/PianoRoll.tsx` · DAC sample-picker `title` (F25) | correct | true, but DAC-scoped; does not generalise |
| 30 | `src/components/PianoRoll.tsx` · `voiceHint` / `showVoiceHint` | correct surface, unused here | the reusable non-modal notice; 6 call sites |
| 31 | `src/components/ArrangementView.tsx` · (no notice element) | silent | documented gap; blocks (b)/(c) at arrangement level |
| 32 | `import/vgm_import.rs` · `ImportState::process_key_on` | **correct** | the only code in Seraph that models the steal |
| 33 | `import/vgm_import.rs` · `ImportState::process_ym2612` (`0x2B` arm) | **correct** | maintains `dac_enabled` |
| 34 | `import/smps_mapper.rs` · FM index assignment | silent / unverified | sequential `fm_idx`; could produce FM6 + DAC |

**Count: 34 surfaces. 16 wrong, 15 silent, 3 correct** (#23 counted as correct-ish is
listed under correct only for the layout name; it enforces nothing).

Of the 16 wrong: 11 are preview-side (and therefore fixed together by one change,
since preview and WAV export share the code path), 3 are UI-label/meter, and 2 are the
independent export defects.

---

## 7. What I could NOT establish, and why

1. **The driver's actual steal semantics** — deliberately out of scope per the dispatch; a sibling agent owns it. Everything in §5 option (a) that concerns *resumption* (does FM6 come back, when, re-keyed or mid-envelope) is unspecifiable until that lands. **BLOCKED by design, not by obstacle.**

2. **Whether real SMPS source songs produce FM6 + DAC through `smps_mapper`** (§2.4, surface #34). `map_smps_to_project` assigns FM indices sequentially from 0, so the answer depends on the driver's channel-table convention and on the corpus. Both are outside this investigation. Surface #34 is therefore marked *unverified*, not *wrong*.

3. **Runtime confirmation of anything.** No emulator (standing invariant), and I did not run the app or the build. In particular I did **not** empirically confirm that FM6 and a DAC sample audibly coexist in the running preview — the claim rests on reading `render`'s mix expression and the absence of any suppression. **TAGGED for the controller's foreground follow-up:** play a project with an FM6 sustain under a drum hit and confirm by ear; that is a 30-second check that would upgrade the central claim from read-verified to heard-verified.

4. **Test-suite baselines.** `docs/OVERSEER.md` records "cargo 264/0, vitest …" at the F25/F26 landing. I did **not** re-run either suite, so that figure is *documented, not measured by me*. Any "N new tests on top of 264" arithmetic should re-measure first.

5. **Absolute cost.** Every hour/scope judgement in §5 is judged, not measured, and this workspace's booked estimates have been wrong by up to 6x. The measured inputs (line counts, call-site counts, grep outputs) are stated inline so the next reader can re-derive rather than trust.

6. **Whether the NUL byte in `PianoRoll.tsx` (§0.1) has caused a *specific* prior miss.** I can show it makes the file invisible to default grep, and that it is the only such file; I cannot show which past conclusion it corrupted. Worth booking regardless — it is the exact "empty results are two-valued" failure the method constraints warn about, sitting in the tree as a permanent trap.
