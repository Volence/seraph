import { useRef, useState, useEffect } from "react";
import styles from "./Knob.module.css";

interface KnobProps {
  min: number;
  max: number;
  value: number;
  onChange: (value: number) => void;
  label: string;
  size?: number;
}

export function Knob({ min, max, value, onChange, label, size = 40 }: KnobProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [dragging, setDragging] = useState(false);
  const dragStartY = useRef(0);
  const dragStartValue = useRef(0);
  const [editing, setEditing] = useState(false);
  const [editText, setEditText] = useState("");

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = window.devicePixelRatio || 1;
    canvas.width = size * dpr;
    canvas.height = size * dpr;
    ctx.scale(dpr, dpr);

    const cx = size / 2;
    const cy = size / 2;
    const radius = size / 2 - 4;
    const startAngle = 0.75 * Math.PI;
    const endAngle = 2.25 * Math.PI;
    const range = max - min;
    const normalized = range > 0 ? (value - min) / range : 0;
    const valueAngle = startAngle + normalized * (endAngle - startAngle);

    ctx.clearRect(0, 0, size, size);

    ctx.beginPath();
    ctx.arc(cx, cy, radius, startAngle, endAngle);
    ctx.strokeStyle = getComputedStyle(canvas).getPropertyValue("--knob-track").trim() || "#444";
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.stroke();

    ctx.beginPath();
    ctx.arc(cx, cy, radius, startAngle, valueAngle);
    ctx.strokeStyle = getComputedStyle(canvas).getPropertyValue("--knob-fill").trim() || "#4a9eff";
    ctx.lineWidth = 3;
    ctx.lineCap = "round";
    ctx.stroke();

    const ix = cx + (radius - 4) * Math.cos(valueAngle);
    const iy = cy + (radius - 4) * Math.sin(valueAngle);
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(ix, iy);
    ctx.strokeStyle = "#fff";
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }, [value, min, max, size]);

  function handleMouseDown(e: React.MouseEvent) {
    if (e.detail === 2) {
      setEditing(true);
      setEditText(String(value));
      return;
    }
    setDragging(true);
    dragStartY.current = e.clientY;
    dragStartValue.current = value;
  }

  useEffect(() => {
    if (!dragging) return;

    const range = max - min;
    const sensitivity = Math.max(range / 100, 0.5);

    function handleMouseMove(e: MouseEvent) {
      const dy = dragStartY.current - e.clientY;
      const delta = Math.round(dy * sensitivity);
      const newValue = Math.max(min, Math.min(max, dragStartValue.current + delta));
      onChange(newValue);
    }

    function handleMouseUp() {
      setDragging(false);
    }

    window.addEventListener("mousemove", handleMouseMove);
    window.addEventListener("mouseup", handleMouseUp);
    return () => {
      window.removeEventListener("mousemove", handleMouseMove);
      window.removeEventListener("mouseup", handleMouseUp);
    };
  }, [dragging, min, max, onChange]);

  function handleEditSubmit() {
    const parsed = parseInt(editText, 10);
    if (!isNaN(parsed)) {
      onChange(Math.max(min, Math.min(max, parsed)));
    }
    setEditing(false);
  }

  return (
    <div className={styles.knob}>
      <canvas
        ref={canvasRef}
        style={{ width: size, height: size }}
        onMouseDown={handleMouseDown}
        className={styles.canvas}
      />
      {editing ? (
        <input
          className={styles.editInput}
          style={{ width: size }}
          value={editText}
          onChange={(e) => setEditText(e.target.value)}
          onBlur={handleEditSubmit}
          onKeyDown={(e) => e.key === "Enter" && handleEditSubmit()}
          autoFocus
        />
      ) : (
        <span className={styles.value}>{value}</span>
      )}
      <span className={styles.label}>{label}</span>
    </div>
  );
}
