import { useRef, useEffect, useCallback } from "react";
import styles from "./PianoRollKeys.module.css";

interface PianoRollKeysProps {
  minPitch: number;
  maxPitch: number;
  rowHeight: number;
  scrollTop: number;
  onAudition: (pitch: number) => void;
}

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

function isBlackKey(pitch: number): boolean {
  return [1, 3, 6, 8, 10].includes(pitch % 12);
}

export function PianoRollKeys({ minPitch, maxPitch, rowHeight, scrollTop, onAudition }: PianoRollKeysProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNotes = maxPitch - minPitch + 1;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = 48;
    const h = totalNotes * rowHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    for (let i = 0; i < totalNotes; i++) {
      const pitch = maxPitch - i;
      const y = i * rowHeight;
      const black = isBlackKey(pitch);

      ctx.fillStyle = black ? "#1a1a1a" : "#2a2a2a";
      ctx.fillRect(0, y, w, rowHeight);
      ctx.strokeStyle = "#333";
      ctx.beginPath();
      ctx.moveTo(0, y + rowHeight);
      ctx.lineTo(w, y + rowHeight);
      ctx.stroke();

      const octave = Math.floor(pitch / 12) - 1;
      const name = NOTE_NAMES[pitch % 12];
      ctx.fillStyle = black ? "#666" : "#999";
      ctx.font = "9px sans-serif";
      ctx.fillText(`${name}${octave}`, 4, y + rowHeight - 3);
    }
  }, [totalNotes, rowHeight, maxPitch]);

  useEffect(() => { draw(); }, [draw]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const idx = Math.floor(y / rowHeight);
    const pitch = maxPitch - idx;
    if (pitch >= minPitch && pitch <= maxPitch) {
      onAudition(pitch);
    }
  }

  return (
    <div ref={containerRef} className={styles.keys} style={{ marginTop: -scrollTop }}>
      <canvas ref={canvasRef} onClick={handleClick} />
    </div>
  );
}
