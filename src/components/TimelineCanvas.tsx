import { useRef, useEffect, useCallback } from "react";
import type { Track, SelectedRegion } from "../types/model";
import styles from "./TimelineCanvas.module.css";

interface TimelineCanvasProps {
  tracks: Track[];
  ticksPerPixel: number;
  scrollLeft: number;
  trackHeight: number;
  playbackTick: number;
  playing: boolean;
  selectedRegion: SelectedRegion | null;
  onRegionClick: (trackId: string, regionId: string) => void;
  onRegionDoubleClick: (trackId: string, regionId: string) => void;
  onEmptyDoubleClick: (trackId: string, startTick: number) => void;
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

export function TimelineCanvas({
  tracks,
  ticksPerPixel,
  scrollLeft,
  trackHeight,
  playbackTick,
  playing,
  selectedRegion,
  onRegionClick,
  onRegionDoubleClick,
  onEmptyDoubleClick,
}: TimelineCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const animRef = useRef(0);

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
  }, [tracks, ticksPerPixel, scrollLeft, trackHeight, playbackTick, playing, selectedRegion]);

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

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return;

    const startTick = scrollLeft * ticksPerPixel;
    const track = tracks[trackIdx];

    for (const region of track.regions) {
      const rx = (region.startTick - startTick) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        onRegionClick(track.id, region.id);
        return;
      }
    }
  }

  function handleDoubleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return;

    const startTick = scrollLeft * ticksPerPixel;
    const tick = x * ticksPerPixel + startTick;
    const track = tracks[trackIdx];

    for (const region of track.regions) {
      const rx = (region.startTick - startTick) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        onRegionDoubleClick(track.id, region.id);
        return;
      }
    }

    onEmptyDoubleClick(track.id, tick);
  }

  return (
    <div ref={containerRef} className={styles.container}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
      />
    </div>
  );
}
