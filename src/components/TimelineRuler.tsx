import { useRef, useEffect, useCallback, useState } from "react";
import styles from "./TimelineRuler.module.css";

interface TimelineRulerProps {
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  onSeek: (tick: number) => void;
  onScrollChange: (scrollLeft: number) => void;
}

export function TimelineRuler({ ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar, onSeek, onScrollChange }: TimelineRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [dragging, setDragging] = useState(false);
  const dragRef = useRef<{ startX: number; startScroll: number } | null>(null);
  const clickGuardRef = useRef(false);

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
  }, [ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar]);

  useEffect(() => {
    draw();
    const obs = new ResizeObserver(draw);
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [draw]);

  function handleMouseDown(e: React.MouseEvent) {
    e.preventDefault();
    clickGuardRef.current = false;
    dragRef.current = { startX: e.clientX, startScroll: scrollLeft };
    setDragging(true);
  }

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

  return (
    <div
      ref={containerRef}
      className={styles.rulerContainer}
      onMouseDown={handleMouseDown}
      style={{ cursor: dragging ? "grabbing" : "grab" }}
    >
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
