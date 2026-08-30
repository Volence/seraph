# PITCH-FORMAT — Seraph's half of the joint plan

**Status:** draft for the aeon lane to review at its next boundary. **No code, no bytes.**
**Project:** SOUND-TRUTH. **Written:** 2026-08-30, seraph lane.

**Provenance, stated first because it governs how this should be read.** The owner's
choice — that the music driver stores *musical pitch* rather than raw chip numbers — reached
this lane **relayed off the aeon lane's board, not witnessed here**, and whether the item
belongs to this lane at all is still an open question on his row. **This document does not
assume the ruling and does not claim the work.** It states what Seraph needs under either
answer, so that confirming it costs him one sentence instead of a design conversation.

Every claim about Seraph below was read out of this tree today, at
`origin/main` `ce309ce`. Every claim about aeon is a **question**, not an assertion.

---

## 1. What Seraph stores today

**Seraph is already musical-pitch-native, and has been all along.** `model::song::Note`
carries:

| Field | Type | Meaning |
|---|---|---|
| `pitch` | `u8` | MIDI note number |
| `detune` | `i8` | offset from that note |

There is no chip register value anywhere in the song model. The conversion to hardware
happens at the edges, in `audio::frequency`:

- **FM:** `midi_to_fm_freq(midi) -> (block, fnum)`, a 12-entry semitone table with the octave
  as `block`.
- **PSG:** `midi_to_psg_period(midi) -> u16`, an 84-entry table covering **MIDI 36–119**.

**SMPS export already writes musical pitch, not numbers** — `export::smps::smps_note_name`
emits `nC4`, `nEb3` and so on. So on the export side, "musical pitch" is not a change of
direction for Seraph; it is what already ships.

## 2. The three things Seraph actually needs

Ordered by how much pain their absence causes. **None of these require the driver to store
musical pitch** — they require the driver's *choice* to be knowable and its tables to be
readable.

1. **The driver's frequency tables must be readable from driver source, not transcribed
   here.** This is the load-bearing requirement and it is independent of the ruling. Seraph
   currently carries its own `FM_FNUM_TABLE` and `PSG_PERIOD_TABLE`, the second commented
   *"matches Flamedriver/S3K Z80 driver exactly"* — a **transcription**, and therefore a
   thing that can silently drift from the driver it claims to match. The standing rule in
   this repo is parse constants from source at use time, never copy them. Whatever the
   ruling, Seraph should be reading Memra's tables rather than holding a copy.
2. **A stated pitch range per channel type, with stated out-of-range behaviour.** Seraph's
   PSG table covers MIDI 36–119 and **silently clamps outside it**: below 36 it returns
   `1023`, above 119 it returns `0`. Both are effectively "no note". Today nothing warns the
   author. If the driver's range differs, songs that sound correct in the app go silent in
   the game, which is this lane's standing anti-pattern (the preview must not promise what
   the driver cannot do).
3. **A stated home for transposition.** If a song is transposed, does the stored data move,
   or does the driver apply an offset at play time? This decides whether Seraph can offer
   non-destructive transposition at all.

## 3. What each answer costs, honestly

**If the driver stores musical pitch (a note index):**

- *Gains:* transposition, key changes and instrument swaps stay musical end to end.
  Re-importing a song recovers what the author wrote, not an approximation. Seraph's model
  maps to it directly, so the app's export becomes a near-identity rather than a conversion.
- *Costs, and they are aeon's to weigh, not this lane's:* the driver does a table lookup per
  note-on, spending Z80 cycles in the sound update and ROM bytes on the tables. This lane
  has **no measurement** of either and will not guess at them.

**If the driver stores raw chip numbers:**

- *Gains:* no lookup at play time, no table in ROM.
- *Costs, and this half IS measurable from Seraph's side and is already visible in this
  tree:* **the reverse conversion is lossy and the loss is not theoretical.**
  `fm_freq_to_midi` picks the nearest table entry and returns the remainder as `detune` —
  that residual exists precisely because the mapping does not invert cleanly.
  `psg_period_to_midi` likewise picks a nearest match. So a song round-tripped through raw
  numbers comes back as *nearest note plus a correction*, not as what was written.
  Transposition must then be pre-baked into the data, and any later edit re-bakes it.

**Seraph's recommendation: musical pitch, for the reason the round-trip already
demonstrates rather than on principle.** The evidence is that this tree already contains the
lossy inverse and already has to model its residual as a per-note `detune` field. That field
is the cost of raw numbers, made concrete. But **the cycle and ROM costs sit entirely on
aeon's side and this lane has not measured them**, so this is a recommendation offered with
one half of the ledger filled in, and it should lose to a measured objection.

## 4. Exactly what aeon must answer

Phrased so each can be answered from driver source, and none presumes the ruling:

1. **What does Memra's note-on take** — a musical note index, or a raw `(block, fnum)` for FM
   and a period for PSG? If it differs per channel type, state each.
2. **Where do the frequency tables live in Memra's source, and can Seraph read them at build
   time?** Name the file and symbol. This is the request that matters most and it is
   independent of question 1.
3. **What is the playable range per channel type, and what does the driver do outside it** —
   clamp, wrap, silence, or refuse? Seraph needs the exact behaviour to warn the author
   instead of letting a song go silent in the game.
4. **Is detune a first-class field in the driver, and in what units?** Seraph stores `i8` per
   note; if Memra has no equivalent, Seraph must know what to do with a detuned note at
   export rather than dropping it silently.
5. **Where does transposition happen** — baked into song data, or applied by the driver at
   play time?
6. **Does channel 6 change any of this?** Under owner ruling d-7 the driver profile carries
   the channel-6 behaviour; this lane wants to know whether pitch handling differs when FM6
   is a music voice versus the DAC.

## 5. What this document deliberately does not do

- **It does not claim the work.** Whether PITCH-FORMAT is seraph's or aeon's is the owner's
  open question, untouched here.
- **It does not assume the ruling.** Section 2's requirements hold under either answer.
- **It does not size the change.** Sizing needs answers to §4 first, and this lane's booked
  sizes have been wrong twice today in the direction of optimism (F29 booked M, actually L;
  F32 booked as one defect, actually two).
- **It states no aeon-side number.** No cycle counts, no ROM figures. Those are aeon's
  measurements to make, and this lane's standing rule is to prefer a measured claim over a
  booked one and to make neither on someone else's tree.
