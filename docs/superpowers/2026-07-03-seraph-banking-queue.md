# Seraph Banking Queue — 2026-07-03

**What this is:** the canonical status record for the Seraph Aeon-profile
banking effort (same pattern as `aeon/docs/superpowers/2026-07-03-sound-banking-queue.md`).
Master design: `specs/2026-07-03-aeon-profile-banking-design.md` (APPROVED,
user, 2026-07-03). Grounding inventory:
`2026-07-03-memra-author-surface-inventory.md`. Each package gets a research
pass, a spec (where marked), a user review gate, and a cold-executable plan —
so any future session (any model) can execute without re-deriving.

Standing decisions: driver named **Memra** (docs/UI only); driver-in-the-loop
guarantee; Aeon-native model / wide import / narrow export; manifest lives in
**empyrean**; correct-by-construction UI over export-time validation.

## Package queue (execution order)

| # | Package | Deliverables | Status |
|---|---------|-------------|--------|
| S0 | **Memra contract** — capability manifest in empyrean, generated-from/validated-against aeon source; Memra naming pass; budget-gate feature flags | spec + plan | **IN PROGRESS** (spec session started 2026-07-03) |
| S1 | Aeon-native project model + compiler (project → packer input → MEV) | spec + plan | queued |
| S2 | Verification gate (export-time oracle/VGM A/B; permanent CI-style) | plan (+ spec §) | queued |
| S3 | Driver-in-the-loop preview (embedded Z80 + real Memra blob) | spec + plan | queued |
| S4 | Aeon-profile authoring UX (4 strata + plugin rack; approved in master design) | spec + plan | queued |
| S5 | Import funnel (SMPS/VGM/Zyrinx retarget + GEMS; SMPS export = marked lossy) | spec + plan | queued |
| S6 | SFX Workshop (Song/Jingle/SFX project types; approved in master design) | spec + plan | queued |

**Engine dependencies (aeon queue, banked 2026-07-03, not yet executed):**
S6 ⇐ engine package 2 (Stage B/C header fields); S4 ghost/echo + ExtCh3 UX ⇐
engine package 5 Tier 2 budget gates; COMM markers ⇐ engine package 1. Seraph
designs target the banked specs (normative); manifest flags carry the gates.

## Log

- 2026-07-03: Master design + inventory committed (fd14c4b). Queue doc created.
  S0 spec session opened.
