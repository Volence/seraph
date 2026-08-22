import { useState, useEffect, useCallback, useRef } from "react";
import type { Note, SelectedRegion, SongMetadata } from "../types/model";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import { PianoRollKeys, MELODIC_KEYS_WIDTH, DAC_KEYS_WIDTH } from "./PianoRollKeys";
import { PianoRollCanvas } from "./PianoRollCanvas";
import { VelocityLane } from "./VelocityLane";
import * as ipc from "../api/ipc";
import * as grid from "../utils/grid";
import { SONG_REVERTED_EVENT } from "../utils/keyboard";
import { OCTAVE_SEMITONES, PITCH_RANGES, DEFAULT_PITCH_RANGE, transposeNotes, nudgeNotes } from "../utils/pianoRollEdit";
import { copyNotes, getNoteClipboard, lastCopiedKind, planNotePaste } from "../utils/clipboard";
import { setPianoRollNoteSelectionActive } from "../utils/noteSelection";
import { isEditableTarget } from "../utils/keyboard";
import styles from "./PianoRoll.module.css";

interface PianoRollProps {
  region: SelectedRegion;
  onClose: () => void;
  playing: boolean;
  projectMeta: SongMetadata;
  /** The white seek cursor (absolute song ticks); anchors Ctrl+V paste. */
  seekTick: number;
}

const GRID_OPTIONS: { label: string; divisor: number }[] = [
  { label: "1/1", divisor: 1 },
  { label: "1/2", divisor: 2 },
  { label: "1/4", divisor: 4 },
  { label: "1/8", divisor: 8 },
  { label: "1/16", divisor: 16 },
  { label: "1/32", divisor: 32 },
  { label: "1/4T", divisor: 6 },
  { label: "1/8T", divisor: 12 },
];

const CHANNEL_COLORS: Record<string, string> = {
  fm: "#4a9eff",
  psg: "#44cc66",
  dac: "#ff8844",
};

const DAC_SAMPLE_NAMES: Record<number, string> = {
  36: "Snare S3", 37: "High Tom", 38: "Mid Tom S3", 39: "Low Tom S3",
  40: "Floor Tom S3", 41: "Kick S3", 42: "Muffled Snare", 43: "Crash Cymbal",
  44: "Ride Cymbal", 45: "Low Metal Hit", 46: "Metal Hit", 47: "Hi Metal Hit",
  48: "Higher Metal", 49: "Mid Metal Hit", 50: "Clap S3", 51: "Elec Hi Tom",
  52: "Elec Mid Tom", 53: "Elec Lo Tom", 54: "Elec Floor Tom", 55: "Tight Snare",
  56: "Midpitch Snare", 57: "Loose Snare", 58: "Looser Snare", 59: "Hi Timpani",
  60: "Lo Timpani", 61: "Mid Timpani", 62: "Quick Loose Snr", 63: "Click",
  64: "Power Kick",
};

export function PianoRoll({ region, onClose, playing, projectMeta, seekTick }: PianoRollProps) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [selectedNotes, setSelectedNotes] = useState<Set<number>>(new Set());
  const [gridIdx, setGridIdx] = useState(4);
  const [scrollTop, setScrollTop] = useState(0);
  const ticksPerBeat = projectMeta.ticksPerBeat;
  const ticksPerBar = grid.ticksPerBar(projectMeta);
  const gridSnapTicks = Math.round(ticksPerBar / GRID_OPTIONS[gridIdx].divisor);

  const isDac = region.channelType === "dac";
  const computedRange = (() => {
    const [defaultMin, defaultMax] = PITCH_RANGES[region.channelType] || DEFAULT_PITCH_RANGE;
    if (notes.length === 0) return [defaultMin, defaultMax] as [number, number];
    const pitches = notes.map(n => n.pitch);
    const lo = Math.min(...pitches);
    const hi = Math.max(...pitches);
    if (isDac) {
      return [Math.max(0, lo - 1), hi + 1] as [number, number];
    }
    return [Math.min(defaultMin, lo), Math.max(defaultMax, hi)] as [number, number];
  })();
  const [minPitch, maxPitch] = computedRange;
  const drumLabels: Record<number, string> | undefined = isDac ? DAC_SAMPLE_NAMES : undefined;
  const rowHeight = isDac ? 22 : 14;
  const defaultTpp = Math.min(region.durationTicks / 800, ticksPerBar * 8 / 800);
  const [ticksPerPixel, setTicksPerPixel] = useState(defaultTpp);
  // Key-column width is owned here so the velocity lane can offset itself by
  // the same amount (G5). Reset when the channel kind flips (DAC keys are
  // wider and user-resizable).
  const [keysWidth, setKeysWidth] = useState(isDac ? DAC_KEYS_WIDTH : MELODIC_KEYS_WIDTH);
  useEffect(() => {
    setKeysWidth(isDac ? DAC_KEYS_WIDTH : MELODIC_KEYS_WIDTH);
  }, [isDac]);
  const [pianoScrollLeft, setPianoScrollLeft] = useState(0);
  const channelColor = CHANNEL_COLORS[region.channelType] || "#888";

  function handleZoom(deltaY: number, centerX: number) {
    const zoomFactor = deltaY > 0 ? 1.15 : 0.87;
    setTicksPerPixel((prev) => {
      const next = prev * zoomFactor;
      const minTpp = ticksPerBeat / 480;
      const clamped = Math.max(minTpp, Math.min(next, ticksPerBar * 2));
      const tickAtCenter = (centerX + pianoScrollLeft) * prev;
      const newScrollLeft = tickAtCenter / clamped - centerX;
      setPianoScrollLeft(Math.max(0, newScrollLeft));
      return clamped;
    });
  }
  const { interpolatedTick } = usePlaybackPosition(playing, projectMeta.tempo, projectMeta.ticksPerBeat);
  const playheadTick = playing ? interpolatedTick - region.startTick : -1;

  // Ref so refresh()'s identity doesn't churn with the parent's inline
  // onClose prop (refresh re-runs on identity change).
  const onCloseRef = useRef(onClose);
  onCloseRef.current = onClose;

  const refresh = useCallback(async () => {
    const tracks = await ipc.listTracks();
    const track = tracks.find((t) => t.id === region.trackId);
    const r = track?.regions.find((r) => r.id === region.regionId);
    if (!r) {
      // The open region no longer exists (e.g. undo of its creation, or a
      // reverted move) — close rather than editing a stale view.
      onCloseRef.current();
      return;
    }
    setNotes(r.notes);
  }, [region.trackId, region.regionId]);

  useEffect(() => { refresh(); }, [refresh]);

  // Undo/redo replaced the tracks server-side — re-fetch immediately.
  useEffect(() => {
    const onReverted = () => { refresh(); };
    window.addEventListener(SONG_REVERTED_EVENT, onReverted);
    return () => window.removeEventListener(SONG_REVERTED_EVENT, onReverted);
  }, [refresh]);

  // Publish whether this piano roll owns a note selection, so the
  // arrangement's window-level Delete handler defers to us (G1). Cleared on
  // unmount so region deletion works again once the roll closes.
  useEffect(() => {
    setPianoRollNoteSelectionActive(selectedNotes.size > 0);
  }, [selectedNotes]);
  useEffect(() => () => setPianoRollNoteSelectionActive(false), []);

  async function handleNoteAdd(tick: number, pitch: number, duration: number) {
    await ipc.addNote(region.trackId, region.regionId, tick, pitch, 100, duration);
    refresh();
  }

  async function handleNoteClick(index: number, additive: boolean) {
    setSelectedNotes((prev) => {
      if (!additive) return new Set([index]);
      const next = new Set(prev);
      next.add(index);
      return next;
    });
  }

  function handleMarqueeSelect(indices: number[], additive: boolean) {
    setSelectedNotes((prev) => {
      if (!additive) return new Set(indices);
      const next = new Set(prev);
      for (const i of indices) next.add(i);
      return next;
    });
  }

  function handleClearSelection() {
    setSelectedNotes(new Set());
  }

  async function handleNoteResize(index: number, newDurationTicks: number) {
    const note = notes[index];
    if (!note) return;
    await ipc.updateNote(region.trackId, region.regionId, index, note.tick, note.pitch, note.velocity, newDurationTicks);
    refresh();
  }

  async function handleNotesMove(moves: { index: number; tick: number; pitch: number }[]) {
    for (const m of moves) {
      const note = notes[m.index];
      if (!note) continue;
      await ipc.updateNote(region.trackId, region.regionId, m.index, m.tick, m.pitch, note.velocity, note.durationTicks);
    }
    refresh();
  }

  // Drag gestures (note move/resize) fire one updateNote per mousemove;
  // bracketing mousedown→mouseup in an undo group coalesces the whole drag
  // into a single undo step.
  const handleGestureStart = useCallback(() => { ipc.beginUndoGroup(); }, []);
  const handleGestureEnd = useCallback(() => { ipc.endUndoGroup(); }, []);

  async function handleVelocityChange(index: number, velocity: number) {
    const note = notes[index];
    if (!note) return;
    await ipc.updateNote(region.trackId, region.regionId, index, note.tick, note.pitch, velocity, note.durationTicks);
    refresh();
  }

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      // Don't hijack keys while the user is in a form control (e.g. the
      // grid-size <select>, where arrows change the value) or any other
      // editable target (G2).
      if (isEditableTarget(e.target)) return;

      if ((e.key === "ArrowUp" || e.key === "ArrowDown") && selectedNotes.size > 0) {
        e.preventDefault();
        const direction = e.key === "ArrowUp" ? 1 : -1;
        const step = e.ctrlKey || e.metaKey ? OCTAVE_SEMITONES : 1;
        // Blocked (null) when any selected note would leave the pitch range:
        // the whole move is refused so intervals stay intact.
        const moves = transposeNotes(notes, selectedNotes, direction * step, minPitch, maxPitch);
        if (moves) {
          (async () => {
            // One transpose = one undo step: group the batch.
            await ipc.beginUndoGroup();
            try {
              for (const m of moves) {
                const n = notes[m.index];
                await ipc.updateNote(region.trackId, region.regionId, m.index, n.tick, m.pitch, n.velocity, n.durationTicks);
              }
            } finally {
              await ipc.endUndoGroup();
            }
            refresh();
          })();
        }
      }
      // Shared by Delete/Backspace and Ctrl+X: grouped multi-delete
      // (one gesture = one undo step), descending so indices stay valid.
      const deleteSelection = async () => {
        const sorted = Array.from(selectedNotes).sort((a, b) => b - a);
        await ipc.beginUndoGroup();
        try {
          for (const idx of sorted) {
            await ipc.deleteNote(region.trackId, region.regionId, idx);
          }
        } finally {
          await ipc.endUndoGroup();
        }
        setSelectedNotes(new Set());
        refresh();
      };

      if ((e.key === "ArrowLeft" || e.key === "ArrowRight") && selectedNotes.size > 0) {
        e.preventDefault();
        const direction = e.key === "ArrowRight" ? 1 : -1;
        // Plain arrow = the grid selector's snap step; Ctrl/Cmd = 1-tick fine nudge.
        const step = e.ctrlKey || e.metaKey ? 1 : gridSnapTicks;
        // Blocked (null) when any selected note would cross tick 0 or the
        // region end: block-whole-move, matching the transpose convention.
        const moves = nudgeNotes(notes, selectedNotes, direction * step, region.durationTicks);
        if (moves) {
          (async () => {
            // One nudge keypress = one undo step: group the batch.
            await ipc.beginUndoGroup();
            try {
              for (const m of moves) {
                const n = notes[m.index];
                await ipc.updateNote(region.trackId, region.regionId, m.index, m.tick, n.pitch, n.velocity, n.durationTicks);
              }
            } finally {
              await ipc.endUndoGroup();
            }
            refresh();
          })();
        }
      }
      if ((e.key === "Delete" || e.key === "Backspace") && selectedNotes.size > 0) {
        deleteSelection();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "c" && selectedNotes.size > 0) {
        e.preventDefault();
        copyNotes(notes, selectedNotes);
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "x" && selectedNotes.size > 0) {
        e.preventDefault();
        // Cut = copy + grouped multi-delete.
        copyNotes(notes, selectedNotes);
        deleteSelection();
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "v" && lastCopiedKind() === "notes") {
        const clip = getNoteClipboard();
        if (clip.length > 0) {
          e.preventDefault();
          // Paste-anchor rule (Ableton-ish default): notes land relative to
          // the EARLIEST copied note, anchored at the seek cursor when it
          // falls inside the open region, else at tick 0 of the region.
          const cursorInRegion = seekTick >= region.startTick
            && seekTick < region.startTick + region.durationTicks;
          const anchor = cursorInRegion ? seekTick - region.startTick : 0;
          const plan = planNotePaste(clip, anchor, region.durationTicks);
          if (plan.skipped > 0) {
            // Loud, not silent: dropped notes would have started past the
            // region end.
            console.warn(`paste: skipped ${plan.skipped} note(s) past the region end`);
          }
          if (plan.placements.length > 0) {
            (async () => {
              const newIndices: number[] = [];
              // One paste = one undo step: group the addNote batch.
              await ipc.beginUndoGroup();
              try {
                for (const p of plan.placements) {
                  const newIdx = await ipc.addNote(region.trackId, region.regionId, p.tick, p.pitch, p.velocity, p.durationTicks);
                  newIndices.push(newIdx);
                }
              } finally {
                await ipc.endUndoGroup();
              }
              await refresh();
              // Pasted notes become the new selection.
              setSelectedNotes(new Set(newIndices));
            })();
          }
        }
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "a") {
        e.preventDefault();
        setSelectedNotes(new Set(notes.map((_, i) => i)));
      }
      if ((e.ctrlKey || e.metaKey) && e.key === "d" && selectedNotes.size > 0) {
        e.preventDefault();
        (async () => {
          const sorted = Array.from(selectedNotes).sort((a, b) => a - b);
          const maxEnd = sorted.reduce((m, idx) => {
            const n = notes[idx];
            return Math.max(m, n.tick + n.durationTicks);
          }, 0);
          const minStart = sorted.reduce((m, idx) => Math.min(m, notes[idx].tick), Infinity);
          const offset = maxEnd - minStart;
          const newIndices: number[] = [];
          // One duplicate = one undo step: group the addNote batch.
          await ipc.beginUndoGroup();
          try {
            for (const idx of sorted) {
              const n = notes[idx];
              const newIdx = await ipc.addNote(region.trackId, region.regionId, n.tick + offset, n.pitch, n.velocity, n.durationTicks);
              newIndices.push(newIdx);
            }
          } finally {
            await ipc.endUndoGroup();
          }
          await refresh();
          setSelectedNotes(new Set(newIndices));
        })();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedNotes, notes, minPitch, maxPitch, region.trackId, region.regionId, region.startTick, region.durationTicks, gridSnapTicks, seekTick, refresh]);

  const fmPreviewTimer = useRef<ReturnType<typeof setTimeout>>(0 as unknown as ReturnType<typeof setTimeout>);

  async function handleAudition(pitch: number) {
    const tracks = await ipc.listTracks();
    const track = tracks.find((t) => t.id === region.trackId);
    if (!track?.instrumentId) return;
    if (region.channelType === "fm") {
      clearTimeout(fmPreviewTimer.current);
      await ipc.previewFmInstrument(track.instrumentId, pitch);
      fmPreviewTimer.current = setTimeout(() => { ipc.stopFmPreview(); }, 500);
    } else if (region.channelType === "psg") {
      await ipc.previewPsgInstrument(track.instrumentId, pitch);
    } else {
      const dacNote = notes.find(n => n.pitch === pitch && n.instrumentId);
      const dacInstId = dacNote?.instrumentId ?? track.instrumentId;
      if (dacInstId) await ipc.previewDac(dacInstId);
    }
  }

  const barStart = Math.floor(region.startTick / ticksPerBar) + 1;
  const barEnd = Math.ceil((region.startTick + region.durationTicks) / ticksPerBar);

  const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
  function noteName(pitch: number): string {
    const octave = Math.floor(pitch / 12) - 1;
    return `${NOTE_NAMES[pitch % 12]}${octave}`;
  }

  const selInfo = selectedNotes.size === 1
    ? (() => {
        const idx = Array.from(selectedNotes)[0];
        const n = notes[idx];
        if (!n) return "";
        const dur = `${(n.durationTicks / ticksPerBeat).toFixed(2)} beats`;
        return `${noteName(n.pitch)} vel:${n.velocity} ${dur}${n.detune ? ` det:${n.detune > 0 ? "+" : ""}${n.detune}` : ""}${n.modulation ? " mod" : ""}`;
      })()
    : selectedNotes.size > 1
      ? `${selectedNotes.size} notes`
      : "";

  return (
    <div className={styles.pianoRoll}>
      <div className={styles.header}>
        <span className={styles.label}>
          {region.trackName} | Bars {barStart}-{barEnd}
        </span>
        {selInfo && <span className={styles.noteInfo}>{selInfo}</span>}
        <select
          className={styles.gridSelect}
          value={gridIdx}
          onChange={(e) => setGridIdx(parseInt(e.target.value))}
        >
          {GRID_OPTIONS.map((opt, i) => (
            <option key={opt.label} value={i}>{opt.label}</option>
          ))}
        </select>
        <button className={styles.closeBtn} onClick={onClose}>x</button>
      </div>
      <div className={styles.body}>
        <PianoRollKeys
          minPitch={minPitch}
          maxPitch={maxPitch}
          rowHeight={rowHeight}
          scrollTop={scrollTop}
          width={keysWidth}
          onWidthChange={setKeysWidth}
          onAudition={handleAudition}
          drumLabels={drumLabels}
        />
        <PianoRollCanvas
          notes={notes}
          minPitch={minPitch}
          maxPitch={maxPitch}
          durationTicks={region.durationTicks}
          ticksPerPixel={ticksPerPixel}
          scrollLeft={pianoScrollLeft}
          rowHeight={rowHeight}
          gridSnapTicks={gridSnapTicks}
          channelColor={channelColor}
          selectedNotes={selectedNotes}
          onNoteClick={handleNoteClick}
          onMarqueeSelect={handleMarqueeSelect}
          onClearSelection={handleClearSelection}
          onNoteAdd={handleNoteAdd}
          onAudition={handleAudition}
          onNoteResize={handleNoteResize}
          onNotesMove={handleNotesMove}
          onGestureStart={handleGestureStart}
          onGestureEnd={handleGestureEnd}
          onScrollTopChange={setScrollTop}
          onScrollLeftChange={setPianoScrollLeft}
          onZoom={handleZoom}
          playheadTick={playheadTick}
          playing={playing}
        />
      </div>
      <VelocityLane
        notes={notes}
        durationTicks={region.durationTicks}
        ticksPerPixel={ticksPerPixel}
        scrollLeft={pianoScrollLeft}
        channelColor={channelColor}
        keysWidth={keysWidth}
        onVelocityChange={handleVelocityChange}
      />
    </div>
  );
}
