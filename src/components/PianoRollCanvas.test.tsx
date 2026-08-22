import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRollCanvas } from "./PianoRollCanvas";
import type { Note } from "../types/model";
import { PITCH_RANGES } from "../utils/pianoRollEdit";
import { FOLLOW_REPOSITION_FRACTION } from "../utils/followPlayhead";

// jsdom canvases have no 2d context and getBoundingClientRect() is all
// zeros, so drawing is a no-op and event clientX/clientY map 1:1 onto
// canvas coordinates — which makes the hit-test/gesture math testable.
// What CANNOT be asserted here (TAGGED for the visual pass): the marquee
// rectangle actually rendering, cursor changes, selection highlight color.

function note(tick: number, pitch: number, durationTicks = 200): Note {
  return { tick, pitch, velocity: 100, durationTicks };
}

const [FM_MIN, FM_MAX] = PITCH_RANGES.fm;

const TPP = 10; // ticksPerPixel
const ROW_H = 10;
const GRID = 100; // gridSnapTicks

// note 0: ticks 100..300 (px 10..30), pitch 100 → rows from top = FM_MAX-100
// note 1: ticks 500..600 (px 50..60), pitch 90
// note 2: ticks 1000..1100 (px 100..110), pitch 100
const NOTES = [note(100, 100, 200), note(500, 90, 100), note(1000, 100, 100)];

// Identity of the edited region. Gestures are tagged with it at mousedown
// and refuse to commit once it changes under them (region switches do not
// remount this component); these tests never switch, so one value serves.
const REGION_ID = "region-1";

const rowY = (pitch: number) => (FM_MAX - pitch) * ROW_H + ROW_H / 2;

const handlers = {
  onNoteClick: vi.fn(),
  onMarqueeSelect: vi.fn(),
  onClearSelection: vi.fn(),
  onNoteAdd: vi.fn(),
  onNotesPaint: vi.fn(),
  onNoteErase: vi.fn(),
  onAudition: vi.fn(),
  onNoteResize: vi.fn(),
  onNotesMove: vi.fn(),
  onScrollTopChange: vi.fn(),
  onScrollLeftChange: vi.fn(),
  onZoom: vi.fn(),
};

function renderCanvas(selected: Set<number> = new Set(), drawMode = false, notes: Note[] = NOTES) {
  const { container } = render(
    <PianoRollCanvas
      regionId={REGION_ID}
      drawMode={drawMode}
      notes={notes}
      minPitch={FM_MIN}
      maxPitch={FM_MAX}
      durationTicks={4000}
      ticksPerPixel={TPP}
      scrollLeft={0}
      rowHeight={ROW_H}
      gridSnapTicks={GRID}
      channelColor="#4a9eff"
      selectedNotes={selected}
      playheadTick={-1}
      playing={false}
      {...handlers}
    />,
  );
  const canvas = container.querySelector("canvas");
  expect(canvas).not.toBeNull();
  return canvas as HTMLCanvasElement;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("PianoRollCanvas marquee selection", () => {
  it("drag on empty grid selects the notes intersecting the rectangle", () => {
    const canvas = renderCanvas();
    // start at top-left empty corner, drag to (60, below pitch 90's row)
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: 5, button: 0 });
    fireEvent.mouseMove(window, { clientX: 60, clientY: rowY(90) + ROW_H });
    fireEvent.mouseUp(window, { clientX: 60, clientY: rowY(90) + ROW_H });
    // rect: ticks 50..600, pitch band down to 90-ish → notes 0 and 1, not 2
    expect(handlers.onMarqueeSelect).toHaveBeenCalledWith([0, 1], false);
    expect(handlers.onClearSelection).not.toHaveBeenCalled();
    expect(handlers.onNotesMove).not.toHaveBeenCalled();
  });

  it("shift+drag reports an additive marquee", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: 5, button: 0, shiftKey: true });
    fireEvent.mouseMove(window, { clientX: 40, clientY: rowY(100) + ROW_H });
    fireEvent.mouseUp(window, { clientX: 40, clientY: rowY(100) + ROW_H });
    expect(handlers.onMarqueeSelect).toHaveBeenCalledWith([0], true);
  });

  it("plain click on empty grid (no drag) clears the selection", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: 5, button: 0 });
    fireEvent.mouseUp(window, { clientX: 6, clientY: 6 });
    expect(handlers.onClearSelection).toHaveBeenCalled();
    expect(handlers.onMarqueeSelect).not.toHaveBeenCalled();
  });

  it("alt+drag on empty grid pans instead of marqueeing", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: 5, button: 0, altKey: true });
    fireEvent.mouseMove(window, { clientX: 50, clientY: 5, altKey: true });
    fireEvent.mouseUp(window, { clientX: 50, clientY: 5 });
    expect(handlers.onScrollLeftChange).toHaveBeenCalled();
    expect(handlers.onMarqueeSelect).not.toHaveBeenCalled();
    expect(handlers.onClearSelection).not.toHaveBeenCalled();
  });
});

describe("PianoRollCanvas note clicks and drags", () => {
  it("plain click on an unselected note selects it", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 0 });
    fireEvent.mouseUp(window, { clientX: 15, clientY: rowY(100) });
    expect(handlers.onNoteClick).toHaveBeenCalledWith(0, false);
    // no accidental instant drag
    expect(handlers.onNotesMove).not.toHaveBeenCalled();
  });

  it("dragging an unselected note selects it and moves only it", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 0 });
    fireEvent.mouseMove(window, { clientX: 35, clientY: rowY(100) });
    expect(handlers.onNoteClick).toHaveBeenCalledWith(0, false);
    // +20px * 10 tpp = +200 ticks, snapped to GRID
    expect(handlers.onNotesMove).toHaveBeenLastCalledWith([{ index: 0, tick: 300, pitch: 100 }]);
    fireEvent.mouseUp(window);
  });

  it("dragging an already-selected note keeps the selection and moves the whole group", () => {
    const canvas = renderCanvas(new Set([0, 1]));
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 0 });
    expect(handlers.onNoteClick).not.toHaveBeenCalled();
    fireEvent.mouseMove(window, { clientX: 35, clientY: rowY(100) });
    expect(handlers.onNotesMove).toHaveBeenLastCalledWith([
      { index: 0, tick: 300, pitch: 100 },
      { index: 1, tick: 700, pitch: 90 },
    ]);
    fireEvent.mouseUp(window);
  });

  it("group drag clamps pitch so no member leaves the visible range", () => {
    const canvas = renderCanvas(new Set([0, 1]));
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 0 });
    // drag up far past the top: pitch delta must clamp to FM_MAX - 100
    fireEvent.mouseMove(window, { clientX: 15, clientY: rowY(100) - 100 * ROW_H });
    const maxDelta = FM_MAX - 100; // note 0 is the highest member
    expect(handlers.onNotesMove).toHaveBeenLastCalledWith([
      { index: 0, tick: 100, pitch: 100 + maxDelta },
      { index: 1, tick: 500, pitch: 90 + maxDelta },
    ]);
    fireEvent.mouseUp(window);
  });

  it("double-click on empty grid still draws a new note", () => {
    const canvas = renderCanvas();
    fireEvent.doubleClick(canvas, { clientX: 5, clientY: rowY(95) });
    fireEvent.mouseUp(window, { clientX: 5, clientY: rowY(95) });
    // tick 50 snaps down to 0; one grid unit long
    expect(handlers.onNoteAdd).toHaveBeenCalledWith(0, 95, GRID);
  });
});

describe("PianoRollCanvas Draw Mode paint (F6)", () => {
  // Grid cells are GRID ticks wide (= GRID/TPP = 10px here) and one row
  // tall. Pitch 95 is empty in NOTES, so a run there is unobstructed.
  const EMPTY_PITCH = 95;
  const cell = (tick: number, pitch: number) => ({ tick, pitch, durationTicks: GRID });

  it("a plain click paints exactly one grid-length note", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseUp(window, { clientX: 5, clientY: rowY(EMPTY_PITCH) });
    // x=5 → tick 50 → floors to cell 0, one grid unit long.
    expect(handlers.onNotesPaint).toHaveBeenCalledTimes(1);
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([cell(0, EMPTY_PITCH)]);
    // Painting must not also marquee-select or clear the selection.
    expect(handlers.onMarqueeSelect).not.toHaveBeenCalled();
    expect(handlers.onClearSelection).not.toHaveBeenCalled();
  });

  it("a drag paints a run of grid-snapped notes in ONE commit", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseMove(window, { clientX: 15, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseMove(window, { clientX: 25, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseUp(window, { clientX: 25, clientY: rowY(EMPTY_PITCH) });
    // One call, not three: the parent wraps it in ONE undo group.
    expect(handlers.onNotesPaint).toHaveBeenCalledTimes(1);
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([
      cell(0, EMPTY_PITCH), cell(GRID, EMPTY_PITCH), cell(GRID * 2, EMPTY_PITCH),
    ]);
  });

  it("a drag inside one cell paints one note, not one per mousemove", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseMove(window, { clientX: 6, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseMove(window, { clientX: 9, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseUp(window, { clientX: 9, clientY: rowY(EMPTY_PITCH) });
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([cell(0, EMPTY_PITCH)]);
  });

  it("follows the pointer's row, so a diagonal drag paints a melodic run", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseMove(window, { clientX: 15, clientY: rowY(EMPTY_PITCH + 1) });
    fireEvent.mouseUp(window, { clientX: 15, clientY: rowY(EMPTY_PITCH + 1) });
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([
      cell(0, EMPTY_PITCH), cell(GRID, EMPTY_PITCH + 1),
    ]);
  });

  it("Shift locks the run to the row it started on (Ableton Pitch Lock)", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseMove(window, { clientX: 15, clientY: rowY(EMPTY_PITCH + 3), shiftKey: true });
    fireEvent.mouseUp(window, { clientX: 15, clientY: rowY(EMPTY_PITCH + 3) });
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([
      cell(0, EMPTY_PITCH), cell(GRID, EMPTY_PITCH),
    ]);
  });

  it("skips cells an existing note already occupies", () => {
    // Row of NOTES[0] (ticks 100..300). Cells 100 and 200 are taken; cell 0
    // ends exactly where it starts and cell 300 starts exactly where it
    // ends, so both are free (touching is not overlapping).
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(100), button: 0 });
    for (const x of [15, 25, 35]) {
      fireEvent.mouseMove(window, { clientX: x, clientY: rowY(100) });
    }
    fireEvent.mouseUp(window, { clientX: 35, clientY: rowY(100) });
    expect(handlers.onNotesPaint).toHaveBeenCalledWith([cell(0, 100), cell(300, 100)]);
  });

  it("paints nothing past the region end", () => {
    // durationTicks 4000 → x=450 is tick 4500, outside the document.
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 450, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseUp(window, { clientX: 450, clientY: rowY(EMPTY_PITCH) });
    expect(handlers.onNotesPaint).not.toHaveBeenCalled();
  });

  it("a second click in the same cell adds nothing before the re-fetch lands", () => {
    // `notes` is refreshed asynchronously, so a double-click in Draw Mode is
    // two gestures inside one IPC round trip. A stacked duplicate would be
    // inaudible under last-note-priority — silent corruption.
    const canvas = renderCanvas(new Set(), true);
    for (let i = 0; i < 2; i++) {
      fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
      fireEvent.mouseUp(window, { clientX: 5, clientY: rowY(EMPTY_PITCH) });
    }
    expect(handlers.onNotesPaint).toHaveBeenCalledTimes(1);
  });

  it("auditions once per pitch, not once per painted cell", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0 });
    fireEvent.mouseMove(window, { clientX: 15, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseMove(window, { clientX: 25, clientY: rowY(EMPTY_PITCH + 1) });
    fireEvent.mouseUp(window, { clientX: 25, clientY: rowY(EMPTY_PITCH + 1) });
    expect(handlers.onAudition.mock.calls).toEqual([[EMPTY_PITCH], [EMPTY_PITCH + 1]]);
  });

  it("still moves and resizes notes in Draw Mode (erase is right-click, not left)", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 0 });
    fireEvent.mouseMove(window, { clientX: 35, clientY: rowY(100) });
    expect(handlers.onNotesMove).toHaveBeenLastCalledWith([{ index: 0, tick: 300, pitch: 100 }]);
    expect(handlers.onNotesPaint).not.toHaveBeenCalled();
    fireEvent.mouseUp(window);
  });

  it("Alt+drag still pans in Draw Mode", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH), button: 0, altKey: true });
    fireEvent.mouseMove(window, { clientX: 50, clientY: rowY(EMPTY_PITCH), altKey: true });
    fireEvent.mouseUp(window, { clientX: 50, clientY: rowY(EMPTY_PITCH) });
    expect(handlers.onScrollLeftChange).toHaveBeenCalled();
    expect(handlers.onNotesPaint).not.toHaveBeenCalled();
  });

  it("suppresses the no-mode double-click draw while Draw Mode is on", () => {
    // The two clicks already painted their cell; the draw path on top would
    // add another note there.
    const canvas = renderCanvas(new Set(), true);
    fireEvent.doubleClick(canvas, { clientX: 5, clientY: rowY(EMPTY_PITCH) });
    fireEvent.mouseUp(window, { clientX: 5, clientY: rowY(EMPTY_PITCH) });
    expect(handlers.onNoteAdd).not.toHaveBeenCalled();
  });

  it("does not paint with Draw Mode off — empty-space drag still marquees", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 5, clientY: 5, button: 0 });
    fireEvent.mouseMove(window, { clientX: 60, clientY: rowY(90) + ROW_H });
    fireEvent.mouseUp(window, { clientX: 60, clientY: rowY(90) + ROW_H });
    expect(handlers.onNotesPaint).not.toHaveBeenCalled();
    expect(handlers.onMarqueeSelect).toHaveBeenCalledWith([0, 1], false);
  });
});

describe("PianoRollCanvas right-click erase (F6)", () => {
  it("erases the note under the cursor with Draw Mode off", () => {
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 15, clientY: rowY(100), button: 2 });
    fireEvent.mouseUp(window, { clientX: 15, clientY: rowY(100) });
    expect(handlers.onNoteErase).toHaveBeenCalledWith(0);
    // A right-press is not a select and not a drag.
    expect(handlers.onNoteClick).not.toHaveBeenCalled();
    expect(handlers.onNotesMove).not.toHaveBeenCalled();
  });

  it("erases in Draw Mode too (no tool switch for the commonest correction)", () => {
    const canvas = renderCanvas(new Set(), true);
    fireEvent.mouseDown(canvas, { clientX: 55, clientY: rowY(90), button: 2 });
    fireEvent.mouseUp(window, { clientX: 55, clientY: rowY(90) });
    expect(handlers.onNoteErase).toHaveBeenCalledWith(1);
    expect(handlers.onNotesPaint).not.toHaveBeenCalled();
  });

  it("erases the note near its right edge too (no resize-zone dead spot)", () => {
    // x=29 is inside NOTES[0] (px 10..30) and within EDGE_THRESHOLD of the
    // edge — the zone a left-press would resize in.
    const canvas = renderCanvas();
    fireEvent.mouseDown(canvas, { clientX: 29, clientY: rowY(100), button: 2 });
    fireEvent.mouseUp(window, { clientX: 29, clientY: rowY(100) });
    expect(handlers.onNoteErase).toHaveBeenCalledWith(0);
    expect(handlers.onNoteResize).not.toHaveBeenCalled();
  });

  it("does nothing on empty space, in either mode", () => {
    for (const mode of [false, true]) {
      vi.clearAllMocks();
      const canvas = renderCanvas(new Set(), mode);
      fireEvent.mouseDown(canvas, { clientX: 200, clientY: rowY(95), button: 2 });
      fireEvent.mouseUp(window, { clientX: 200, clientY: rowY(95) });
      expect(handlers.onNoteErase).not.toHaveBeenCalled();
      expect(handlers.onNotesPaint).not.toHaveBeenCalled();
      expect(handlers.onClearSelection).not.toHaveBeenCalled();
      expect(handlers.onMarqueeSelect).not.toHaveBeenCalled();
    }
  });
});

describe("PianoRollCanvas follow-playhead suppression", () => {
  // jsdom rects are all zeros, which disables follow (viewWidth 0), so give
  // every element a real width for this describe only.
  const VIEW_W = 1000;
  let rectSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    rectSpy = vi.spyOn(Element.prototype, "getBoundingClientRect").mockReturnValue({
      x: 0, y: 0, top: 0, left: 0, bottom: 0, right: VIEW_W,
      width: VIEW_W, height: 400,
      toJSON: () => ({}),
    } as DOMRect);
  });

  afterEach(() => {
    rectSpy.mockRestore();
  });

  function renderPlaying(playheadTick: number, suppressFollow: boolean) {
    render(
      <PianoRollCanvas
        regionId={REGION_ID}
        drawMode={false}
        notes={NOTES}
        minPitch={FM_MIN}
        maxPitch={FM_MAX}
        durationTicks={40000}
        ticksPerPixel={TPP}
        scrollLeft={0}
        rowHeight={ROW_H}
        gridSnapTicks={GRID}
        channelColor="#4a9eff"
        selectedNotes={new Set()}
        playheadTick={playheadTick}
        playing={true}
        suppressFollow={suppressFollow}
        {...handlers}
      />,
    );
  }

  // Playhead px = tick / TPP; past the follow edge of VIEW_W so follow fires.
  const PAST_EDGE_TICK = VIEW_W * TPP * 0.9;
  const EXPECTED_TARGET = PAST_EDGE_TICK / TPP - VIEW_W * FOLLOW_REPOSITION_FRACTION;

  it("pages forward while playing when follow is not suppressed (harness sanity)", () => {
    renderPlaying(PAST_EDGE_TICK, false);
    expect(handlers.onScrollLeftChange).toHaveBeenCalledWith(EXPECTED_TARGET);
  });

  it("never scrolls while suppressFollow is set (preview loop active)", () => {
    // Owner ruling: while looping, the view moves only by user input.
    renderPlaying(PAST_EDGE_TICK, true);
    expect(handlers.onScrollLeftChange).not.toHaveBeenCalled();
  });
});
