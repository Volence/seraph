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
  selectedRegion: SelectedRegion | null;
  onRegionClick: (trackId: string, regionId: string) => void;
  onRegionDoubleClick: (trackId: string, regionId: string) => void;
  onRegionCreate: (trackId: string, startTick: number, durationTicks: number) => void;
}

const CHANNEL_COLORS: Record<string, string> = {
  fm: "#4a9eff",
  psg: "#44cc66",
  dac: "#ff8844",
};

function trackChannelType(track: Track): string {
  const ch = track.channel;
  if (ch === "PsgNoise") return "psg";
  if (typeof ch === "object" && "Fm" in ch) return "fm";
  if (typeof ch === "object" && "Psg" in ch) return "psg";
  return "dac";
}

interface DragState {
  trackIdx: number;
  startX: number;
  currentX: number;
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
  selectedRegion,
  onRegionClick,
  onRegionDoubleClick,
  onRegionCreate,
}: TimelineCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const animRef = useRef(0);
  const [drag, setDrag] = useState<DragState | null>(null);
  const dragRef = useRef<DragState | null>(null);
  dragRef.current = drag;

  const ticksPerBar = ticksPerBeat * beatsPerBar;

  function pixelToTick(px: number): number {
    return px * ticksPerPixel + scrollLeft * ticksPerPixel;
  }

  function snapToBar(tick: number): number {
    return Math.round(tick / ticksPerBar) * ticksPerBar;
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

    for (let i = 0; i < tracks.length; i++) {
      const track = tracks[i];
      const color = CHANNEL_COLORS[trackChannelType(track)] || "#888";
      const y = i * trackHeight + 2;
      const rh = trackHeight - 4;

      for (const region of track.regions) {
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

        if (selectedRegion?.trackId === track.id && selectedRegion?.regionId === region.id) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2;
          ctx.strokeRect(rx + 1, y + 1, rrw - 2, rh - 2);
        }

        if (ticksPerPixel < 4 && region.notes.length > 0) {
          ctx.fillStyle = color + "88";
          for (const note of region.notes) {
            const nx = rx + note.tick / ticksPerPixel;
            const nw = Math.max(1, note.durationTicks / ticksPerPixel);
            const noteY = y + rh - ((note.pitch - 24) / 96) * rh;
            ctx.fillRect(nx, noteY, nw, 2);
          }
        }
      }
    }

    // Draw drag preview
    const d = dragRef.current;
    if (d && d.trackIdx < tracks.length) {
      const track = tracks[d.trackIdx];
      const color = CHANNEL_COLORS[trackChannelType(track)] || "#888";
      const rawStart = pixelToTick(Math.min(d.startX, d.currentX));
      const rawEnd = pixelToTick(Math.max(d.startX, d.currentX));
      const snapStart = snapToBar(rawStart);
      const snapEnd = Math.max(snapStart + ticksPerBar, snapToBar(rawEnd));
      const px1 = (snapStart - startTick) / ticksPerPixel;
      const px2 = (snapEnd - startTick) / ticksPerPixel;
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
  }, [tracks, ticksPerPixel, scrollLeft, trackHeight, playbackTick, playing, selectedRegion, ticksPerBar, drag]);

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

  function hitTestRegion(x: number, y: number): { trackId: string; regionId: string } | null {
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return null;
    const startTick = scrollLeft * ticksPerPixel;
    const track = tracks[trackIdx];
    for (const region of track.regions) {
      const rx = (region.startTick - startTick) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        return { trackId: track.id, regionId: region.id };
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

    const hit = hitTestRegion(x, y);
    if (hit) return;

    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return;

    e.preventDefault();
    setDrag({ trackIdx, startX: x, currentX: x });
  }

  useEffect(() => {
    if (!drag) return;

    function handleMouseMove(e: MouseEvent) {
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      setDrag((prev) => prev ? { ...prev, currentX: x } : null);
    }

    function handleMouseUp(e: MouseEvent) {
      const d = dragRef.current;
      if (!d) return;
      const canvas = canvasRef.current;
      if (!canvas) return;
      const rect = canvas.getBoundingClientRect();
      const endX = e.clientX - rect.left;

      const rawStart = pixelToTick(Math.min(d.startX, endX));
      const rawEnd = pixelToTick(Math.max(d.startX, endX));
      const snapStart = snapToBar(rawStart);
      const snapEnd = Math.max(snapStart + ticksPerBar, snapToBar(rawEnd));

      if (d.trackIdx < tracks.length) {
        onRegionCreate(tracks[d.trackIdx].id, snapStart, snapEnd - snapStart);
      }
      setDrag(null);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [drag, ticksPerPixel, scrollLeft, ticksPerBar, tracks, onRegionCreate]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const hit = hitTestRegion(x, y);
    if (hit) {
      onRegionClick(hit.trackId, hit.regionId);
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

  return (
    <div ref={containerRef} className={styles.container}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onMouseDown={handleMouseDown}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
      />
    </div>
  );
}
