import { useRef, useEffect, useCallback, useState } from "react";
import type { Note } from "../types/model";
import styles from "./PianoRollCanvas.module.css";

interface PianoRollCanvasProps {
  notes: Note[];
  minPitch: number;
  maxPitch: number;
  durationTicks: number;
  ticksPerPixel: number;
  scrollLeft: number;
  rowHeight: number;
  gridSnapTicks: number;
  channelColor: string;
  selectedNotes: Set<number>;
  onNoteClick: (index: number) => void;
  onNoteAdd: (tick: number, pitch: number, durationTicks: number) => void;
  onAudition: (pitch: number) => void;
  onNoteResize: (index: number, newDurationTicks: number) => void;
  onScrollTopChange: (scrollTop: number) => void;
  onScrollLeftChange: (scrollLeft: number) => void;
  onZoom: (delta: number, centerX: number) => void;
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
  scrollLeft,
  rowHeight,
  gridSnapTicks,
  channelColor,
  selectedNotes,
  onNoteClick,
  onNoteAdd,
  onAudition,
  onNoteResize,
  onScrollTopChange,
  onScrollLeftChange,
  onZoom,
  playheadTick,
  playing,
}: PianoRollCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ noteIndex: number; startX: number; origDuration: number } | null>(null);
  const drawingRef = useRef<{ startTick: number; pitch: number; endTick: number } | null>(null);
  const [isDrawing, setIsDrawing] = useState(false);
  const [isPanning, setIsPanning] = useState(false);
  const panRef = useRef<{ startX: number; startY: number; startScrollLeft: number; startScrollTop: number } | null>(null);

  const totalNotes = maxPitch - minPitch + 1;
  const canvasHeight = totalNotes * rowHeight;

  function pixelToTick(px: number): number {
    return (px + scrollLeft) * ticksPerPixel;
  }

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    const viewW = rect.width;
    const viewH = Math.max(rect.height, canvasHeight);

    canvas.width = viewW * dpr;
    canvas.height = viewH * dpr;
    canvas.style.width = `${viewW}px`;
    canvas.style.height = `${viewH}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const startTick = scrollLeft * ticksPerPixel;

    for (let i = 0; i < totalNotes; i++) {
      const pitch = maxPitch - i;
      const y = i * rowHeight;
      ctx.fillStyle = isBlackKey(pitch) ? "#1a1a1a" : "#1e1e1e";
      ctx.fillRect(0, y, viewW, rowHeight);
      ctx.strokeStyle = "#2a2a2a";
      ctx.beginPath();
      ctx.moveTo(0, y + rowHeight);
      ctx.lineTo(viewW, y + rowHeight);
      ctx.stroke();
    }

    const gridPx = gridSnapTicks / ticksPerPixel;
    if (gridPx > 4) {
      ctx.strokeStyle = "#2a2a2a";
      const firstGrid = Math.floor(startTick / gridSnapTicks) * gridSnapTicks;
      for (let tick = firstGrid; tick < startTick + viewW * ticksPerPixel; tick += gridSnapTicks) {
        const x = (tick - startTick) / ticksPerPixel;
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, viewH);
        ctx.stroke();
      }
    }

    for (let i = 0; i < notes.length; i++) {
      const note = notes[i];
      if (note.pitch < minPitch || note.pitch > maxPitch) continue;
      const x = (note.tick - startTick / 1) / ticksPerPixel;
      const noteEndPx = (note.tick + note.durationTicks - startTick) / ticksPerPixel;
      if (noteEndPx < 0 || x > viewW) continue;
      const w = Math.max(2, note.durationTicks / ticksPerPixel);
      const drawX = (note.tick - startTick) / ticksPerPixel;
      const row = maxPitch - note.pitch;
      const y = row * rowHeight + 1;
      const h = rowHeight - 2;

      const selected = selectedNotes.has(i);
      ctx.fillStyle = selected ? channelColor : channelColor + "cc";
      ctx.fillRect(drawX, y, w, h);
      ctx.strokeStyle = selected ? "#ffffff" : channelColor;
      ctx.lineWidth = 1;
      ctx.strokeRect(drawX + 0.5, y + 0.5, w - 1, h - 1);

      if (note.modulation && w > 8) {
        ctx.strokeStyle = "#ffffff44";
        ctx.lineWidth = 1;
        ctx.beginPath();
        const midY = y + h / 2;
        for (let px = drawX + 2; px < drawX + w - 2; px += 3) {
          const wy = midY + Math.sin((px - drawX) * 0.8) * (h * 0.2);
          if (px === drawX + 2) ctx.moveTo(px, wy);
          else ctx.lineTo(px, wy);
        }
        ctx.stroke();
      }

      if (note.detune && note.detune !== 0 && w > 14 && h >= 10) {
        ctx.fillStyle = "#ffffffaa";
        ctx.font = `${Math.min(9, h - 2)}px monospace`;
        ctx.fillText(note.detune > 0 ? `+${note.detune}` : `${note.detune}`, drawX + 2, y + h - 2);
      }
    }

    const dr = drawingRef.current;
    if (dr) {
      const x = (dr.startTick - startTick) / ticksPerPixel;
      const dur = Math.max(gridSnapTicks, dr.endTick - dr.startTick);
      const w = Math.max(2, dur / ticksPerPixel);
      const row = maxPitch - dr.pitch;
      const y = row * rowHeight + 1;
      const h = rowHeight - 2;
      ctx.fillStyle = channelColor + "88";
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = channelColor;
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);
      ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
      ctx.setLineDash([]);
    }

    if (playheadTick >= 0 && playheadTick <= durationTicks) {
      const px = (playheadTick - startTick) / ticksPerPixel;
      if (px >= 0 && px <= viewW) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(px, 0);
        ctx.lineTo(px, viewH);
        ctx.stroke();
      }
    }
  }, [notes, minPitch, maxPitch, durationTicks, ticksPerPixel, scrollLeft, rowHeight, gridSnapTicks, channelColor, selectedNotes, canvasHeight, totalNotes, playheadTick]);

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

  function findNoteAtPos(viewX: number, y: number): { index: number; nearEdge: boolean } | null {
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;
    const startTick = scrollLeft * ticksPerPixel;

    for (let i = 0; i < notes.length; i++) {
      const n = notes[i];
      if (n.pitch !== clickPitch) continue;
      const nx = (n.tick - startTick) / ticksPerPixel;
      const nw = Math.max(2, n.durationTicks / ticksPerPixel);
      if (viewX >= nx && viewX <= nx + nw) {
        const nearEdge = viewX >= nx + nw - EDGE_THRESHOLD;
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

    // Any click on empty space: pan
    e.preventDefault();
    const container = containerRef.current;
    panRef.current = {
      startX: e.clientX,
      startY: e.clientY,
      startScrollLeft: scrollLeft,
      startScrollTop: container?.scrollTop ?? 0,
    };
    setIsPanning(true);
  }

  function handleDoubleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findNoteAtPos(x, y);
    if (hit) return;

    const clickTick = pixelToTick(x);
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;
    const snapped = Math.floor(clickTick / gridSnapTicks) * gridSnapTicks;
    if (clickPitch >= minPitch && clickPitch <= maxPitch) {
      e.preventDefault();
      drawingRef.current = { startTick: snapped, pitch: clickPitch, endTick: snapped + gridSnapTicks };
      setIsDrawing(true);
      onAudition(clickPitch);
    }
  }

  // Panning effect
  useEffect(() => {
    if (!isPanning) return;

    function handleMouseMove(e: MouseEvent) {
      const p = panRef.current;
      if (!p) return;
      const dx = e.clientX - p.startX;
      const dy = e.clientY - p.startY;
      onScrollLeftChange(Math.max(0, p.startScrollLeft - dx));
      const container = containerRef.current;
      if (container) container.scrollTop = Math.max(0, p.startScrollTop - dy);
    }

    function handleMouseUp() {
      panRef.current = null;
      setIsPanning(false);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isPanning, onScrollLeftChange]);

  useEffect(() => {
    if (!isDrawing) return;

    function handleMouseMove(e: MouseEvent) {
      const canvas = canvasRef.current;
      if (!canvas || !drawingRef.current) return;
      canvas.style.cursor = "crosshair";
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const rawEndTick = pixelToTick(x);
      const snappedEnd = Math.ceil(rawEndTick / gridSnapTicks) * gridSnapTicks;
      drawingRef.current.endTick = Math.max(
        drawingRef.current.startTick + gridSnapTicks,
        snappedEnd,
      );
      draw();
    }

    function handleMouseUp() {
      const dr = drawingRef.current;
      if (dr) {
        const duration = Math.max(gridSnapTicks, dr.endTick - dr.startTick);
        onNoteAdd(dr.startTick, dr.pitch, duration);
      }
      drawingRef.current = null;
      setIsDrawing(false);
      const canvas = canvasRef.current;
      if (canvas) canvas.style.cursor = "";
      draw();
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isDrawing, ticksPerPixel, scrollLeft, gridSnapTicks, onNoteAdd, draw]);

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
    if (drag || isDrawing || isPanning) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findNoteAtPos(x, y);
    canvas.style.cursor = hit?.nearEdge ? "ew-resize" : "";
  }

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function handleWheel(e: WheelEvent) {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const rect = canvasRef.current?.getBoundingClientRect();
        const centerX = rect ? e.clientX - rect.left : 0;
        onZoom(e.deltaY, centerX);
      } else if (e.shiftKey) {
        e.preventDefault();
        onScrollLeftChange(Math.max(0, scrollLeft + e.deltaY));
      } else if (e.deltaX !== 0) {
        e.preventDefault();
        onScrollLeftChange(Math.max(0, scrollLeft + e.deltaX));
      }
    }

    container.addEventListener("wheel", handleWheel, { passive: false });
    return () => container.removeEventListener("wheel", handleWheel);
  }, [scrollLeft, onScrollLeftChange, onZoom]);

  function handleVerticalScroll() {
    if (containerRef.current) {
      onScrollTopChange(containerRef.current.scrollTop);
    }
  }

  function handleContextMenu(e: React.MouseEvent) {
    e.preventDefault();
  }

  return (
    <div
      ref={containerRef}
      className={styles.container}
      onScroll={handleVerticalScroll}
      onContextMenu={handleContextMenu}
    >
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onDoubleClick={handleDoubleClick}
      />
    </div>
  );
}
