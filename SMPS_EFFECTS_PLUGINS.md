# SMPS Effects / Sequencer Plugins

These are not audio DSP plugins — they are **sequencer transforms** that modify note data at playback time and emit the corresponding SMPS coordination flags on export. The Genesis has no plugin system; these effects map 1:1 to existing SMPS commands.

## Proposed Effects

### Staccato Gate (smpsNoteFill)
- Parameter: fill duration (in SMPS ticks, 0 = off)
- Playback: truncates note audible duration to min(note_duration, fill)
- Export: emits `smpsNoteFill $XX` before the affected region
- Example: ICZ1 PSG channels use `smpsNoteFill $09` for rhythmic staccato sections

### Pitch Slide (smpsPitchSlide)
- Parameters: slide speed, direction
- Playback: gradually bends pitch between notes
- Export: emits `smpsPitchSlide` coordination flag

### Vibrato (smpsModSet)
- Parameters: wait, speed, delta, steps
- Playback: already partially implemented as NoteModulation on individual notes
- Export: emits `smpsModSet $wait, $speed, $delta, $steps`
- Could be promoted from per-note to a track-level effect

### Volume Fade (smpsFMAlterVol)
- Parameters: delta per step, step count
- Playback: gradual volume ramp over time
- Export: emits `smpsFMAlterVol` in a loop pattern

## Architecture

- Tracks get an optional list of effects, each with a region (start tick, end tick) and parameters
- Sequencer applies effects during playback (modifying note events before they hit the audio engine)
- Exporter translates effects back to SMPS coordination flags at the correct positions
- UI: effect lanes below each track, or a per-track effect list in the inspector

## Key Principle

Everything maps to real SMPS commands. No effect should exist that can't be exported. This keeps the tool honest — what you hear is what the Genesis will play.
