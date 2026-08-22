import { useRef, useEffect, useCallback, useState } from "react";
import type { SnapMode } from "../utils/grid";
import { dragPreviewLoop, type PreviewLoopRange } from "../utils/previewLoop";
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
  /** Upper-half drag released: commit this preview-loop range. */
  onLoopDrag: (startTick: number, endTick: number) => void;
}

// Accent for the loop band; matches the FM region blue used elsewhere.
const LOOP_COLOR = "#4a9eff";

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
}: TimelineRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const dragRef = useRef<{ startX: number; startScroll: number } | null>(null);
  const clickGuardRef = useRef(false);
  // Gesture split: the upper half of the ruler is the loop strip (drag paints
  // a preview-loop range), the lower half keeps the original drag-to-scroll /
  // click-to-seek. The halves never overlap, so neither gesture collides
  // with the other; wheel scrolling is unaffected.
  const [loopDragging, setLoopDragging] = useState(false);
  const loopAnchorRef = useRef(0);
  const [loopDragRange, setLoopDragRange] = useState<PreviewLoopRange | null>(null);
  const [hoverLoopStrip, setHoverLoopStrip] = useState(false);

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

    const ticksPerBar = ticksPerBeat * beatsPerBar;
    const startTick = scrollLeft * ticksPerPixel;
    const endTick = startTick + w * ticksPerPixel;

    const firstBar = Math.floor(startTick / ticksPerBar);
    const lastBar = Math.ceil(endTick / ticksPerBar);

    ctx.fillStyle = "#888888";
    ctx.font = "10px sans-serif";

    for (let bar = firstBar; bar <= lastBar; bar++) {
      const tick = bar * ticksPerBar;
      const x = (tick - startTick) / ticksPerPixel;

      ctx.strokeStyle = "#555555";
      ctx.beginPath();
      ctx.moveTo(x, h - 8);
      ctx.lineTo(x, h);
      ctx.stroke();

      ctx.fillText(`${bar + 1}`, x + 3, 12);

      if (ticksPerPixel < ticksPerBeat) {
        for (let beat = 1; beat < beatsPerBar; beat++) {
          const bx = ((tick + beat * ticksPerBeat) - startTick) / ticksPerPixel;
          ctx.strokeStyle = "#3a3a3a";
          ctx.beginPath();
          ctx.moveTo(bx, h - 4);
          ctx.lineTo(bx, h);
          ctx.stroke();
        }
      }
    }

    // Preview-loop bracket: the live drag range while painting, otherwise
    // the committed range whenever the loop is enabled.
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
        // Top rail + down-hooks at both ends (classic loop bracket).
        ctx.beginPath();
        ctx.moveTo(x1 + 1, bandH + 4);
        ctx.lineTo(x1 + 1, 1);
        ctx.lineTo(x2 - 1, 1);
        ctx.lineTo(x2 - 1, bandH + 4);
        ctx.stroke();
      }
    }
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

  function inLoopStrip(clientY: number): boolean {
    const container = containerRef.current;
    if (!container) return false;
    const rect = container.getBoundingClientRect();
    return clientY - rect.top < rect.height / 2;
  }

  function handleMouseDown(e: React.MouseEvent) {
    e.preventDefault();
    if (inLoopStrip(e.clientY)) {
      const anchor = tickAt(e.clientX);
      loopAnchorRef.current = anchor;
      setLoopDragRange(dragPreviewLoop(anchor, anchor, gridMeta, snapMode));
      setLoopDragging(true);
      return;
    }
    clickGuardRef.current = false;
    dragRef.current = { startX: e.clientX, startScroll: scrollLeft };
    setDragging(true);
  }

  // Loop-strip drag: paint the snapped range live, commit on release.
  useEffect(() => {
    if (!loopDragging) return;

    function rangeAt(clientX: number): PreviewLoopRange {
      return dragPreviewLoop(loopAnchorRef.current, tickAt(clientX), gridMeta, snapMode);
    }

    function handleMouseMove(e: MouseEvent) {
      setLoopDragRange(rangeAt(e.clientX));
    }

    function handleMouseUp(e: MouseEvent) {
      const range = rangeAt(e.clientX);
      setLoopDragging(false);
      setLoopDragRange(null);
      onLoopDrag(range.start, range.end);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
    // gridMeta is rebuilt per render from ticksPerBeat/beatsPerBar.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [loopDragging, scrollLeft, ticksPerPixel, ticksPerBeat, beatsPerBar, snapMode, onLoopDrag]);

  useEffect(() => {
    if (!dragging) return;

    function handleMouseMove(e: MouseEvent) {
      const d = dragRef.current;
      if (!d) return;
      const delta = e.clientX - d.startX;
      if (Math.abs(delta) > 3) clickGuardRef.current = true;
      onScrollChange(Math.max(0, d.startScroll - delta));
    }

    function handleMouseUp(e: MouseEvent) {
      if (!clickGuardRef.current) {
        const canvas = canvasRef.current;
        if (canvas) {
          const rect = canvas.getBoundingClientRect();
          const x = e.clientX - rect.left;
          const tick = (x + scrollLeft) * ticksPerPixel;
          onSeek(Math.max(0, Math.round(tick)));
        }
      }
      dragRef.current = null;
      setDragging(false);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging, scrollLeft, ticksPerPixel, onScrollChange, onSeek]);

  const cursor = dragging
    ? "grabbing"
    : loopDragging || hoverLoopStrip
      ? "col-resize"
      : "grab";

  return (
    <div
      ref={containerRef}
      className={styles.rulerContainer}
      onMouseDown={handleMouseDown}
      onMouseMove={(e) => {
        if (!dragging && !loopDragging) setHoverLoopStrip(inLoopStrip(e.clientY));
      }}
      onMouseLeave={() => setHoverLoopStrip(false)}
      style={{ cursor }}
    >
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
