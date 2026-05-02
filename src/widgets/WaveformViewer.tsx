import { useRef, useEffect } from "react";

interface WaveformViewerProps {
  data: number[];
  width?: number;
  height?: number;
}

export function WaveformViewer({ data, width = 500, height = 120 }: WaveformViewerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

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

    if (data.length === 0) return;

    const style = getComputedStyle(canvas);
    const dacColor = style.getPropertyValue("--accent-dac").trim() || "#ff8844";

    const centerY = height / 2;
    ctx.beginPath();
    ctx.moveTo(0, centerY);
    ctx.lineTo(width, centerY);
    ctx.strokeStyle = "var(--border)";
    ctx.lineWidth = 1;
    ctx.stroke();

    const step = Math.max(1, Math.floor(data.length / width));
    ctx.beginPath();
    for (let x = 0; x < width; x++) {
      const sampleIndex = Math.floor((x / width) * data.length);
      let minVal = 255;
      let maxVal = 0;
      for (let j = sampleIndex; j < Math.min(sampleIndex + step, data.length); j++) {
        minVal = Math.min(minVal, data[j]);
        maxVal = Math.max(maxVal, data[j]);
      }
      const yMin = ((255 - maxVal) / 255) * height;
      const yMax = ((255 - minVal) / 255) * height;
      if (x === 0) {
        ctx.moveTo(x, (yMin + yMax) / 2);
      }
      ctx.lineTo(x, yMin);
      ctx.lineTo(x, yMax);
    }
    ctx.strokeStyle = dacColor;
    ctx.lineWidth = 1;
    ctx.stroke();
  }, [data, width, height]);

  return (
    <canvas
      ref={canvasRef}
      style={{ width, height, display: "block", border: "1px solid var(--border)", borderRadius: 4, background: "var(--bg-app)" }}
    />
  );
}
