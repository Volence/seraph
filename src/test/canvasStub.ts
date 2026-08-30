// A stub 2D canvas context for the jsdom test environment (audit item F46).
//
// ---------------------------------------------------------------------------
// WHY THIS EXISTS
// ---------------------------------------------------------------------------
// jsdom does not implement HTMLCanvasElement#getContext. It returns null and
// prints "Not implemented: HTMLCanvasElement's getContext()" through its OWN
// virtual console, which bypasses vitest's reporter entirely -- no reporter or
// `silent` setting touches it. On this suite that was 1237 of 1365 log lines,
// ~91% of every test log, one repeated sentence.
//
// The fix is NOT to suppress that message. The two parcels before this one
// existed to STOP hiding output (F44 turned React's act() warning back on, F45
// un-pinned the reporter that was dropping console output from passing tests);
// filtering jsdom's virtual console would undo that work in the same breath.
//
// jsdom's message is a TRUE statement -- canvas really is not implemented here.
// So make the statement false instead of unsayable: give the environment a
// working-enough 2D context. The message then stops being printed because
// there is nothing left to report, not because anyone muted it.
//
// ---------------------------------------------------------------------------
// WHAT THIS DOES *NOT* ESTABLISH -- READ BEFORE TRUSTING A GREEN TEST
// ---------------------------------------------------------------------------
// This stub RASTERISES NOTHING. There are no pixels behind it. Specifically:
//
//   * It is NOT evidence that anything looks right on screen. It cannot be.
//     Nothing here has a colour, a font, or a pixel. A test asserting on the
//     ops below proves that a draw call was ISSUED with certain arguments --
//     never that the result is legible, correctly positioned on a real
//     display, or visually correct in any sense.
//   * measureText() returns a NOMINAL ESTIMATE, not a measurement (see below).
//     Any layout decision a component makes from it is fabricated, and a test
//     that passes because of it has proven nothing about real text layout.
//   * Compositing, clipping, transforms and alpha are RECORDED, NOT APPLIED.
//     ctx.clip() does not clip; a subsequent fillRect is recorded in full even
//     if a real context would have discarded every pixel of it.
//
// Its one honest claim is narrow and worth stating plainly: the drawing code
// in src/components and src/widgets now RUNS under test. Before this, every
// one of those twelve draw functions hit `if (!ctx) return;` on its second
// line and aborted, 1237 times per run -- so a crash in any of that code was
// unreachable by the suite. Now it executes, and a crash in it fails a test.
// That is a real gain. It is not a rendering test.
//
// ---------------------------------------------------------------------------
// SCOPE
// ---------------------------------------------------------------------------
// Only "2d" is stubbed. Any other context type ("webgl", "bitmaprenderer", ...)
// is delegated to jsdom's own implementation, which will report it as not
// implemented exactly as before -- because for those it still is. This stub
// must never grow into a blanket claim that canvas works.
//
// The surface below is an explicit ALLOWLIST, deliberately not a catch-all
// Proxy. An unknown property returns undefined, so `ctx.filRect(...)` throws a
// TypeError here just as it would in a browser. A permissive Proxy would
// swallow that typo and let it reach a real screen.

/** One recorded call or property assignment, in the order it happened. */
export interface CanvasOp {
  /** Method name ("fillRect"), or "set:<prop>" for a property assignment. */
  op: string;
  args: unknown[];
}

const opsByCanvas = new WeakMap<HTMLCanvasElement, CanvasOp[]>();
const ctxByCanvas = new WeakMap<HTMLCanvasElement, CanvasRenderingContext2D>();

/**
 * The drawing calls this canvas has received, oldest first, across every
 * redraw since it was created -- this does not reset per frame or per test.
 * Callers that want one frame should slice from the last "clearRect".
 *
 * Reading this tells you WHAT WAS ASKED FOR, never what was produced. See the
 * header: there are no pixels behind these numbers.
 */
export function canvasOps(canvas: HTMLCanvasElement): CanvasOp[] {
  return opsByCanvas.get(canvas) ?? [];
}

/** Mutable state a real context exposes as readable/writable properties. */
const INITIAL_STATE: Record<string, unknown> = {
  fillStyle: "#000000",
  strokeStyle: "#000000",
  lineWidth: 1,
  lineCap: "butt",
  lineJoin: "miter",
  miterLimit: 10,
  lineDashOffset: 0,
  font: "10px sans-serif",
  textAlign: "start",
  textBaseline: "alphabetic",
  direction: "inherit",
  globalAlpha: 1,
  globalCompositeOperation: "source-over",
  shadowBlur: 0,
  shadowColor: "rgba(0, 0, 0, 0)",
  shadowOffsetX: 0,
  shadowOffsetY: 0,
  filter: "none",
  imageSmoothingEnabled: true,
};

/**
 * Nominal advance width per character. NOT a measurement -- jsdom has no font
 * metrics to measure with. 6px is a plausible figure for the ~10-11px UI fonts
 * this app draws with, chosen so text-fitting code takes a realistic branch
 * rather than the degenerate one that `width: 0` would force (everything
 * always fits). Nothing in src/ consumes measureText today; if something
 * starts to, its behaviour under test is built on this constant, not on type.
 */
const NOMINAL_CHAR_WIDTH = 6;

function makeStubContext(canvas: HTMLCanvasElement): CanvasRenderingContext2D {
  const ops: CanvasOp[] = [];
  opsByCanvas.set(canvas, ops);

  const record = (op: string, ...args: unknown[]): void => {
    ops.push({ op, args });
  };
  /** A method that records its call and returns nothing, like the real one. */
  const noteCall =
    (op: string) =>
    (...args: unknown[]): void => {
      record(op, ...args);
    };

  let lineDash: number[] = [];

  const ctx: Record<string, unknown> = {
    canvas,

    // --- path construction ---
    beginPath: noteCall("beginPath"),
    closePath: noteCall("closePath"),
    moveTo: noteCall("moveTo"),
    lineTo: noteCall("lineTo"),
    arc: noteCall("arc"),
    arcTo: noteCall("arcTo"),
    ellipse: noteCall("ellipse"),
    rect: noteCall("rect"),
    roundRect: noteCall("roundRect"),
    quadraticCurveTo: noteCall("quadraticCurveTo"),
    bezierCurveTo: noteCall("bezierCurveTo"),

    // --- painting ---
    fill: noteCall("fill"),
    stroke: noteCall("stroke"),
    clip: noteCall("clip"),
    fillRect: noteCall("fillRect"),
    strokeRect: noteCall("strokeRect"),
    clearRect: noteCall("clearRect"),
    fillText: noteCall("fillText"),
    strokeText: noteCall("strokeText"),
    drawImage: noteCall("drawImage"),

    // --- transforms and state ---
    save: noteCall("save"),
    restore: noteCall("restore"),
    scale: noteCall("scale"),
    translate: noteCall("translate"),
    rotate: noteCall("rotate"),
    transform: noteCall("transform"),
    setTransform: noteCall("setTransform"),
    resetTransform: noteCall("resetTransform"),

    // --- line dash: getLineDash must return what setLineDash was given ---
    setLineDash(segments: number[]) {
      record("setLineDash", segments);
      lineDash = Array.isArray(segments) ? [...segments] : [];
    },
    getLineDash: () => [...lineDash],

    /**
     * See NOMINAL_CHAR_WIDTH. This is an estimate, not a measurement; the
     * other TextMetrics fields are zero because there is nothing to measure.
     */
    measureText(text: string) {
      record("measureText", text);
      const width = String(text ?? "").length * NOMINAL_CHAR_WIDTH;
      return {
        width,
        actualBoundingBoxLeft: 0,
        actualBoundingBoxRight: width,
        actualBoundingBoxAscent: 0,
        actualBoundingBoxDescent: 0,
        fontBoundingBoxAscent: 0,
        fontBoundingBoxDescent: 0,
      };
    },

    // Gradients record their stops so a test can see which colours were
    // requested; they paint nothing.
    createLinearGradient(...args: unknown[]) {
      record("createLinearGradient", ...args);
      return makeStubGradient(record);
    },
    createRadialGradient(...args: unknown[]) {
      record("createRadialGradient", ...args);
      return makeStubGradient(record);
    },
    createPattern(...args: unknown[]) {
      record("createPattern", ...args);
      return null;
    },
  };

  // State properties are real accessors: assignment is recorded (so a test can
  // see the colour a rect was filled with, in order), and reads return the
  // last assigned value, as a browser does.
  for (const [prop, initial] of Object.entries(INITIAL_STATE)) {
    let value = initial;
    Object.defineProperty(ctx, prop, {
      get: () => value,
      set: (next: unknown) => {
        value = next;
        record(`set:${prop}`, next);
      },
      enumerable: true,
      configurable: true,
    });
  }

  return ctx as unknown as CanvasRenderingContext2D;
}

function makeStubGradient(record: (op: string, ...args: unknown[]) => void): CanvasGradient {
  return {
    addColorStop(offset: number, color: string) {
      record("addColorStop", offset, color);
    },
  } as unknown as CanvasGradient;
}

/**
 * Replace jsdom's unimplemented getContext with the stub above, for "2d" only.
 * Idempotent: a second call is a no-op, so re-importing setup does not stack
 * wrappers. Repeated getContext("2d") on the same canvas returns the SAME
 * context object, matching browser semantics -- components rely on that when
 * they re-read the context on every redraw.
 */
export function installCanvasStub(): void {
  const proto = HTMLCanvasElement.prototype as HTMLCanvasElement & {
    __seraphCanvasStub?: true;
  };
  if (proto.__seraphCanvasStub) return;

  const original = proto.getContext;
  proto.getContext = function (this: HTMLCanvasElement, kind: string, ...rest: unknown[]) {
    if (kind !== "2d") {
      // Honestly still unimplemented. Let jsdom say so.
      return (original as unknown as (...a: unknown[]) => unknown).call(this, kind, ...rest);
    }
    let ctx = ctxByCanvas.get(this);
    if (!ctx) {
      ctx = makeStubContext(this);
      ctxByCanvas.set(this, ctx);
    }
    return ctx;
  } as unknown as typeof proto.getContext;

  proto.__seraphCanvasStub = true;
}
