import { useRef, useEffect, useCallback } from "react";
import styles from "./PianoRollKeys.module.css";

/** Key-column widths. The width is owned by PianoRoll so siblings (the
 *  velocity lane) can offset themselves by the same amount (G5). */
export const MELODIC_KEYS_WIDTH = 48;
export const DAC_KEYS_WIDTH = 120;
export const MIN_KEYS_WIDTH = 48;
export const MAX_KEYS_WIDTH = 240;

interface PianoRollKeysProps {
  minPitch: number;
  maxPitch: number;
  rowHeight: number;
  scrollTop: number;
  width: number;
  onWidthChange: (width: number) => void;
  onAudition: (pitch: number) => void;
  drumLabels?: Record<number, string>;
}

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

function isBlackKey(pitch: number): boolean {
  return [1, 3, 6, 8, 10].includes(pitch % 12);
}

export function PianoRollKeys({ minPitch, maxPitch, rowHeight, scrollTop, width, onWidthChange, onAudition, drumLabels }: PianoRollKeysProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNotes = maxPitch - minPitch + 1;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = width;
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

      if (drumLabels) {
        ctx.fillStyle = i % 2 === 0 ? "#2a2a2a" : "#242424";
        ctx.fillRect(0, y, w, rowHeight);
        ctx.strokeStyle = "#333";
        ctx.beginPath();
        ctx.moveTo(0, y + rowHeight);
        ctx.lineTo(w, y + rowHeight);
        ctx.stroke();

        const label = drumLabels[pitch] || `DAC ${pitch - 35}`;
        ctx.fillStyle = "#ccc";
        ctx.font = "11px sans-serif";
        ctx.fillText(label, 4, y + rowHeight - 6);
      } else {
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
    }
  }, [totalNotes, rowHeight, maxPitch, drumLabels, width]);

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

  function handleResizeStart(e: React.MouseEvent) {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = width;
    function onMove(ev: MouseEvent) {
      const newW = Math.max(MIN_KEYS_WIDTH, Math.min(MAX_KEYS_WIDTH, startWidth + ev.clientX - startX));
      onWidthChange(newW);
    }
    function onUp() {
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    }
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }

  return (
    <div ref={containerRef} className={styles.keys} style={{ marginTop: -scrollTop, width }}>
      <canvas ref={canvasRef} onClick={handleClick} />
      {drumLabels && (
        <div className={styles.resizeHandle} onMouseDown={handleResizeStart} />
      )}
    </div>
  );
}
