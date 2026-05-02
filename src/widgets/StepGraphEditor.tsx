import { useRef, useEffect, useState, useCallback } from "react";
import styles from "./StepGraphEditor.module.css";

interface StepGraphEditorProps {
  values: number[];
  max: number;
  onChange: (values: number[]) => void;
  loopPoint: number | null;
  onLoopChange: (point: number | null) => void;
}

export function StepGraphEditor({ values, max, onChange, loopPoint, onLoopChange }: StepGraphEditorProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [drawing, setDrawing] = useState(false);
  const [draggingLoop, setDraggingLoop] = useState(false);
  const width = Math.max(200, values.length * 16);
  const height = 120;

  const barWidth = useCallback(() => {
    return Math.max(8, Math.min(20, width / values.length));
  }, [width, values.length]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, width, height);

    const bw = barWidth();
    const gap = 2;
    const style = getComputedStyle(canvas);
    const psgColor = style.getPropertyValue("--accent-psg").trim() || "#44cc66";

    for (let i = 0; i < values.length; i++) {
      const x = i * bw;
      const barH = (values[i] / max) * (height - 20);
      const y = height - 10 - barH;
      ctx.fillStyle = psgColor;
      ctx.globalAlpha = 0.7;
      ctx.fillRect(x + gap / 2, y, bw - gap, barH);
      ctx.globalAlpha = 1;
      ctx.strokeStyle = psgColor;
      ctx.strokeRect(x + gap / 2, y, bw - gap, barH);
    }

    if (loopPoint !== null && loopPoint < values.length) {
      const lx = loopPoint * bw + bw / 2;
      ctx.beginPath();
      ctx.moveTo(lx, 0);
      ctx.lineTo(lx, height);
      ctx.strokeStyle = "#ff6644";
      ctx.lineWidth = 2;
      ctx.setLineDash([4, 2]);
      ctx.stroke();
      ctx.setLineDash([]);
      ctx.fillStyle = "#ff6644";
      ctx.beginPath();
      ctx.moveTo(lx - 5, 0);
      ctx.lineTo(lx + 5, 0);
      ctx.lineTo(lx, 8);
      ctx.fill();
    }
  }, [values, max, width, height, loopPoint, barWidth]);

  function getBarIndex(e: React.MouseEvent): number {
    const rect = canvasRef.current!.getBoundingClientRect();
    const x = e.clientX - rect.left;
    return Math.floor(x / barWidth());
  }

  function getBarValue(e: React.MouseEvent): number {
    const rect = canvasRef.current!.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const normalized = 1 - (y - 10) / (height - 20);
    return Math.max(0, Math.min(max, Math.round(normalized * max)));
  }

  function handleMouseDown(e: React.MouseEvent) {
    const idx = getBarIndex(e);
    if (idx < 0 || idx >= values.length) return;

    if (loopPoint !== null && Math.abs(idx - loopPoint) <= 0) {
      setDraggingLoop(true);
      return;
    }

    setDrawing(true);
    const val = getBarValue(e);
    const newValues = [...values];
    newValues[idx] = val;
    onChange(newValues);
  }

  function handleMouseMove(e: React.MouseEvent) {
    if (draggingLoop) {
      const idx = getBarIndex(e);
      if (idx >= 0 && idx < values.length) onLoopChange(idx);
      return;
    }
    if (!drawing) return;
    const idx = getBarIndex(e);
    if (idx < 0 || idx >= values.length) return;
    const val = getBarValue(e);
    const newValues = [...values];
    newValues[idx] = val;
    onChange(newValues);
  }

  function handleMouseUp() {
    setDrawing(false);
    setDraggingLoop(false);
  }

  return (
    <div className={styles.container}>
      <canvas
        ref={canvasRef}
        style={{ width, height }}
        onMouseDown={handleMouseDown}
        onMouseMove={handleMouseMove}
        onMouseUp={handleMouseUp}
        onMouseLeave={handleMouseUp}
        className={styles.canvas}
      />
    </div>
  );
}
