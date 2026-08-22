import { useRef, useEffect, useCallback, useState } from "react";
import { rulerMarks } from "../utils/rulerMarks";
import { resolveDragAxis, dragZoomFactor, type DragAxis } from "../utils/zoomDrag";
import styles from "./PianoRollRuler.module.css";

/**
 * Measure ruler across the top of the piano-roll grid. Labels are ABSOLUTE
 * song bar numbers (matching the arrangement ruler and the "Bars N-M"
 * header), derived from ticksPerBar via rulerMarks — the region's start
 * tick offsets the view, it never renumbers the bars.
 *
 * Grammar (shared with the arrangement ruler, dominant-axis locked):
 * vertical drag zooms the time axis around the grab x; horizontal drag
 * scrolls; a zero-move click seeks to the clicked position (clamped to
 * the open region).
 */
export interface PianoRollRulerProps {
  /** Absolute song tick of the open region's start. */
  regionStartTick: number;
  regionDurationTicks: number;
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  /** Click: seek to this ABSOLUTE song tick (clamped to the region). */
  onSeek: (tick: number) => void;
  /** Vertical drag increment: rescale time around `anchorPx` by `factor`. */
  onZoom: (anchorPx: number, factor: number) => void;
  onScrollChange: (scrollLeft: number) => void;
}

interface RulerGesture {
  x0: number;
  y0: number;
  anchorPx: number;
  lastY: number;
  startScroll: number;
  axis: DragAxis;
}

export function PianoRollRuler({
  regionStartTick,
  regionDurationTicks,
  ticksPerPixel,
  scrollLeft,
  ticksPerBeat,
  beatsPerBar,
  onSeek,
  onZoom,
  onScrollChange,
}: PianoRollRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const gestureRef = useRef<RulerGesture | null>(null);
  const [dragging, setDragging] = useState(false);
  const [dragAxis, setDragAxis] = useState<DragAxis>("none");

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

    // The view maps local region pixels to ABSOLUTE song ticks.
    const absStart = regionStartTick + scrollLeft * ticksPerPixel;
    const absEnd = absStart + w * ticksPerPixel;

    ctx.fillStyle = "#888888";
    ctx.font = "10px sans-serif";

    for (const mark of rulerMarks(absStart, absEnd, gridMeta, ticksPerPixel)) {
      const x = (mark.tick - regionStartTick) / ticksPerPixel - scrollLeft;
      if (mark.kind === "bar") {
        ctx.strokeStyle = "#555555";
        ctx.beginPath();
        ctx.moveTo(x, h - 7);
        ctx.lineTo(x, h);
        ctx.stroke();
        if (mark.labeled) ctx.fillText(`${mark.bar}`, x + 3, 11);
      } else {
        ctx.strokeStyle = "#3a3a3a";
        ctx.beginPath();
        ctx.moveTo(x, h - 4);
        ctx.lineTo(x, h);
        ctx.stroke();
      }
    }

    // Dim everything past the region's end so the ruler shows where the
    // editable range stops.
    const endX = regionDurationTicks / ticksPerPixel - scrollLeft;
    if (endX < w) {
      ctx.fillStyle = "rgba(0, 0, 0, 0.35)";
      ctx.fillRect(Math.max(0, endX), 0, w - Math.max(0, endX), h);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [regionStartTick, regionDurationTicks, ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar]);

  useEffect(() => {
    draw();
    const obs = new ResizeObserver(draw);
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [draw]);

  function handleMouseDown(e: React.MouseEvent) {
    e.preventDefault();
    const container = containerRef.current;
    if (!container) return;
    const rect = container.getBoundingClientRect();
    gestureRef.current = {
      x0: e.clientX,
      y0: e.clientY,
      anchorPx: e.clientX - rect.left,
      lastY: e.clientY,
      startScroll: scrollLeft,
      axis: "none",
    };
    setDragAxis("none");
    setDragging(true);
  }

  useEffect(() => {
    if (!dragging) return;

    function handleMouseMove(e: MouseEvent) {
      const g = gestureRef.current;
      if (!g) return;
      if (g.axis === "none") {
        g.axis = resolveDragAxis(e.clientX - g.x0, e.clientY - g.y0);
        if (g.axis === "none") return;
        setDragAxis(g.axis);
      }
      if (g.axis === "vertical") {
        onZoom(g.anchorPx, dragZoomFactor(e.clientY - g.lastY));
        g.lastY = e.clientY;
        return;
      }
      onScrollChange(Math.max(0, g.startScroll - (e.clientX - g.x0)));
    }

    function handleMouseUp(e: MouseEvent) {
      const g = gestureRef.current;
      gestureRef.current = null;
      setDragging(false);
      setDragAxis("none");
      if (!g) return;
      if (g.axis === "none") {
        // Zero-move click: seek to the ABSOLUTE tick under the pointer,
        // clamped to the open region's range.
        const container = containerRef.current;
        if (!container) return;
        const rect = container.getBoundingClientRect();
        const localTick = (e.clientX - rect.left + g.startScroll) * ticksPerPixel;
        const clamped = Math.max(0, Math.min(localTick, regionDurationTicks));
        onSeek(Math.round(regionStartTick + clamped));
      }
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging, ticksPerPixel, regionStartTick, regionDurationTicks, onZoom, onScrollChange, onSeek]);

  const cursor = dragging
    ? dragAxis === "vertical"
      ? "ns-resize"
      : "grabbing"
    : "grab";

  return (
    <div
      ref={containerRef}
      className={styles.rulerContainer}
      onMouseDown={handleMouseDown}
      style={{ cursor }}
    >
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
