import { useRef, useEffect, useCallback, useState } from "react";
import type { Track, SelectedRegion } from "../types/model";
import styles from "./TimelineCanvas.module.css";

interface TimelineCanvasProps {
  tracks: Track[];
  ticksPerPixel: number;
  scrollLeft: number;
  trackHeight: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  playbackTick: number;
  playing: boolean;
  selectedRegions: SelectedRegion[];
  onRegionClick: (trackId: string, regionId: string, ctrlKey: boolean) => void;
  onRegionDoubleClick: (trackId: string, regionId: string) => void;
  onEmptyDoubleClick: (trackId: string, tick: number, duration: number) => void;
  onSelectRegions: (regions: SelectedRegion[]) => void;
  onRegionMove: (srcTrackId: string, regionId: string, dstTrackId: string, startTick: number, tickDelta: number, trackDelta: number) => void;
  onRegionResize: (trackId: string, regionId: string, startTick: number, durationTicks: number) => void;
}

const CHANNEL_COLORS: Record<string, string> = {
  fm: "#4a9eff",
  psg: "#44cc66",
  dac: "#ff8844",
};

const EDGE_THRESHOLD = 8;

function trackChannelType(track: Track): string {
  const ch = track.channel;
  if (ch === "PsgNoise") return "psg";
  if (typeof ch === "object" && "Fm" in ch) return "fm";
  if (typeof ch === "object" && "Psg" in ch) return "psg";
  return "dac";
}

type DragMode = "select" | "create" | "move" | "resize-left" | "resize-right";

interface DragState {
  mode: DragMode;
  trackIdx: number;
  startX: number;
  startY: number;
  currentX: number;
  currentTrackIdx: number;
  regionTrackId?: string;
  regionId?: string;
  regionStartTick?: number;
  regionDuration?: number;
}

export function TimelineCanvas({
  tracks,
  ticksPerPixel,
  scrollLeft,
  trackHeight,
  ticksPerBeat,
  beatsPerBar,
  playbackTick,
  playing,
  selectedRegions,
  onRegionClick,
  onRegionDoubleClick,
  onEmptyDoubleClick,
  onSelectRegions,
  onRegionMove,
  onRegionResize,
}: TimelineCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const animRef = useRef(0);
  const [drag, setDrag] = useState<DragState | null>(null);
  const dragRef = useRef<DragState | null>(null);
  dragRef.current = drag;
  const lastEmptyClickRef = useRef<{ x: number; y: number; time: number } | null>(null);

  const ticksPerBar = ticksPerBeat * beatsPerBar;

  function pixelToTick(px: number): number {
    return px * ticksPerPixel + scrollLeft * ticksPerPixel;
  }

  function snapToBar(tick: number): number {
    return Math.floor(tick / ticksPerBar) * ticksPerBar;
  }

  function clampCreateRegion(trackIdx: number, rawStart: number, rawEnd: number): { start: number; end: number } | null {
    if (trackIdx < 0 || trackIdx >= tracks.length) return null;
    const sStart = snapToBar(Math.max(0, rawStart));
    let sEnd = Math.max(sStart + ticksPerBar, snapToBar(rawEnd) + ticksPerBar);
    const track = tracks[trackIdx];
    for (const r of track.regions) {
      if (r.startTick > sStart && r.startTick < sEnd) {
        sEnd = r.startTick;
      }
    }
    if (sEnd <= sStart) return null;
    const overlaps = track.regions.some((r) =>
      sStart < r.startTick + r.durationTicks && sEnd > r.startTick
    );
    if (overlaps) return null;
    return { start: sStart, end: sEnd };
  }

  function hitTestRegion(x: number, y: number): {
    trackIdx: number;
    trackId: string;
    regionId: string;
    edge: "left" | "right" | null;
  } | null {
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return null;
    const startTick = scrollLeft * ticksPerPixel;
    const track = tracks[trackIdx];
    for (const region of track.regions) {
      const rx = (region.startTick - startTick) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        let edge: "left" | "right" | null = null;
        if (x - rx < EDGE_THRESHOLD) edge = "left";
        else if (rx + rw - x < EDGE_THRESHOLD) edge = "right";
        return { trackIdx, trackId: track.id, regionId: region.id, edge };
      }
    }
    return null;
  }

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = Math.max(rect.height, tracks.length * trackHeight) * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${Math.max(rect.height, tracks.length * trackHeight)}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = Math.max(rect.height, tracks.length * trackHeight);
    const startTick = scrollLeft * ticksPerPixel;

    ctx.clearRect(0, 0, w, h);

    for (let i = 0; i < tracks.length; i++) {
      ctx.fillStyle = i % 2 === 0 ? "#1e1e1e" : "#222222";
      ctx.fillRect(0, i * trackHeight, w, trackHeight);
      ctx.strokeStyle = "#2a2a2a";
      ctx.beginPath();
      ctx.moveTo(0, (i + 1) * trackHeight);
      ctx.lineTo(w, (i + 1) * trackHeight);
      ctx.stroke();
    }

    const d = dragRef.current;

    for (let i = 0; i < tracks.length; i++) {
      const track = tracks[i];
      const color = CHANNEL_COLORS[trackChannelType(track)] || "#888";
      const y = i * trackHeight + 2;
      const rh = trackHeight - 4;

      for (const region of track.regions) {
        const isDraggedRegion = d && (d.mode === "resize-left" || d.mode === "resize-right")
          && d.regionTrackId === track.id && d.regionId === region.id;
        const isMovingSelected = d && d.mode === "move"
          && selectedRegions.length > 1
          && selectedRegions.some((s) => s.regionId === d.regionId)
          && selectedRegions.some((s) => s.trackId === track.id && s.regionId === region.id);
        const isMovingSingle = d && d.mode === "move"
          && d.regionTrackId === track.id && d.regionId === region.id
          && !selectedRegions.some((s) => s.regionId === d.regionId && selectedRegions.length > 1);
        const isBeingDragged = isDraggedRegion || isMovingSelected || isMovingSingle;

        if (isBeingDragged) continue;

        const x = (region.startTick - startTick) / ticksPerPixel;
        const rw = region.durationTicks / ticksPerPixel;

        if (x + rw < 0 || x > w) continue;

        ctx.fillStyle = color + "33";
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;

        const rx = Math.round(x);
        const rrw = Math.round(rw);
        ctx.fillRect(rx, y, rrw, rh);
        ctx.strokeRect(rx + 0.5, y + 0.5, rrw - 1, rh - 1);

        if (selectedRegions.some((s) => s.trackId === track.id && s.regionId === region.id)) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2;
          ctx.strokeRect(rx + 1, y + 1, rrw - 2, rh - 2);
        }

        if (region.notes.length > 0) {
          let lo = 127, hi = 0;
          for (const note of region.notes) {
            if (note.pitch < lo) lo = note.pitch;
            if (note.pitch > hi) hi = note.pitch;
          }
          const span = Math.max(hi - lo, 1);
          const pad = 3;
          const innerH = rh - pad * 2;
          const noteH = Math.max(1, Math.min(3, Math.floor(innerH / (span + 1))));
          ctx.fillStyle = color + "bb";
          for (const note of region.notes) {
            const nx = rx + note.tick / ticksPerPixel;
            const nw = Math.max(1, note.durationTicks / ticksPerPixel);
            const pct = span > 0 ? (note.pitch - lo) / span : 0.5;
            const noteY = y + pad + innerH - pct * (innerH - noteH);
            ctx.fillRect(nx, noteY, nw, noteH);
          }
        }
      }
    }

    if (d) {
      if (d.mode === "select") {
        const x1 = Math.min(d.startX, d.currentX);
        const x2 = Math.max(d.startX, d.currentX);
        const sy1 = Math.min(d.startY, d.currentTrackIdx * trackHeight);
        const sy2 = Math.max(d.startY, (d.currentTrackIdx + 1) * trackHeight);
        ctx.fillStyle = "rgba(74, 158, 255, 0.1)";
        ctx.strokeStyle = "rgba(74, 158, 255, 0.6)";
        ctx.lineWidth = 1;
        ctx.setLineDash([3, 3]);
        ctx.fillRect(x1, sy1, x2 - x1, sy2 - sy1);
        ctx.strokeRect(x1, sy1, x2 - x1, sy2 - sy1);
        ctx.setLineDash([]);
      }

      if (d.mode === "create" && d.trackIdx < tracks.length) {
        const rawStart = pixelToTick(d.startX);
        const rawEnd = pixelToTick(d.currentX);
        const clamped = clampCreateRegion(d.trackIdx, Math.min(rawStart, rawEnd), Math.max(rawStart, rawEnd));
        if (clamped) {
          const track = tracks[d.trackIdx];
          const color = CHANNEL_COLORS[trackChannelType(track)] || "#888";
          const px1 = (clamped.start - startTick) / ticksPerPixel;
          const px2 = (clamped.end - startTick) / ticksPerPixel;
          const dy = d.trackIdx * trackHeight + 2;
          const dh = trackHeight - 4;
          ctx.fillStyle = color + "55";
          ctx.strokeStyle = color;
          ctx.lineWidth = 2;
          ctx.setLineDash([4, 4]);
          ctx.fillRect(px1, dy, px2 - px1, dh);
          ctx.strokeRect(px1, dy, px2 - px1, dh);
          ctx.setLineDash([]);
        }
      }

      if (d.mode === "move" && d.regionStartTick != null && d.regionDuration != null) {
        const deltaPx = d.currentX - d.startX;
        const deltaTick = deltaPx * ticksPerPixel;
        const snappedTickDelta = snapToBar(Math.max(0, d.regionStartTick + deltaTick)) - d.regionStartTick;
        const trackDelta = d.currentTrackIdx - d.trackIdx;

        const regionsToPreview = selectedRegions.length > 1 && selectedRegions.some((s) => s.regionId === d.regionId)
          ? selectedRegions
          : [{ trackId: d.regionTrackId!, regionId: d.regionId!, startTick: d.regionStartTick, durationTicks: d.regionDuration }];

        ctx.setLineDash([4, 4]);
        for (const sr of regionsToPreview) {
          const srcIdx = tracks.findIndex((t) => t.id === sr.trackId);
          const tIdx = Math.max(0, Math.min(tracks.length - 1, srcIdx + trackDelta));
          const previewTrack = tIdx < tracks.length ? tracks[tIdx] : null;
          const c = previewTrack ? (CHANNEL_COLORS[trackChannelType(previewTrack)] || "#888") : "#888";
          const region = tracks[srcIdx]?.regions.find((r) => r.id === sr.regionId);
          const dur = region ? region.durationTicks : sr.durationTicks;
          const newStart = Math.max(0, sr.startTick + snappedTickDelta);
          const px1 = (newStart - startTick) / ticksPerPixel;
          const pw = dur / ticksPerPixel;
          const dy = tIdx * trackHeight + 2;
          const dh = trackHeight - 4;
          ctx.fillStyle = c + "55";
          ctx.strokeStyle = c;
          ctx.lineWidth = 2;
          ctx.fillRect(px1, dy, pw, dh);
          ctx.strokeRect(px1, dy, pw, dh);
        }
        ctx.setLineDash([]);
      }

      if ((d.mode === "resize-left" || d.mode === "resize-right") && d.regionStartTick != null && d.regionDuration != null) {
        const origStart = d.regionStartTick;
        const origEnd = origStart + d.regionDuration;
        const deltaPx = d.currentX - d.startX;
        const deltaTick = deltaPx * ticksPerPixel;
        let newStart: number, newEnd: number;
        if (d.mode === "resize-left") {
          newStart = snapToBar(Math.max(0, origStart + deltaTick));
          newEnd = origEnd;
          if (newStart >= newEnd) newStart = newEnd - ticksPerBar;
        } else {
          newStart = origStart;
          newEnd = snapToBar(origEnd + deltaTick);
          if (newEnd <= newStart) newEnd = newStart + ticksPerBar;
        }
        const tIdx = d.trackIdx;
        const track = tIdx < tracks.length ? tracks[tIdx] : null;
        const color = track ? (CHANNEL_COLORS[trackChannelType(track)] || "#888") : "#888";
        const px1 = (newStart - startTick) / ticksPerPixel;
        const pw = (newEnd - newStart) / ticksPerPixel;
        const dy = tIdx * trackHeight + 2;
        const dh = trackHeight - 4;
        ctx.fillStyle = color + "55";
        ctx.strokeStyle = color;
        ctx.lineWidth = 2;
        ctx.setLineDash([4, 4]);
        ctx.fillRect(px1, dy, pw, dh);
        ctx.strokeRect(px1, dy, pw, dh);
        ctx.setLineDash([]);
      }
    }

    if (playing) {
      const cx = (playbackTick - startTick) / ticksPerPixel;
      if (cx >= 0 && cx <= w) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(cx, 0);
        ctx.lineTo(cx, h);
        ctx.stroke();
      }
    }
  }, [tracks, ticksPerPixel, scrollLeft, trackHeight, playbackTick, playing, selectedRegions, ticksPerBar, drag]);

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

  function handleMouseDown(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = hitTestRegion(x, y);
    if (hit) {
      e.preventDefault();
      const track = tracks[hit.trackIdx];
      const region = track.regions.find((r) => r.id === hit.regionId);
      if (!region) return;

      if (hit.edge === "left") {
        setDrag({
          mode: "resize-left", trackIdx: hit.trackIdx, startX: x, startY: y, currentX: x,
          currentTrackIdx: hit.trackIdx,
          regionTrackId: hit.trackId, regionId: hit.regionId,
          regionStartTick: region.startTick, regionDuration: region.durationTicks,
        });
      } else if (hit.edge === "right") {
        setDrag({
          mode: "resize-right", trackIdx: hit.trackIdx, startX: x, startY: y, currentX: x,
          currentTrackIdx: hit.trackIdx,
          regionTrackId: hit.trackId, regionId: hit.regionId,
          regionStartTick: region.startTick, regionDuration: region.durationTicks,
        });
      } else {
        setDrag({
          mode: "move", trackIdx: hit.trackIdx, startX: x, startY: y, currentX: x,
          currentTrackIdx: hit.trackIdx,
          regionTrackId: hit.trackId, regionId: hit.regionId,
          regionStartTick: region.startTick, regionDuration: region.durationTicks,
        });
      }
      return;
    }

    e.preventDefault();
    const trackIdx = Math.max(0, Math.floor(y / trackHeight));
    const now = Date.now();
    const last = lastEmptyClickRef.current;
    if (last && now - last.time < 400 && Math.abs(x - last.x) < 10 && Math.abs(y - last.y) < 10) {
      lastEmptyClickRef.current = null;
      if (trackIdx < tracks.length) {
        setDrag({ mode: "create", trackIdx, startX: x, startY: y, currentX: x, currentTrackIdx: trackIdx });
        return;
      }
    }
    lastEmptyClickRef.current = { x, y, time: now };
    setDrag({ mode: "select", trackIdx, startX: x, startY: y, currentX: x, currentTrackIdx: trackIdx });
  }

  useEffect(() => {
    if (!drag) return;
    const canvas = canvasRef.current;
    if (canvas) {
      if (drag.mode === "move") canvas.style.cursor = "grabbing";
      else if (drag.mode === "resize-left" || drag.mode === "resize-right") canvas.style.cursor = "col-resize";
      else canvas.style.cursor = "default";
    }

    function handleMouseMove(e: MouseEvent) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;
      const newTrackIdx = Math.max(0, Math.min(tracks.length - 1, Math.floor(y / trackHeight)));
      setDrag((prev) => prev ? { ...prev, currentX: x, currentTrackIdx: newTrackIdx } : null);
    }

    function handleMouseUp(e: MouseEvent) {
      const d = dragRef.current;
      if (!d) { setDrag(null); return; }
      const canvas = canvasRef.current;
      if (!canvas) { setDrag(null); return; }
      const rect = canvas.getBoundingClientRect();
      const endX = e.clientX - rect.left;
      const endY = e.clientY - rect.top;

      const movedEnough = Math.abs(endX - d.startX) > 4 || Math.abs(endY - d.startY) > 4;

      if (d.mode === "select" && movedEnough) {
        const tickLeft = pixelToTick(Math.min(d.startX, endX));
        const tickRight = pixelToTick(Math.max(d.startX, endX));
        const minTrack = Math.min(d.trackIdx, Math.max(0, Math.floor(endY / trackHeight)));
        const maxTrack = Math.max(d.trackIdx, Math.max(0, Math.floor(endY / trackHeight)));
        const hits: SelectedRegion[] = [];
        for (let i = minTrack; i <= maxTrack && i < tracks.length; i++) {
          const track = tracks[i];
          for (const region of track.regions) {
            if (region.startTick + region.durationTicks > tickLeft && region.startTick < tickRight) {
              hits.push({
                trackId: track.id, trackName: track.name, regionId: region.id,
                channelType: trackChannelType(track) as "fm" | "psg" | "dac",
                startTick: region.startTick, durationTicks: region.durationTicks,
              });
            }
          }
        }
        if (hits.length > 0) onSelectRegions(hits);
      }

      if (d.mode === "create") {
        const rawStart = pixelToTick(d.startX);
        const rawEnd = pixelToTick(endX);
        const clamped = clampCreateRegion(d.trackIdx, Math.min(rawStart, rawEnd), Math.max(rawStart, rawEnd));
        if (clamped && d.trackIdx < tracks.length) {
          onEmptyDoubleClick(tracks[d.trackIdx].id, clamped.start, clamped.end - clamped.start);
        }
      }

      if (d.mode === "move" && movedEnough && d.regionTrackId && d.regionId && d.regionStartTick != null) {
        const deltaPx = endX - d.startX;
        const deltaTick = deltaPx * ticksPerPixel;
        const snappedTickDelta = snapToBar(Math.max(0, d.regionStartTick + deltaTick)) - d.regionStartTick;
        const newStart = Math.max(0, d.regionStartTick + snappedTickDelta);
        const endTrackIdx = Math.max(0, Math.min(tracks.length - 1, Math.floor(endY / trackHeight)));
        const trackDelta = endTrackIdx - d.trackIdx;
        const dstTrackId = tracks[endTrackIdx].id;
        onRegionMove(d.regionTrackId, d.regionId, dstTrackId, newStart, snappedTickDelta, trackDelta);
      }

      if ((d.mode === "resize-left" || d.mode === "resize-right") && movedEnough && d.regionTrackId && d.regionId && d.regionStartTick != null && d.regionDuration != null) {
        const origStart = d.regionStartTick;
        const origEnd = origStart + d.regionDuration;
        const deltaPx = endX - d.startX;
        const deltaTick = deltaPx * ticksPerPixel;
        let newStart: number, newEnd: number;
        if (d.mode === "resize-left") {
          newStart = snapToBar(Math.max(0, origStart + deltaTick));
          newEnd = origEnd;
          if (newStart >= newEnd) newStart = newEnd - ticksPerBar;
        } else {
          newStart = origStart;
          newEnd = snapToBar(origEnd + deltaTick);
          if (newEnd <= newStart) newEnd = newStart + ticksPerBar;
        }
        onRegionResize(d.regionTrackId, d.regionId, newStart, newEnd - newStart);
      }

      setDrag(null);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [drag, ticksPerPixel, scrollLeft, ticksPerBar, tracks, trackHeight, onSelectRegions, onEmptyDoubleClick, onRegionMove, onRegionResize]);

  function handleClick(e: React.MouseEvent) {
    if (drag) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = hitTestRegion(x, y);
    if (hit) {
      onRegionClick(hit.trackId, hit.regionId, e.ctrlKey || e.metaKey);
    }
  }

  function handleDoubleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = hitTestRegion(x, y);
    if (hit) {
      onRegionDoubleClick(hit.trackId, hit.regionId);
    }
  }

  function handleMouseMoveCanvas(e: React.MouseEvent) {
    if (drag) return;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = hitTestRegion(x, y);
    if (hit?.edge) {
      canvas.style.cursor = "col-resize";
    } else if (hit) {
      canvas.style.cursor = "grab";
    } else {
      canvas.style.cursor = "default";
    }
  }

  return (
    <div ref={containerRef} className={styles.container}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMoveCanvas}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
      />
    </div>
  );
}
