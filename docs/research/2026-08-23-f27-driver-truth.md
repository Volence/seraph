# F27 — what the driver actually does with `$2B` / FM6

**Question.** Seraph's preview treats the DAC as an independent stream summed into the
mix and never writes YM2612 register `$2B`, so an FM6 track and a DAC track can sound
together in the preview. The chip cannot do that. This report establishes, from source,
what aeon's driver does, what the field does, and what that licenses for the fix.

**Revision anchors.** Every aeon citation below was read via
`git show <sha>:<path>` at aeon `origin/master` =
**`139995f256f5e50c26d2053c229dd09b5e70c84d`**, re-verified unchanged at 2026-08-23T08:06Z
(fetched before and after the read pass). No aeon working-tree file was read.
S3K citations are from `skdisasm` at `2fcd861c208f342b6d14df694c6422c74f20a4be`.
Seraph citations are from this worktree (branch `research/f27-driver-truth`).

Symbols, not line numbers, are the durable anchors. Line numbers appear only where a
`.lst` disassembly's own address column is the identifier.

---

## 1. What aeon's own driver does

### 1.1 The short answer

aeon **does** write `$2B`, at four distinct moments, and it has a **three-mode per-song
contract** for who owns chip channel 6. The collision is not "unrepresented" — it is
modelled explicitly, in the song header, and enforced partly in the driver and partly in
the packer. But the enforcement has a hole (§1.6).

### 1.2 The three modes are declared in the song header

`engine/sound/sound_constants.emp`, the `SH_FLAGS` block:

```
// bit0 SH_F_FM6_FM       : FM6 is a 6th FM SEQUENCER voice (DAC mode OFF, $2B=$00).
// bit1 SH_F_STREAM       : RESERVED — always SET (the packer force-sets it).
// bit2 SH_F_FM6_ADAPTIVE : FM6 TIME-SHARES ch6 with the DAC (requires SH_F_FM6_FM).
pub const SH_F_FM6_FM_B   = 0
pub const SH_F_STREAM_B   = 1
pub const SH_F_FM6_ADAPTIVE_B = 2
```

and the route enum in the same file states the constraint as a comment on
`CHROUTE_FM6`:

```
// FM6 is a routable FM voice (part II, ch-in-part 2, chsel $06), inserted
// contiguously after FM5; the PSG/DAC routes shift up by one. FM6 and the DAC are
// mutually exclusive on ch6 (shared via $2B bit7).
...
pub const CHROUTE_FM6 = 5    // 6th FM voice (part II ch2, chsel $06) — DAC-off songs
...
pub const CHROUTE_DAC  = 10  // emits $E2 DAC triggers only
```

So: **DEDICATE** (neither flag — ch6 is the DAC), **FM6-FM** (`SH_F_FM6_FM` — ch6 is a
sixth FM voice, DAC off), **ADAPTIVE** (`SH_F_FM6_FM|SH_F_FM6_ADAPTIVE` — ch6 time-shares).

Only the ADAPTIVE bit reaches the Z80. `engine/sound/sound_api.emp`, header comment on
`Sound_PlayMusic`:

> The flags are forwarded because the Z80 loader reads the adaptive-FM6 bit
> (`SH_F_FM6_ADAPTIVE`) from the param block during the header parse (`SH_F_FM6_FM` is
> materialized at pack time; **never read at runtime**).

That sentence is the crux of §1.6.

### 1.3 When `$2B` is written — all four sites

**(a) Driver init, once.** `engine/sound/z80_sound_driver.emp`, `z80_init`'s YM bring-up:

```
        // DAC ENABLE ONCE: $2B = $80 (DAC mode on), then SELECT $2A once, then
        // PRIME the $2A latch to $80 (DC center). After this the addr port stays
        // parked on $2A forever, so every `ld (de),a` writes DAC data. $2B is
        // NEVER toggled again (no per-play enable edge -> no click).
        ld      (hl), SND_REG_DAC_ENABLE // $4000 = $2B (select DAC-enable reg)
        ld      a, $80
        ld      (de), a                 // $4001 = $80 -> DAC mode ON
```

(The "NEVER toggled again" claim in that comment is **stale** — three more write sites
follow. Flagging it as a doc defect in the driver, not a mechanism.)

**(b) Every song load, unconditionally OFF.** `Snd_LoadSong`, same file:

```
        // 1. DAC mode OFF: $2B = $00. ABSOLUTE addressing (preserve de=$4001), then
        //    re-park $2A. ...
        ld      a, SND_REG_DAC_ENABLE   // $2B
        ld      (SND_Z80_YM_A0), a      // select $2B on $4000
        xor     a
        ld      (SND_Z80_YM_A1), a      // $4001 = $00 -> DAC mode OFF
```

This is not gated on any flag. **Every** song load hands ch6 back to FM.

**(c) Every sample start, ON.** `Snd_StartSample`, same file — with an FM6 key-off first
when ADAPTIVE:

```
        ld      a, (SND_FM6_ADAPTIVE)
        or      a
        jr      z, .ss_no_fm6_keyoff
        ld      a, SND_REG_KEY_ONOFF    // $28 (key on/off, part I)
    .ss_keyoff_addr:
        ld      (SND_Z80_YM_A0), a
        ld      a, 6                    // chsel FM6 = $06, op-mask 0 -> key OFF
    .ss_keyoff_data:
        ld      (SND_Z80_YM_A1), a
    .ss_no_fm6_keyoff:
        // Re-assert DAC mode ($2B bit7), then re-park $2A. (One-time per trigger,
        // not per loop -> no recurring click.)
        ld      a, SND_REG_DAC_ENABLE
    .ss_dacen_addr:
        ld      (SND_Z80_YM_A0), a      // $4000 = $2B (select DAC-enable reg)
        ld      a, $80
    .ss_dacen_data:
        ld      (SND_Z80_YM_A1), a      // $4001 = $80 -> DAC mode ON
```

Note the asymmetry: the **key-off is gated** on `SND_FM6_ADAPTIVE`; the **`$2B=$80` is
not**. Any sample start takes ch6, in any mode.

Both call contexts reach this: the sequencer's `$E2` opcode
(`sound_sequencer.emp::Seq_Op_Dac` → `Seq_HookDac` → `Snd_StartSample`) and the 68k
mailbox (`z80_sound_driver.emp::SndDrv_PollMailbox`, `ld a, (SND_REQ_SAMPLE)` →
`Snd_DacLookup` → `Snd_StartSample`). `Sound_PlaySample` in `sound_api.emp` posts that
mailbox slot, so a **game SFX sample** enters by the same door as a music drum.

**(d) Sample stop — OFF, but only when ADAPTIVE.** The `.stop` arm of the streaming loop:

```
    .stop:
        call    Snd_ParkDac
        ld      a, $80
        ld      (SND_Z80_YM_A1), a      // $2A = $80 (DC center silence)
        ...
        ld      a, (SND_FM6_ADAPTIVE)
        or      a
        jr      z, .stop_done
        ld      a, SND_REG_DAC_ENABLE   // $2B
    .stop_dacoff_addr:
        ld      (SND_Z80_YM_A0), a
        xor     a
    .stop_dacoff_data:
        ld      (SND_Z80_YM_A1), a      // $2B = $00 -> ch6 returns to FM (output was centered)
        ld      ix, (SND_FM6_CHAN_PTR)
        ...
        bit     SCF_KEYED_B, (ix+sc_flags)
        jr      z, .stop_repark         // FM6 resting -> leave it silent
        ld      a, SND_REG_KEY_ONOFF    // $28 (key on/off, part I)
    .stop_rekey_addr:
        ld      (SND_Z80_YM_A0), a
        ld      a, SND_FM_KEYON_OPMASK|6 // $F0 | chsel(FM6 = $06) = $F6 -> key ON (all ops)
```

So in ADAPTIVE mode ch6 is handed back and FM6's held note is re-keyed with a real 0→1
EG edge. **In every other mode `$2B` stays `$80` after the first sample, for the rest of
the song.**

### 1.4 What happens to an FM6 note that STARTS while the DAC is enabled

It is **silently swallowed at the driver's single key-on chokepoint**.
`engine/sound/sound_fm.emp`, inside `Fm_NoteOn` at `.keyon`:

```
        // --- FM6 dedicate (Layer 4): while a DAC sample owns ch6 (SND_STAT_DAC_ACTIVE),
        // $2B bit7 makes the DAC REPLACE FM6's output — a $28 key-on would only
        // retrigger a silenced EG. Skip the chip key-on, but still set SCF_KEYED so
        // the sequencer's bookkeeping stays consistent. This is the SINGLE chokepoint
        // for every FM key-on, so gating here covers them all.
        ld      a, (ix+sc_route)
        cp      CHROUTE_FM6
        jr      nz, .do_keyon            // not FM6 -> key on normally
        ld      a, (SND_STAT_DAC_ACTIVE)
        or      a
        jr      z, .do_keyon             // DAC idle -> FM6 keys on normally
        set     SCF_KEYED_B, (ix+sc_flags) // DAC owns ch6: advance bookkeeping, no chip $28
        jp      Snd_ParkDac
```

This is a **silent steal with consistent bookkeeping**: no `$28` reaches the chip, but
`SCF_KEYED` is set so the sequencer's state machine (and, in ADAPTIVE mode, the `.stop`
re-key that reads exactly that bit) stays coherent. Nothing is reported, logged, or
surfaced. `SND_STAT_DAC_ACTIVE` is not in the debug mirror path examined here.

The suppression is keyed on `SND_STAT_DAC_ACTIVE`, **not** on `$2B`. Those diverge — see
§1.6.

### 1.5 What happens to an FM6 note ALREADY sounding when a DAC sample starts

- **ADAPTIVE**: explicitly keyed off (§1.3c) before `$2B=$80`, so the exhaust re-key is a
  true EG edge; then restored at `.stop` if `SCF_KEYED`. A rest stays silent
  (`jr z, .stop_repark`). This is a genuine, click-managed time-share.
- **Any non-ADAPTIVE mode**: no key-off. The FM6 EG keeps running; the chip simply stops
  routing its output (`$2B` bit 7 replaces ch6's output with the DAC latch — this is the
  driver's own statement in the `Fm_NoteOn` comment above, not a datasheet reading of
  mine). Because `.stop` skips the `$2B=$00` restore in this mode, **the note never comes
  back**.

`Sequencer_StopAll` is explicit that it does not intervene: *"Does NOT touch `$2B` (DAC
enable)."* So even a StopMusic does not reclaim ch6; only the next `Snd_LoadSong` does.

### 1.6 Is FM6 reserved / excluded from allocation? Partly. Here is the hole.

**SFX allocation: yes, hard-excluded.** `engine/sound/sound_sfx.emp`, `SfxEligTable`:

```
        SFXEL_NONE,   // CHROUTE_FM6  (5) — reserved v1 (DAC / DAC-off FM)
```

with the design note in the same file:

> (c) FM6<->DAC MUTUAL EXCLUSION IS MOOT. FM6 is SFXEL_NONE in SfxEligTable, so no
> SFX ever steals FM6 — there is no path where an SFX and the DAC contend for
> it. (Opening FM6 to SFX for DAC-off songs is a one-byte table edit, 5b.)

**Music allocation: no.** The packer, `tools/song_packer.py::pack_song`, validates
exactly two things relevant here:

```python
    if (flags & SH_F_FM6_ADAPTIVE) and not (flags & SH_F_FM6_FM):
        raise PackError("SH_F_FM6_ADAPTIVE requires SH_F_FM6_FM (FM6 must be an FM voice to time-share with the DAC)")
    ...
    routes = [ch.route for ch in song.channels]
    if len(set(routes)) != len(routes):
        ... raise PackError("duplicate channel route(s) ...")
```

and restricts the `$E2` trigger to the DAC route, `song_packer.py::Dac.validate`:

```python
    def validate(self, route):
        if route != CHROUTE_DAC:
            raise PackError(f"Dac on non-DAC route {route}")
```

**There is no check that a `CHROUTE_FM6` channel and a `CHROUTE_DAC` channel do not
coexist, and no check that a `CHROUTE_FM6` channel implies `SH_F_FM6_FM`.**
`CHROUTE_FM6` and `CHROUTE_DAC` are distinct route ids (5 and 10), so the duplicate-route
check does not fire. I grepped `song_packer.py` for every `FM6` / `SH_F_FM6` occurrence;
the only two are the constant definitions and the ADAPTIVE⇒FM6_FM implication above.

**So the format DOES permit authoring both.** Two concrete malformed-but-packable songs:

1. `flags = SH_F_FM6_FM` (no ADAPTIVE), channels include both an FM6 melody and a DAC
   drum track. Packs clean. On hardware: `Snd_LoadSong` sets `$2B=$00`, FM6 plays; the
   first `$E2` sets `$2B=$80` and — because `SND_FM6_ADAPTIVE` is 0 — **never clears it**.
   FM6 is dead for the rest of the song. Worse, once the sample finishes,
   `SND_STAT_DAC_ACTIVE` is cleared at `.stop_done`, so `Fm_NoteOn`'s suppression stops
   firing and subsequent FM6 note-ons **do** write `$28` to a channel whose output the
   chip has replaced. The driver believes FM6 is playing; the chip emits nothing.
2. `flags = SH_F_FM6_FM`, DAC-free song, but the **game** calls `Sound_PlaySample` for an
   SFX. Same outcome by the mailbox path — and this one cannot be caught at pack time at
   all, because it is a runtime 68k call, not song data.

Case 1 is the one Seraph can act on. Case 2 is an engine-side gap I am recording but not
proposing to fix here.

*(The chip-level consequence in case 1 — "`$28` writes reach a channel whose output is
replaced" — is inference from combining `Snd_LoadSong`/`Snd_StartSample`/`.stop`/
`Fm_NoteOn` with the driver's own `$2B` bit-7 description. I did not observe it running;
see §5 for the emulator TAG.)*

---

## 2. What established drivers do

I read four Z80 blobs' paired `.lst` disassemblies directly, plus S3K's commented driver
source, plus what the in-repo docs *assert* about MDSDRV. I mark which is which.

### 2.1 Read directly from disassembly

**Batman (`docs/research/z80_blobs/batman_z80.lst`) — toggled every scheduler pass from a
state flag, AND ch6's FM voice excluded from the update while active.** Init at `$0022`:

```
0022  DD 36 00 2B  ld (ix+0),$2B
0026  DD 36 01 00  ld (ix+1),$00      ; $2B = $00 -> DAC disabled at boot
```

Main scheduler loop at `$006E`:

```
006E  3A EB 00     ld a,($00EB)       ; RAM "sample active" flag ($80 or $00)
0071..0075         busy-wait
0077  DD 36 00 2B  ld (ix+0),$2B
007B  DD 77 01     ld (ix+1),a        ; $2B <- the flag, EVERY pass
007E  FE 80        cp $80
0080  28 14        jr z,$0096         ; active -> enter the DAC feed coroutine
```

And in `Update_FM_B` at `$0130`, the ch6-mapped voice's chip update is **skipped**
whenever the flag says a sample is playing:

```
0134  06 04        ld b,$04
0136  CD 52 0B     call $0B52         ; Channel_Load, channel 4
0139  3A EB 00     ld a,($00EB)
013C  FE 80        cp $80
013E  28 09        jr z,$0149         ; DAC active -> SKIP channel 5 entirely
0140  FD 21 83 16  ld iy,$1683
0144  06 05        ld b,$05
0146  CD 52 0B     call $0B52         ; Channel_Load, channel 5 (ch6)
```

This is allocation-level exclusion, decided per sub-frame. The in-repo
`batman_driver_analysis.md` reaches the same reading (its §4 "channel 5 only if no DAC
active (`$00EB≠$80`)", and §11 "DAC steals FM channel 6 (only 5 FM voices while a sample
plays)") — I confirmed it against the bytes rather than taking it on the doc's word.

**Alien Storm (`alien_z80.lst`) — toggled per sample.** `$2B=$00` at init (`0034`) and at
the sample-end path (`06F0`, immediately after a `$B6` pan write); `$2B=$80` at two
sample-start sites (`00FC`, `021B`).

**Gunstar Heroes (`gunstar_z80.lst`) — toggled per sample.** The main loop polls a 68k
command byte at `$1FFF` and, on a request, does `ld a,$80 / ld hl,$4000 / ld (hl),$2B /
inc hl / ld (hl),a` (`0038`-`0043`); the stop path writes `$2B=$00` (`0159`).

**Thunder Force IV (`tf4_z80.lst`) — toggled per sample, via a helper.** The register
literal never appears as `ld (hl),$2B`, which is why a naive grep reads as "never writes
it". It goes through the busy-wait YM writer at `$04BE` (`ld a,($4000) / jp m` spin, then
`ld a,h / ld ($4000),a`), with the register in `h`:

```
0208  3E 80        ld a,$80
020A  32 02 00     ld ($0002),a
020D  26 2B        ld h,$2B
020F  CD BE 04     call $04BE     ; $2B = $80 -> DAC on at sample start
...
0235  AF           xor a
0236  32 02 00     ld ($0002),a
0239  26 2B        ld h,$2B
023B  C3 BE 04     jp $04BE       ; $2B = $00 -> DAC off at sample end
```

**Sonic 3 & Knuckles "Flamedriver" (`skdisasm/Sound/Z80 Sound Driver.asm` @ `2fcd861`) —
toggled per sample, and FM6-as-music does not exist.** `zPlayDigitalAudio` disables the
DAC on entry and spins idle until a sample is queued, then enables it:

```
zPlayDigitalAudio:
        di
        ld      a, 2Bh                          ; DAC enable/disable register
        ld      c, 0                            ; Value to disable DAC
        call    zWriteFMI
.dac_idle_loop:
        ei
        ...
        ld      a, (zDACIndex)
        or      a
        jr      z, .dac_idle_loop
        ld      a, 2Bh
        ld      c, 80h                          ; Value to enable DAC
        di
        call    zWriteFMI
```

and the track model collapses the two into one slot — `zSongFM6_DAC: zTrack`, dispatched
as `ld ix, zSongFM6_DAC / bit 7,(ix+zTrack.PlaybackControl) / call nz, zUpdateDACTrack` —
with the init table carrying the comment `db 80h, 6 ; FM6 music track (does not exist in
this driver)`.

### 2.2 Asserted by an in-repo doc, NOT read by me

**MDSDRV.** `docs/research/2026-08-07-mdsdrv/core.md` §0.1, marked `[V]` (the doc's own
verified-against-source marker), states the channel-id enum is
`00-05` FM1-FM6/**PCM1**, `06-08` PSG1-3, `09` PSG noise, `0a-0f` dummy/PCM2/PCM3 —
i.e. **slot 5 is FM6-or-PCM1, one physical voice id, so the format cannot express both.**
I could not verify this against MDSDRV source: `docs/research/external/` is **empty** at
this SHA (`git ls-tree -r origin/master docs/research/external/` returns nothing), so
every `mdsdrv.68k:` / `mdsseq.md:` citation in those docs is unreproducible from the repo.
Treat as second-hand.

**`docs/research/dac-driver-redesign-synthesis.md`** is a recommendation document, not a
reading of any shipped driver. It argues *for* permanent DAC-on and names the cost
explicitly — and this is the sentence that most directly frames F27:

> Idle DC-center streaming (feeding `$80` when idle) keeps the DAC channel always-on,
> **stealing FM6 permanently**; confirm FM6 is not needed for music, or only stream `$80`
> idle while a DAC voice is actually allocated.

and leaves it as an open question:

> Should idle output a continuous `$80` DC-center stream (permanent DAC-on, click-free,
> but steals FM6) or fully stop the DAC between samples (frees FM6 for music, but
> reintroduces enable-edge management)? Depends on whether FM6 is used by the music
> engine — defer to user / music-sequencing design.

aeon's shipped driver answered this **in the middle**: enable-once at init, then off at
every song load, on at every sample start, off at sample stop *only in ADAPTIVE mode*.

### 2.3 The convention, stated plainly

**Every driver I read the bytes of — Batman, Alien Storm, Gunstar, TF4, S3K — turns the
DAC OFF when no sample is playing.** None keeps `$2B=$80` permanently. The variation is
in granularity (per-sample edge vs. per-sub-frame re-assert from a flag) and in how the
FM6 voice is handled while stolen (Batman: skip its update entirely; S3K: the voice does
not exist; aeon ADAPTIVE: key-off + re-key). **Nobody lets FM6 and the DAC sound
simultaneously**, and no driver I read represents that state at all.

### 2.4 Two docs that turned out not to bear on the question

- `docs/research/2026-08-10-s3k-dac-kit-survey.md` is a **sample-asset** inventory (WAV
  ids, pitch multipliers, banking size). No channel-arbitration content.
- `docs/research/2026-08-07-mdsdrv/z80-dma.md` is about DMA-window timing and DAC feed
  budgets. It confirms MDSDRV "keeps feeding the DAC from its RAM ring" during DMA waits
  but says nothing about `$2B` or FM6 ownership.

---

## 3. What this implies for Seraph

### 3.1 Where Seraph stands today (verified in this worktree)

- `grep -rniE '0x2b|\$2b|dac_enable|dacEnable' src-tauri/src/audio src-tauri/src/sequencer src-tauri/src/dac`
  → **exit 1** (ran, zero matches). Control: `grep -rn '0x28' src-tauri/src/audio/engine.rs`
  → exit 0, 9 matches. So the finding's premise holds and is not a broken command.
- `src-tauri/src/audio/engine.rs::AudioEngine::render` sums the DAC as a fully independent
  voice: `let dac_mix = dac_sample as f32 * 48.0 / 65536.0;` then
  `let pre_l = (fm_l + dac_mix + psg_mix) * self.master_volume;`. Nothing consults FM
  channel 6's state; nothing mutes it.
- The **only** driver profile Seraph ships is `FlamedriverProfile` (S3K) —
  `src-tauri/src/driver/flamedriver.rs`. Its `channel_layout()` offers **six** FM channels
  *and* a DAC channel at once:
  `FmChannelInfo { index: 5, name: "FM6/DAC".into(), ... }` alongside
  `dac_channels: vec![DacChannelInfo { index: 0, name: "DAC".into() }]`.
  The name already knows about the sharing; the layout contradicts it. There is no aeon
  profile.
- `ChannelAssignment` (`src-tauri/src/model/song.rs`) is `Fm(u8) | Psg(u8) | PsgNoise |
  Dac(u8)` with **no validation** of the FM6/DAC pair anywhere in `src-tauri/src`
  (`grep -rniE 'fm6|FM ?6|channel ?6' src-tauri/src --include='*.rs'` returns exactly the
  one `flamedriver.rs` line above).
- VGM export (`src-tauri/src/export/vgm.rs`) does `ChannelAssignment::Dac(_) => continue`
  — DAC channels are dropped from the export entirely and no `$2B` is emitted. (The
  `out[0x2B] = 0` there is a VGM **header byte offset**, not a YM register — do not
  conflate them.) So an exported VGM of an FM6+DAC song silently loses the drums and
  keeps a melody that the real chip would mute. That is a second, separate divergence.

### 3.2 The rule, stated so it can be implemented

> **Chip channel 6 has exactly one owner at any instant, selected by YM2612 `$2B` bit 7.**
> While bit 7 is set the DAC replaces FM6's output; while it is clear FM6 sounds and the
> DAC is silent. A tool must therefore either (i) never let a project hold both an FM6
> track and a DAC track, or (ii) model an explicit ownership timeline and render FM6 as
> muted for every interval the DAC owns ch6.

For the **aeon** target the timeline is fully determined by the song header:

| mode | header flags | `$2B` timeline | FM6 |
|---|---|---|---|
| DEDICATE | (none) | `$00` at load; `$80` from the first sample onward, forever | not a music voice; any FM6 track is invalid |
| FM6-FM | `SH_F_FM6_FM` | `$00` at load; **must stay `$00`** | a real 6th FM voice; a DAC trigger permanently breaks it |
| ADAPTIVE | `SH_F_FM6_FM \| SH_F_FM6_ADAPTIVE` | `$00` at load, `$80` for the duration of each sample, `$00` on drain | audible **only** outside sample intervals; key-off/re-key at each edge |

For the **Flamedriver (S3K)** target — the profile Seraph actually ships — there is no
timeline to model: FM6-as-music does not exist in that driver (`db 80h, 6 ; FM6 music
track (does not exist in this driver)`). The correct layout has five FM channels plus a
DAC, not six plus a DAC.

### 3.3 Pricing the three options

**"Silent steal" (preview mutes FM6 while the DAC sounds; nothing is said).**
This is *exactly* what aeon's `Fm_NoteOn` chokepoint does, so it is the highest-fidelity
model of the hardware for ADAPTIVE songs, and it is cheap: gate the FM6 voice's
contribution to `fm_l/fm_r` on `dac_samples.is_some()`. **But it is wrong as the whole
answer**, for two reasons the source establishes:
1. For the **Flamedriver profile Seraph ships**, FM6-as-music is not representable at all
   — silently muting it makes the preview honest while leaving the tool offering a track
   that can never exist. The bug is in `channel_layout()`, and silent muting hides it.
2. For aeon, silence is only correct in ADAPTIVE mode. In FM6-FM mode the true hardware
   outcome is **permanent** loss of FM6 from the first drum onward, not a per-hit duck. A
   preview that ducks FM6 per hit and restores it would *sound better than hardware* and
   send the author home with a broken song. Modelling this correctly requires knowing the
   song's mode, which Seraph currently has no field for.

**"Visible warning" (preview still mutes, plus a surfaced diagnostic).**
This is the option the source most supports as a *component*, because the divergence is
otherwise undetectable: the driver's own suppression path deliberately keeps bookkeeping
consistent (`set SCF_KEYED_B` with no chip write), and `Sequencer_StopAll` explicitly
declines to touch `$2B`. Nothing at runtime reports the steal. A tool is the only place
the author can learn. Cost is a diagnostic surface plus the same render gate as above.

**"Authoring-time gate" (refuse the combination).**
This is what the field does. S3K collapses FM6 and DAC into one track slot; MDSDRV
(doc-asserted) uses one channel id for FM6/PCM1; aeon's packer *intends* to gate it and
has the flag vocabulary for it. It is also the only option that makes VGM export coherent
(§3.1). Cost: it forbids ADAPTIVE, which is a real, shipped aeon capability with a
working song (`games/sonic4/data/sound/song_drumtest.py` uses
`SH_F_STREAM | SH_F_FM6_FM | SH_F_FM6_ADAPTIVE`). A blanket gate would make aeon's most
interesting ch6 mode unauthorable in Seraph.

**What the driver's answer rules out.** A pure silent steal with no surfaced signal is
wrong, because the hardware outcome is mode-dependent and one of the modes is
*irreversible*; a preview that models it as a recoverable per-hit duck actively misleads.
A blanket authoring-time gate is also wrong, because ADAPTIVE exists and works. The
defensible shape is: **profile-declared ch6 policy**, with the gate applied where the
profile says FM6 is unavailable (Flamedriver, aeon DEDICATE/FM6-FM) and the modelled
mute-with-diagnostic applied where it is time-shared (aeon ADAPTIVE). That requires
Seraph to grow a song-level ch6-mode field, which it does not have.

### 3.4 Smaller fixes this uncovered, independent of the F27 decision

1. `FlamedriverProfile::channel_layout()` advertises FM6 *and* DAC. For the S3K driver
   that is unconditionally wrong regardless of how F27 is resolved.
2. VGM export drops `ChannelAssignment::Dac(_)` silently. An exported song loses its
   drums with no diagnostic.
3. aeon-side (not Seraph's to fix, worth reporting upstream): `Snd_StartSample`'s
   `$2B=$80` is ungated while `.stop`'s `$2B=$00` is gated on `SND_FM6_ADAPTIVE` — so a
   `Sound_PlaySample` SFX during an `SH_F_FM6_FM` song permanently kills FM6, and the
   packer cannot catch it because it is a runtime 68k call. Also: `song_packer.py` does
   not reject an `FM6 + DAC` channel pair without `SH_F_FM6_ADAPTIVE`, and the
   "`$2B` is NEVER toggled again" comment in `z80_init` is stale (three later write sites).

---

## 4. What this rules in and out for the F27 fix

**Ruled IN**
- Muting FM6's contribution in the preview mixer whenever the DAC stream is active is
  hardware-faithful and matches aeon's own `Fm_NoteOn` chokepoint. Do it.
- Fixing `FlamedriverProfile::channel_layout()` — five FM + DAC, or an explicit
  "FM6 unavailable while DAC is in use" declaration. Independent of everything else.
- A surfaced diagnostic when a project holds both an FM6 track and a DAC track. The
  drivers give the author no runtime signal; the tool is the only place to say it.

**Ruled OUT**
- Silent muting **as the entire fix**. The hardware consequence is mode-dependent and, in
  aeon's FM6-FM mode, irreversible; a per-hit duck models it as recoverable and is
  therefore worse than saying nothing.
- A blanket authoring-time refusal of FM6 + DAC. aeon's ADAPTIVE mode ships, works, and
  has a test song; forbidding it would remove a real capability.
- Any fix that assumes a single ch6 policy across profiles. Flamedriver and aeon differ,
  and aeon differs from itself across three header modes.

**Needs a decision that source cannot make**
- Whether Seraph grows a song-level ch6-mode field (mirroring `SH_F_FM6_FM` /
  `SH_F_FM6_ADAPTIVE`) or defers by treating ch6 as DAC-only for now. Everything above
  prices both; neither is forced by the driver.

---

## 5. What I could NOT establish, and why

1. **Whether real hardware / an accurate emulator actually behaves as §1.6 case 1
   predicts** (post-sample FM6 note-ons writing `$28` into a channel whose output `$2B`
   has replaced, in a non-ADAPTIVE aeon song). This is inference from four code paths plus
   the driver's own comment; I did not observe it. **TAGGED for the controller's
   foreground follow-up** — per standing invariant 1 I did not touch any emulator tool.
   The cheap check is a VGM/register trace of a song packed with `flags = SH_F_FM6_FM`,
   an FM6 melody, and one `$E2` drum.
2. **MDSDRV's actual source.** `docs/research/external/` is empty at
   `139995f2` (`git ls-tree -r origin/master docs/research/external/` → no output,
   exit 0). Every MDSDRV claim in §2.2 is the in-repo doc's `[V]`-marked assertion, which
   I could not reproduce. If the FM6/PCM1 single-slot claim ends up load-bearing for the
   fix, it needs the upstream source.
3. **Whether S3K songs ever drive ch6 as a music voice in practice.** The driver source
   says the FM6 music track "does not exist in this driver" and the track slot is
   `zSongFM6_DAC`, which I read as decisive — but I did not survey the ~50 song files in
   `skdisasm/Sound/Music/` to confirm no song assigns ch6 melodically. Low risk, not
   checked.
4. **Whether Seraph's sequencer has any per-song mode concept I missed.** I grepped
   `src-tauri/src` for `fm6|FM ?6|channel ?6` and found exactly one hit (the
   `flamedriver.rs` label). I did not read the whole sequencer; a mode field under some
   other name could exist, though `ChannelAssignment` having no such variant argues
   against it.
5. **The `.bin` blobs themselves.** Per instruction I read only the paired `.lst` text.
   `tf4_z80_alt.bin` has **no** paired `.lst` at this SHA, so that variant is unread.
   Batman/Alien/Gunstar/TF4 findings rest on the committed disassemblies, which are
   `z80dasm.py` output and could in principle mis-decode a data region as code; the `$2B`
   sites I cite all sit in plainly code-shaped context, so I consider the risk low but not
   zero.
