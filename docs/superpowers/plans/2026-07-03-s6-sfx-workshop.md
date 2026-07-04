# S6 — SFX Workshop Implementation Plan (incl. spec §)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. Repo: seraph, branch `feat/s6-sfx-workshop`. Depends: S1 (model kinds) + S3 (audition); engine package 2 (Stage B/C header fields) must be EXECUTED in aeon for the full header form — build the form from the manifest feature flags so missing fields grey out, exactly like S4 plugins.

## Spec § (design — approved in the master design)

Project kinds Song/Jingle/SFX share instruments/roll/compiler; SFX kind =
short score + SFX header metadata + a workflow tuned for iteration speed.
Components: **`SfxEditor`** single-screen mode (1–3 lanes per the SFX voice
cap from the manifest, bar-limited timeline, space = instant recompile+replay
through the S3 driver engine via the SFX-tier mailbox command — NOT the music
path); **archetype templates** (jump/ring/explosion/splash/skid/charge:
pre-wired patch+sweep+envelope starting points stored as bundled template
projects); **generator plugins** (pitch sweep → porta/PITCHENV, noise burst →
PSGNOISE+envelope, zap arp → REGDELTA flutter — same plugin registry as S4);
**`SfxHeaderForm`** (priority with the ≥$C0 ducks-music threshold marked,
gain/duck/cap, one-shot vs continuous w/ re-ping countdown preview, instance
cap; `MEV_SPINREV` exposed ONLY here — music-illegal per manifest);
**audition-in-context** (load any song, fire the SFX over it in the S3
engine — steal/duck/restore audible); **table integration** (export
assigns/updates the SFX id against aeon's generated tables via a
`sfx_export` that emits the transcoder-compatible blob + an id-map patch, and
the jingle variant enforces its class rules from manifest `rules`).

Jingle kind: the S1 model already carries the class constraints; S6 adds the
export path (multi-channel SFX blob, FM4/FM5+PSG windows, no loop) and a
jingle template.

---

### Task 1: SFX project kind + editor shell
Files: create `src/components/SfxEditor.tsx` (+css); `src-tauri/src/compiler/sfx.rs` (SFX-blob emitter — read `aeon/tools/sfx_transcode.py` FIRST and match its output format byte-for-byte; parity test against it, same skip rule as S1's harness).
- [ ] Kind switcher on new-project; SfxEditor mode: lane cap + bar cap from manifest; space-bar recompile+replay loop (<300 ms target — measure and log).
- [ ] Parity test: fixture SFX through both emitters → identical bytes. Commit.

### Task 2: Header form + continuous class
Files: `src/components/SfxHeaderForm.tsx`; model fields (S1 carries them); compile into blob header.
- [ ] All fields manifest-gated (Stage B/C fields grey until engine package 2 ships → flags flip in a regenerated manifest); continuous-class preview simulates the re-ping countdown (audible stop when pings cease).
- [ ] SpinRev event allowed only when kind==Sfx (compiler rule + UI). Tests. Commit.

### Task 3: Archetypes + generators
Files: `src/plugins/sfxGenerators/*.ts`; bundled templates `src-tauri/assets/templates/sfx/*.json`.
- [ ] Six archetype templates authored by ear against classic references (subjective step — controller/user by-ear pass; FOREGROUND if oracle comparison used); three generator plugins compiling to documented events. Tests: generator param → event snapshot. Commit.

### Task 4: Audition-in-context + jingle export + closeout
- [ ] Audition: song loaded in S3 engine + SFX fired via SFX-tier mailbox (steal/duck/restore real); UI = one button on SfxEditor with song picker.
- [ ] Jingle export path (class rules enforced; blob targets the SFX tier tables); id-map/table integration for both kinds with a dry-run diff shown before writing into the aeon checkout (explicit user-visible file list; never silent writes to aeon).
- [ ] By-ear gate (user) on one SFX + one jingle over a real song. Merge → main; queue S6 → DONE (+log). Commit.
