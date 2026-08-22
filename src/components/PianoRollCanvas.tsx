import { useRef, useEffect, useCallback, useState } from "react";
import type { Note } from "../types/model";
import {
  notesIntersectingRect,
  marqueeRectFromView,
  marqueePreviewSelection,
} from "../utils/pianoRollEdit";
import { followScrollLeft, followAllowed } from "../utils/followPlayhead";
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
  onNoteClick: (index: number, additive: boolean) => void;
  onMarqueeSelect: (indices: number[], additive: boolean) => void;
  onClearSelection: () => void;
  onNoteAdd: (tick: number, pitch: number, durationTicks: number) => void;
  onAudition: (pitch: number) => void;
  onNoteResize: (index: number, newDurationTicks: number) => void;
  onNotesMove: (moves: { index: number; tick: number; pitch: number }[]) => void;
  /** Continuous drag gesture (note move/resize) began — mousedown on a note. */
  onGestureStart?: () => void;
  /** The drag gesture ended (mouseup). Always paired with onGestureStart. */
  onGestureEnd?: () => void;
  onScrollTopChange: (scrollTop: number) => void;
  onScrollLeftChange: (scrollLeft: number) => void;
  onZoom: (delta: number, centerX: number) => void;
  playheadTick: number;
  playing: boolean;
  /** Suppress follow-playhead paging entirely (set while a preview loop is
   *  active — owner ruling: a looping playback must not move the view). */
  suppressFollow?: boolean;
}

const EDGE_THRESHOLD = 6;
// A press-drag shorter than this (px) counts as a plain click, not a drag.
const CLICK_SLOP = 4;

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
  onMarqueeSelect,
  onClearSelection,
  onNoteAdd,
  onAudition,
  onNoteResize,
  onNotesMove,
  onGestureStart,
  onGestureEnd,
  onScrollTopChange,
  onScrollLeftChange,
  onZoom,
  playheadTick,
  playing,
  suppressFollow = false,
}: PianoRollCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const [drag, setDrag] = useState<{ noteIndex: number; startX: number; origDuration: number } | null>(null);
  const [moveDrag, setMoveDrag] = useState<{
    startX: number;
    startY: number;
    anchorTick: number;
    // Positions at drag start of every note the drag moves (the whole
    // selection when the pressed note was already selected).
    targets: { index: number; tick: number; pitch: number }[];
  } | null>(null);
  const drawingRef = useRef<{ startTick: number; pitch: number; endTick: number } | null>(null);
  const [isDrawing, setIsDrawing] = useState(false);
  const [isPanning, setIsPanning] = useState(false);
  const panRef = useRef<{ startX: number; startY: number; startScrollLeft: number; startScrollTop: number } | null>(null);
  // Marquee (rubber-band) selection; x in view px, y in canvas/content px.
  const marqueeRef = useRef<{ startX: number; startY: number; currentX: number; currentY: number; additive: boolean } | null>(null);
  const [isMarquee, setIsMarquee] = useState(false);
  // Live preview of what the in-flight marquee will select on mouseup:
  // while non-null, draw() highlights these indices instead of
  // selectedNotes. The actual selection still commits on mouseup.
  const marqueePreviewRef = useRef<Set<number> | null>(null);

  const totalNotes = maxPitch - minPitch + 1;
  const canvasHeight = totalNotes * rowHeight;
  // Manual scrolls (wheel, pan) suspend follow-playhead briefly (G28).
  const lastManualScrollRef = useRef(-Infinity);

  useEffect(() => {
    if (playheadTick < 0) return;
    if (!followAllowed(playing, suppressFollow, lastManualScrollRef.current, performance.now())) return;
    const container = containerRef.current;
    if (!container) return;
    const viewWidth = container.getBoundingClientRect().width;
    const next = followScrollLeft(playheadTick / ticksPerPixel, scrollLeft, viewWidth);
    if (next !== null) onScrollLeftChange(next);
  }, [playheadTick, playing, suppressFollow, ticksPerPixel, scrollLeft, onScrollLeftChange]);

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

    // During a marquee drag the preview set drives the highlight so the
    // user sees what mouseup will select; otherwise the committed selection.
    const highlight = marqueePreviewRef.current ?? selectedNotes;

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

      const selected = highlight.has(i);
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

    const mq = marqueeRef.current;
    if (mq) {
      const x1 = Math.min(mq.startX, mq.currentX);
      const x2 = Math.max(mq.startX, mq.currentX);
      const y1 = Math.min(mq.startY, mq.currentY);
      const y2 = Math.max(mq.startY, mq.currentY);
      ctx.fillStyle = "rgba(74, 158, 255, 0.1)";
      ctx.strokeStyle = "rgba(74, 158, 255, 0.6)";
      ctx.lineWidth = 1;
      ctx.setLineDash([3, 3]);
      ctx.fillRect(x1, y1, x2 - x1, y2 - y1);
      ctx.strokeRect(x1, y1, x2 - x1, y2 - y1);
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
      onGestureStart?.();
      setDrag({ noteIndex: hit.index, startX: e.clientX, origDuration: notes[hit.index].durationTicks });
      return;
    }

    if (hit) {
      e.preventDefault();
      onGestureStart?.();
      const additive = e.shiftKey;
      const wasSelected = selectedNotes.has(hit.index);
      // A plain press on an unselected note selects it (replacing the
      // selection) before any drag; a press on an already-selected note
      // keeps the selection so the whole group can be dragged.
      if (additive || !wasSelected) {
        onNoteClick(hit.index, additive);
      }
      onAudition(notes[hit.index].pitch);
      const targetIndices = wasSelected
        ? Array.from(selectedNotes)
        : additive
          ? [...selectedNotes, hit.index]
          : [hit.index];
      setMoveDrag({
        startX: e.clientX,
        startY: e.clientY,
        anchorTick: notes[hit.index].tick,
        targets: targetIndices
          .filter((i) => notes[i])
          .map((i) => ({ index: i, tick: notes[i].tick, pitch: notes[i].pitch })),
      });
      return;
    }

    // Empty space: middle button or Alt+drag pans; plain left-drag marquees.
    e.preventDefault();
    if (e.button === 1 || e.altKey) {
      const container = containerRef.current;
      panRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        startScrollLeft: scrollLeft,
        startScrollTop: container?.scrollTop ?? 0,
      };
      setIsPanning(true);
      return;
    }

    marqueeRef.current = { startX: x, startY: y, currentX: x, currentY: y, additive: e.shiftKey };
    // Seed the live preview: an empty rect hits nothing, so a plain press
    // previews the clear and Shift keeps the selection until notes hit.
    marqueePreviewRef.current = marqueePreviewSelection(selectedNotes, [], e.shiftKey);
    setIsMarquee(true);
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
      lastManualScrollRef.current = performance.now();
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

  // Marquee (rubber-band) selection effect
  useEffect(() => {
    if (!isMarquee) return;

    function handleMouseMove(e: MouseEvent) {
      const canvas = canvasRef.current;
      const mq = marqueeRef.current;
      if (!canvas || !mq) return;
      const rect = canvas.getBoundingClientRect();
      mq.currentX = e.clientX - rect.left;
      mq.currentY = e.clientY - rect.top;
      // Live preview: highlight what this rect would select on mouseup.
      const hits = notesIntersectingRect(
        notes,
        marqueeRectFromView(
          mq.startX, mq.startY, mq.currentX, mq.currentY,
          scrollLeft, ticksPerPixel, maxPitch, rowHeight,
        ),
      );
      marqueePreviewRef.current = marqueePreviewSelection(selectedNotes, hits, mq.additive);
      draw();
    }

    function handleMouseUp() {
      const mq = marqueeRef.current;
      marqueeRef.current = null;
      marqueePreviewRef.current = null;
      setIsMarquee(false);
      if (!mq) return;
      const movedEnough =
        Math.abs(mq.currentX - mq.startX) > CLICK_SLOP || Math.abs(mq.currentY - mq.startY) > CLICK_SLOP;
      if (movedEnough) {
        const hits = notesIntersectingRect(
          notes,
          marqueeRectFromView(
            mq.startX, mq.startY, mq.currentX, mq.currentY,
            scrollLeft, ticksPerPixel, maxPitch, rowHeight,
          ),
        );
        onMarqueeSelect(hits, mq.additive);
      } else if (!mq.additive) {
        // Plain click on empty grid: clear the selection.
        onClearSelection();
      }
      draw();
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [isMarquee, notes, maxPitch, rowHeight, ticksPerPixel, scrollLeft, selectedNotes, onMarqueeSelect, onClearSelection, draw]);

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
      onGestureEnd?.();
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
  }, [drag, ticksPerPixel, gridSnapTicks, onNoteResize, onGestureEnd]);

  useEffect(() => {
    if (!moveDrag) return;
    let moved = false;

    function handleMouseMove(e: MouseEvent) {
      if (!moveDrag || moveDrag.targets.length === 0) return;
      const canvas = canvasRef.current;
      if (canvas) canvas.style.cursor = "grabbing";
      const deltaX = e.clientX - moveDrag.startX;
      const deltaY = e.clientY - moveDrag.startY;
      if (Math.abs(deltaX) > 3 || Math.abs(deltaY) > 3) moved = true;
      if (!moved) return;
      // Snap the anchor note, then apply the same deltas to every target so
      // the group keeps its internal offsets.
      const rawTick = moveDrag.anchorTick + deltaX * ticksPerPixel;
      const snappedTick = e.ctrlKey
        ? Math.round(rawTick)
        : Math.round(rawTick / gridSnapTicks) * gridSnapTicks;
      let tickDelta = snappedTick - moveDrag.anchorTick;
      let pitchDelta = -Math.round(deltaY / rowHeight);
      const minTick = Math.min(...moveDrag.targets.map((t) => t.tick));
      const loPitch = Math.min(...moveDrag.targets.map((t) => t.pitch));
      const hiPitch = Math.max(...moveDrag.targets.map((t) => t.pitch));
      tickDelta = Math.max(tickDelta, -minTick);
      pitchDelta = Math.max(minPitch - loPitch, Math.min(maxPitch - hiPitch, pitchDelta));
      onNotesMove(
        moveDrag.targets.map((t) => ({ index: t.index, tick: t.tick + tickDelta, pitch: t.pitch + pitchDelta })),
      );
    }

    function handleMouseUp() {
      onGestureEnd?.();
      setMoveDrag(null);
      const canvas = canvasRef.current;
      if (canvas) canvas.style.cursor = "";
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [moveDrag, ticksPerPixel, gridSnapTicks, rowHeight, minPitch, maxPitch, onNotesMove, onGestureEnd]);

  function handleMouseMove(e: React.MouseEvent) {
    if (drag || moveDrag || isDrawing || isPanning || isMarquee) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = findNoteAtPos(x, y);
    canvas.style.cursor = hit?.nearEdge ? "ew-resize" : hit ? "grab" : "";
  }

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    function handleWheel(e: WheelEvent) {
      lastManualScrollRef.current = performance.now();
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
