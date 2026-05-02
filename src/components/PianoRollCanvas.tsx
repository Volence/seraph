import { useRef, useEffect, useCallback, useState } from "react";
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
  onAudition: (pitch: number) => void;
  onNoteResize: (index: number, newDurationTicks: number) => void;
  onScrollTopChange: (scrollTop: number) => void;
  playheadTick: number;
  playing: boolean;
}

const EDGE_THRESHOLD = 6;

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
  onAudition,
  onNoteResize,
  onScrollTopChange,
  playheadTick,
  playing,
}: PianoRollCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ noteIndex: number; startX: number; origDuration: number } | null>(null);

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

    if (playheadTick >= 0 && playheadTick <= durationTicks) {
      const px = playheadTick / ticksPerPixel;
      ctx.strokeStyle = "#ffffff";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(px, 0);
      ctx.lineTo(px, canvasHeight);
      ctx.stroke();
    }
  }, [notes, minPitch, maxPitch, durationTicks, ticksPerPixel, rowHeight, gridSnapTicks, channelColor, selectedNotes, canvasWidth, canvasHeight, totalNotes, playheadTick]);

  const animRef = useRef(0);
  useEffect(() => {
    function animate() {
      draw();
      if (playing) {
        animRef.current = requestAnimationFrame(animate);
      }
    }
    draw();
    if (playing) {
      animRef.current = requestAnimationFrame(animate);
    }
    return () => cancelAnimationFrame(animRef.current);
  }, [draw, playing]);

  function findNoteAtPos(x: number, y: number): { index: number; nearEdge: boolean } | null {
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;

    for (let i = 0; i < notes.length; i++) {
      const n = notes[i];
      if (n.pitch !== clickPitch) continue;
      const nx = n.tick / ticksPerPixel;
      const nw = Math.max(2, n.durationTicks / ticksPerPixel);
      if (x >= nx && x <= nx + nw) {
        const nearEdge = x >= nx + nw - EDGE_THRESHOLD;
        return { index: i, nearEdge };
      }
    }
    return null;
  }

  function handleMouseDown(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findNoteAtPos(x, y);
    if (hit && hit.nearEdge) {
      e.preventDefault();
      setDrag({ noteIndex: hit.index, startX: e.clientX, origDuration: notes[hit.index].durationTicks });
      return;
    }

    if (hit) {
      onNoteClick(hit.index);
      onAudition(notes[hit.index].pitch);
      return;
    }

    const clickTick = x * ticksPerPixel;
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;
    const snapped = Math.floor(clickTick / gridSnapTicks) * gridSnapTicks;
    if (clickPitch >= minPitch && clickPitch <= maxPitch) {
      onNoteAdd(snapped, clickPitch);
      onAudition(clickPitch);
    }
  }

  useEffect(() => {
    if (!drag) return;

    function handleMouseMove(e: MouseEvent) {
      if (!drag) return;
      const canvas = canvasRef.current;
      if (canvas) canvas.style.cursor = "ew-resize";
      const deltaX = e.clientX - drag.startX;
      const deltaTicks = deltaX * ticksPerPixel;
      const rawDuration = drag.origDuration + deltaTicks;
      const minDuration = e.ctrlKey ? 1 : gridSnapTicks;
      const duration = e.ctrlKey
        ? Math.max(1, Math.round(rawDuration))
        : Math.max(gridSnapTicks, Math.round(rawDuration / gridSnapTicks) * gridSnapTicks);
      onNoteResize(drag.noteIndex, Math.max(minDuration, duration));
    }

    function handleMouseUp() {
      setDrag(null);
      const canvas = canvasRef.current;
      if (canvas) canvas.style.cursor = "";
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [drag, ticksPerPixel, gridSnapTicks, onNoteResize]);

  function handleMouseMove(e: React.MouseEvent) {
    if (drag) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findNoteAtPos(x, y);
    canvas.style.cursor = hit?.nearEdge ? "ew-resize" : "";
  }

  function handleScroll() {
    if (containerRef.current) {
      onScrollTopChange(containerRef.current.scrollTop);
    }
  }

  return (
    <div ref={containerRef} className={styles.container} onScroll={handleScroll}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
      />
    </div>
  );
}
