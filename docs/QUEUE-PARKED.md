# Parked queue rows (off the board, not off the queue)

These rows were removed from `docs/lane-status.json` on 2026-09-06 on the owner's ask,
relayed by the hub: *"can we get a cleanup of the board and then a cleanup of the
worktrees or whatever all this is"*. The board now carries only what waits on him or
would start the day the hold lifts, per `contract/LANE_STATUS.md` rule 6 (the file is a
pointer to where the lane is, not a record of what it decided) and rule 7 (bounds).

**Nothing here was cancelled, deprioritized, or judged.** Each row is reproduced
verbatim as it stood on the board, so putting one back is a copy rather than a rewrite.
The condition that would put it back is stated under each. When one moves, delete it
from this file and add it to the board in the same edit, so the two never both claim it.

The full history of every row lives where it always did, in the Log of
`docs/superpowers/2026-07-03-seraph-banking-queue.md`, found by grepping the id.

## F27

```json
{
  "id": "F27",
  "title": "Make playback follow the driver instead of promising both at once",
  "state": "open",
  "size": "M",
  "blockedBy": "the Memra-only channel 6 mode setting, which d-7 settled the rest of and deliberately left for later"
}
```

Back on the board when: the channel 6 mode question is put to the owner and answered, or
a session decides the fix can be built without it. d-7 settled the rest of that area and
left this piece open on purpose, so this is a deliberate remainder rather than a stall.

## F36

```json
{
  "id": "F36",
  "title": "Instrument channels are written in the order your tracks happen to be arranged, but the driver assigns them by position, so a label can name a different channel than the one that plays",
  "state": "open",
  "size": "M",
  "blockedBy": "mechanism verified, consequence not yet; not put to anyone until it is"
}
```

Back on the board when: somebody measures what an author actually hears or sees when the
mismatch fires. The mechanism is verified; what it costs a user is not, and this lane's
standing rule is that a finding is not put to anyone until the consequence is measured.

## PITCH-FORMAT

```json
{
  "id": "PITCH-FORMAT",
  "title": "My half of the joint plan with the engine lane is written and handed over; the engine lane's measured reply on cycle and ROM cost is what moves it",
  "state": "open",
  "size": "L",
  "blockedBy": "aeon's measured reply on cycle and ROM cost",
  "project": "SOUND-TRUTH"
}
```

Back on the board when: aeon replies with measured cycle and ROM cost. Note one visible
consequence of parking it: this was one of two `SOUND-TRUTH` rows on the Board, so the
hub's project grouping now sees seraph's half of that project through S0 alone. S0 is
still on the board, so the project does not vanish from his screen.

## README-8

```json
{
  "id": "README-8",
  "title": "Two leftovers from the README pass, neither a bug: the app cannot author FM3 special mode, and channel overlap warnings are computed but never shown anywhere",
  "state": "open",
  "size": "M",
  "blockedBy": "not a fix: FM3 special mode is a feature, and the overlap warnings have no place on screen yet"
}
```

Back on the board when: FM3 special mode becomes a feature somebody wants authored, or a
screen exists that could show a channel overlap warning. Neither is a defect, and neither
should be presented to the owner as one.
