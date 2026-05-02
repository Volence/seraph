import { useRef, useEffect, useCallback } from "react";
import styles from "./TimelineRuler.module.css";

interface TimelineRulerProps {
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  onSeek: (tick: number) => void;
}

export function TimelineRuler({ ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar, onSeek }: TimelineRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

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

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const tick = (x + scrollLeft) * ticksPerPixel;
    onSeek(Math.max(0, Math.round(tick)));
  }

  return (
    <div ref={containerRef} className={styles.rulerContainer} onClick={handleClick}>
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
