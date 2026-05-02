import { useRef, useEffect, useCallback } from "react";
import type { Note } from "../types/model";
import styles from "./PianoRollCanvas.module.css";

interface PianoRollCanvasProps {
  notes: Note[];
  minPitch: number;
  maxPitch: number;
  durationTicks: number;
  ticksPerPixel: number;
  rowHeight: number;
  gridSnapTicks: number;
  channelColor: string;
  selectedNotes: Set<number>;
  onNoteClick: (index: number) => void;
  onNoteAdd: (tick: number, pitch: number) => void;
  onScrollTopChange: (scrollTop: number) => void;
}

function isBlackKey(pitch: number): boolean {
  return [1, 3, 6, 8, 10].includes(pitch % 12);
}

export function PianoRollCanvas({
  notes,
  minPitch,
  maxPitch,
  durationTicks,
  ticksPerPixel,
  rowHeight,
  gridSnapTicks,
  channelColor,
  selectedNotes,
  onNoteClick,
  onNoteAdd,
  onScrollTopChange,
}: PianoRollCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNotes = maxPitch - minPitch + 1;
  const canvasWidth = durationTicks / ticksPerPixel;
  const canvasHeight = totalNotes * rowHeight;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasWidth * dpr;
    canvas.height = canvasHeight * dpr;
    canvas.style.width = `${canvasWidth}px`;
    canvas.style.height = `${canvasHeight}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    for (let i = 0; i < totalNotes; i++) {
      const pitch = maxPitch - i;
      const y = i * rowHeight;
      ctx.fillStyle = isBlackKey(pitch) ? "#1a1a1a" : "#1e1e1e";
      ctx.fillRect(0, y, canvasWidth, rowHeight);
      ctx.strokeStyle = "#2a2a2a";
      ctx.beginPath();
      ctx.moveTo(0, y + rowHeight);
      ctx.lineTo(canvasWidth, y + rowHeight);
      ctx.stroke();
    }

    const gridPx = gridSnapTicks / ticksPerPixel;
    if (gridPx > 4) {
      ctx.strokeStyle = "#2a2a2a";
      for (let x = 0; x < canvasWidth; x += gridPx) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, canvasHeight);
        ctx.stroke();
      }
    }

    for (let i = 0; i < notes.length; i++) {
      const note = notes[i];
      if (note.pitch < minPitch || note.pitch > maxPitch) continue;
      const x = note.tick / ticksPerPixel;
      const w = Math.max(2, note.durationTicks / ticksPerPixel);
      const row = maxPitch - note.pitch;
      const y = row * rowHeight + 1;
      const h = rowHeight - 2;

      const selected = selectedNotes.has(i);
      ctx.fillStyle = selected ? channelColor : channelColor + "cc";
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = selected ? "#ffffff" : channelColor;
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
    }
  }, [notes, minPitch, maxPitch, durationTicks, ticksPerPixel, rowHeight, gridSnapTicks, channelColor, selectedNotes, canvasWidth, canvasHeight, totalNotes]);

  useEffect(() => { draw(); }, [draw]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const clickTick = x * ticksPerPixel;
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;

    for (let i = 0; i < notes.length; i++) {
      const n = notes[i];
      if (n.pitch !== clickPitch) continue;
      const nx = n.tick / ticksPerPixel;
      const nw = n.durationTicks / ticksPerPixel;
      if (x >= nx && x <= nx + nw) {
        onNoteClick(i);
        return;
      }
    }

    const snapped = Math.floor(clickTick / gridSnapTicks) * gridSnapTicks;
    if (clickPitch >= minPitch && clickPitch <= maxPitch) {
      onNoteAdd(snapped, clickPitch);
    }
  }

  function handleScroll() {
    if (containerRef.current) {
      onScrollTopChange(containerRef.current.scrollTop);
    }
  }

  return (
    <div ref={containerRef} className={styles.container} onScroll={handleScroll}>
      <canvas ref={canvasRef} className={styles.canvas} onClick={handleClick} />
    </div>
  );
}
