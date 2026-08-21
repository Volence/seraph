import { useRef, useEffect, useCallback } from "react";
import type { Note } from "../types/model";
import styles from "./VelocityLane.module.css";

interface VelocityLaneProps {
  notes: Note[];
  durationTicks: number;
  ticksPerPixel: number;
  scrollLeft: number;
  channelColor: string;
  /** Width of the piano-roll key column; the lane is offset by this so
   *  velocity bars sit directly under their notes (G5). */
  keysWidth: number;
  onVelocityChange: (noteIndex: number, velocity: number) => void;
}

const LANE_HEIGHT = 60;

export function VelocityLane({ notes, durationTicks, ticksPerPixel, scrollLeft, channelColor, keysWidth, onVelocityChange }: VelocityLaneProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;
    const dpr = window.devicePixelRatio || 1;
    const viewW = container.getBoundingClientRect().width;
    canvas.width = viewW * dpr;
    canvas.height = LANE_HEIGHT * dpr;
    canvas.style.width = `${viewW}px`;
    canvas.style.height = `${LANE_HEIGHT}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const startTick = scrollLeft * ticksPerPixel;

    ctx.fillStyle = "#1a1a1a";
    ctx.fillRect(0, 0, viewW, LANE_HEIGHT);

    for (const note of notes) {
      const x = (note.tick - startTick) / ticksPerPixel;
      const w = Math.max(4, note.durationTicks / ticksPerPixel * 0.8);
      if (x + w < 0 || x > viewW) continue;
      const barHeight = (note.velocity / 127) * (LANE_HEIGHT - 4);
      const alpha = 0.5 + (note.velocity / 127) * 0.5;
      ctx.fillStyle = channelColor + Math.round(alpha * 255).toString(16).padStart(2, "0");
      ctx.fillRect(x, LANE_HEIGHT - barHeight - 2, w, barHeight);
    }
  }, [notes, durationTicks, ticksPerPixel, scrollLeft, channelColor]);

  useEffect(() => { draw(); }, [draw]);

  function handleMouseDown(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const startTick = scrollLeft * ticksPerPixel;

    for (let i = 0; i < notes.length; i++) {
      const nx = (notes[i].tick - startTick) / ticksPerPixel;
      const nw = Math.max(4, notes[i].durationTicks / ticksPerPixel * 0.8);
      if (x >= nx && x <= nx + nw) {
        const vel = Math.round((1 - y / LANE_HEIGHT) * 127);
        onVelocityChange(i, Math.max(1, Math.min(127, vel)));
        return;
      }
    }
  }

  return (
    <div ref={containerRef} className={styles.lane} style={{ marginLeft: keysWidth }}>
      <canvas ref={canvasRef} onMouseDown={handleMouseDown} />
    </div>
  );
}
