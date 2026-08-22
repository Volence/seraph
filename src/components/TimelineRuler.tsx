import { useRef, useEffect, useCallback, useState } from "react";
import type { SnapMode } from "../utils/grid";
import { dragPreviewLoop, type PreviewLoopRange } from "../utils/previewLoop";
import { rulerMarks } from "../utils/rulerMarks";
import { loopHitZone, resizePreviewLoop, movePreviewLoop } from "../utils/loopHandles";
import { resolveDragAxis, dragZoomFactor, type DragAxis } from "../utils/zoomDrag";
import styles from "./TimelineRuler.module.css";

interface TimelineRulerProps {
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  /** Last-set preview loop range (drawn when `loopEnabled`). */
  loop: PreviewLoopRange | null;
  loopEnabled: boolean;
  /** Arrangement snap for the loop drag (shared with region editing). */
  snapMode: SnapMode;
  onSeek: (tick: number) => void;
  onScrollChange: (scrollLeft: number) => void;
  /** Horizontal loop gesture released: commit this preview-loop range. */
  onLoopDrag: (startTick: number, endTick: number) => void;
  /** Vertical drag increment: rescale time around `anchorPx` by `factor`. */
  onZoom: (anchorPx: number, factor: number) => void;
}

// Accent for the loop band; matches the FM region blue used elsewhere.
const LOOP_COLOR = "#4a9eff";

/**
 * What a mousedown grabbed. The drawn loop band (upper half) exposes edge
 * handles and a movable body; the rest of the upper half paints a new
 * loop; the lower half scrubs. Any zone dragged VERTICALLY zooms instead
 * (dominant-axis lock, decided once per gesture).
 */
type RulerZone = "loopStart" | "loopEnd" | "loopBody" | "loopStrip" | "scrub";

interface RulerGesture {
  zone: RulerZone;
  x0: number; // client coords at mousedown
  y0: number;
  anchorPx: number; // ruler-relative x, zoom center
  anchorTick: number;
  lastY: number;
  startScroll: number;
  baseLoop: PreviewLoopRange | null;
  axis: DragAxis;
}

const HOVER_CURSOR: Record<RulerZone, string> = {
  loopStart: "ew-resize",
  loopEnd: "ew-resize",
  loopBody: "move",
  loopStrip: "col-resize",
  scrub: "grab",
};

const DRAG_CURSOR: Record<RulerZone, string> = {
  loopStart: "ew-resize",
  loopEnd: "ew-resize",
  loopBody: "grabbing",
  loopStrip: "col-resize",
  scrub: "grabbing",
};

export function TimelineRuler({
  ticksPerPixel,
  scrollLeft,
  ticksPerBeat,
  beatsPerBar,
  loop,
  loopEnabled,
  snapMode,
  onSeek,
  onScrollChange,
  onLoopDrag,
  onZoom,
}: TimelineRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const gestureRef = useRef<RulerGesture | null>(null);
  const [dragging, setDragging] = useState(false);
  const [dragAxis, setDragAxis] = useState<DragAxis>("none");
  const [loopDragRange, setLoopDragRange] = useState<PreviewLoopRange | null>(null);
  const [hoverZone, setHoverZone] = useState<RulerZone | null>(null);

  // The denominator never enters bar math; placeholder 4 satisfies GridMeta.
  const gridMeta = { ticksPerBeat, timeSignature: [beatsPerBar, 4] as [number, number] };

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;
    ctx.clearRect(0, 0, w, h);

    const startTick = scrollLeft * ticksPerPixel;
    const endTick = startTick + w * ticksPerPixel;

    ctx.fillStyle = "#888888";
    ctx.font = "10px sans-serif";

    for (const mark of rulerMarks(startTick, endTick, gridMeta, ticksPerPixel)) {
      const x = (mark.tick - startTick) / ticksPerPixel;
      if (mark.kind === "bar") {
        ctx.strokeStyle = "#555555";
        ctx.beginPath();
        ctx.moveTo(x, h - 8);
        ctx.lineTo(x, h);
        ctx.stroke();
        if (mark.labeled) ctx.fillText(`${mark.bar}`, x + 3, 12);
      } else {
        ctx.strokeStyle = "#3a3a3a";
        ctx.beginPath();
        ctx.moveTo(x, h - 4);
        ctx.lineTo(x, h);
        ctx.stroke();
      }
    }

    // Preview-loop bracket: the live drag range while a loop gesture is in
    // flight, otherwise the committed range whenever the loop is enabled.
    const band = loopDragRange ?? (loopEnabled && loop ? loop : null);
    if (band) {
      const x1 = (band.start - startTick) / ticksPerPixel;
      const x2 = (band.end - startTick) / ticksPerPixel;
      if (x2 >= 0 && x1 <= w) {
        const bandH = Math.max(4, Math.floor(h / 5));
        ctx.fillStyle = LOOP_COLOR + (loopDragRange ? "55" : "44");
        ctx.fillRect(x1, 0, x2 - x1, bandH);
        ctx.strokeStyle = LOOP_COLOR;
        ctx.lineWidth = 2;
        // Top rail + down-hooks at both ends (classic loop bracket). The
        // hooks double as the visible edge handles.
        ctx.beginPath();
        ctx.moveTo(x1 + 1, bandH + 4);
        ctx.lineTo(x1 + 1, 1);
        ctx.lineTo(x2 - 1, 1);
        ctx.lineTo(x2 - 1, bandH + 4);
        ctx.stroke();
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar, loop, loopEnabled, loopDragRange]);

  useEffect(() => {
    draw();
    const obs = new ResizeObserver(draw);
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [draw]);

  function tickAt(clientX: number): number {
    const container = containerRef.current;
    if (!container) return 0;
    const rect = container.getBoundingClientRect();
    const x = clientX - rect.left;
    return Math.max(0, (x + scrollLeft) * ticksPerPixel);
  }

  /** Which zone a pointer position grabs (see RulerZone). */
  function zoneAt(clientX: number, clientY: number): RulerZone {
    const container = containerRef.current;
    if (!container) return "scrub";
    const rect = container.getBoundingClientRect();
    const y = clientY - rect.top;
    if (y >= rect.height / 2) return "scrub";
    if (loopEnabled && loop) {
      const x = clientX - rect.left;
      const startPx = loop.start / ticksPerPixel - scrollLeft;
      const endPx = loop.end / ticksPerPixel - scrollLeft;
      const hit = loopHitZone(x, startPx, endPx);
      if (hit === "start") return "loopStart";
      if (hit === "end") return "loopEnd";
      if (hit === "body") return "loopBody";
    }
    return "loopStrip";
  }

  function handleMouseDown(e: React.MouseEvent) {
    e.preventDefault();
    const container = containerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    gestureRef.current = {
      zone: zoneAt(e.clientX, e.clientY),
      x0: e.clientX,
      y0: e.clientY,
      anchorPx: e.clientX - rect.left,
      anchorTick: tickAt(e.clientX),
      lastY: e.clientY,
      startScroll: scrollLeft,
      baseLoop: loopEnabled && loop ? loop : null,
      axis: "none",
    };
    setDragAxis("none");
    setDragging(true);
  }

  useEffect(() => {
    if (!dragging) return;

    function loopRangeFor(g: RulerGesture, clientX: number): PreviewLoopRange | null {
      switch (g.zone) {
        case "loopStrip":
          return dragPreviewLoop(g.anchorTick, tickAt(clientX), gridMeta, snapMode);
        case "loopStart":
        case "loopEnd":
          if (!g.baseLoop) return null;
          return resizePreviewLoop(
            g.baseLoop,
            g.zone === "loopStart" ? "start" : "end",
            tickAt(clientX),
            gridMeta,
            snapMode,
          );
        case "loopBody":
          if (!g.baseLoop) return null;
          return movePreviewLoop(g.baseLoop, (clientX - g.x0) * ticksPerPixel, gridMeta, snapMode);
        default:
          return null;
      }
    }

    function handleMouseMove(e: MouseEvent) {
      const g = gestureRef.current;
      if (!g) return;
      if (g.axis === "none") {
        g.axis = resolveDragAxis(e.clientX - g.x0, e.clientY - g.y0);
        if (g.axis === "none") return;
        setDragAxis(g.axis);
      }
      if (g.axis === "vertical") {
        // Incremental factors compose multiplicatively, so per-event deltas
        // add up to the same zoom as one big drag.
        onZoom(g.anchorPx, dragZoomFactor(e.clientY - g.lastY));
        g.lastY = e.clientY;
        return;
      }
      if (g.zone === "scrub") {
        onScrollChange(Math.max(0, g.startScroll - (e.clientX - g.x0)));
        return;
      }
      setLoopDragRange(loopRangeFor(g, e.clientX));
    }

    function handleMouseUp(e: MouseEvent) {
      const g = gestureRef.current;
      gestureRef.current = null;
      setDragging(false);
      setDragAxis("none");
      setLoopDragRange(null);
      if (!g) return;
      if (g.axis === "none") {
        // Zero-move click.
        if (g.zone === "scrub") {
          onSeek(Math.round(tickAt(e.clientX)));
        } else if (g.zone === "loopStrip") {
          // One-bar (one snap unit) loop at the clicked position.
          const r = dragPreviewLoop(g.anchorTick, g.anchorTick, gridMeta, snapMode);
          onLoopDrag(r.start, r.end);
        }
        // Clicks on the band or its handles leave the loop untouched.
        return;
      }
      if (g.axis === "horizontal" && g.zone !== "scrub") {
        const r = loopRangeFor(g, e.clientX);
        if (r) onLoopDrag(r.start, r.end);
      }
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    // gridMeta is rebuilt per render from ticksPerBeat/beatsPerBar.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dragging, scrollLeft, ticksPerPixel, ticksPerBeat, beatsPerBar, snapMode, loop, loopEnabled, onLoopDrag, onScrollChange, onSeek, onZoom]);

  const cursor = dragging
    ? dragAxis === "vertical"
      ? "ns-resize"
      : DRAG_CURSOR[gestureRef.current?.zone ?? "scrub"]
    : HOVER_CURSOR[hoverZone ?? "scrub"];

  return (
    <div
      ref={containerRef}
      className={styles.rulerContainer}
      onMouseDown={handleMouseDown}
      onMouseMove={(e) => {
        if (!dragging) setHoverZone(zoneAt(e.clientX, e.clientY));
      }}
      onMouseLeave={() => setHoverZone(null)}
      style={{ cursor }}
    >
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
