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

const rowY = (pitch: number) => (FM_MAX - pitch) * ROW_H + ROW_H / 2;

const handlers = {
  onNoteClick: vi.fn(),
  onMarqueeSelect: vi.fn(),
  onClearSelection: vi.fn(),
  onNoteAdd: vi.fn(),
  onAudition: vi.fn(),
  onNoteResize: vi.fn(),
  onNotesMove: vi.fn(),
  onScrollTopChange: vi.fn(),
  onScrollLeftChange: vi.fn(),
  onZoom: vi.fn(),
};

function renderCanvas(selected: Set<number> = new Set()) {
  const { container } = render(
    <PianoRollCanvas
      notes={NOTES}
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
