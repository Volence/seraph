import { useRef, useEffect } from "react";

interface EnvelopeDisplayProps {
  attackRate: number;
  d1r: number;
  d2r: number;
  sustainLevel: number;
  releaseRate: number;
  width?: number;
  height?: number;
}

export function EnvelopeDisplay({
  attackRate, d1r, d2r, sustainLevel, releaseRate,
  width = 180, height = 70,
}: EnvelopeDisplayProps) {
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

    const pad = 4;
    const w = width - pad * 2;
    const h = height - pad * 2;

    const aFrac = attackRate > 0 ? Math.pow((31 - attackRate) / 31, 1.5) : 1;
    const d1Frac = d1r > 0 ? Math.pow((31 - d1r) / 31, 1.5) : 0;
    const d2Frac = d2r > 0 ? Math.pow((31 - d2r) / 31, 1.5) * 0.3 : 0;
    const rFrac = releaseRate > 0 ? Math.pow((15 - releaseRate) / 15, 1.5) : 1;

    const slNorm = 1 - sustainLevel / 15;
    const slY = pad + h * (1 - slNorm);

    const aW = Math.max(2, aFrac * w * 0.2);
    const d1W = Math.max(0, d1Frac * w * 0.2);
    const susW = w * 0.3;
    const d2W = Math.max(0, d2Frac * w * 0.15);
    const rW = Math.max(2, rFrac * w * 0.15);

    const style = getComputedStyle(canvas);
    const lineColor = style.getPropertyValue("--envelope-line").trim() || "#66ccff";
    const fillColor = style.getPropertyValue("--envelope-fill").trim() || "rgba(102,204,255,0.15)";

    ctx.beginPath();
    let x = pad;
    ctx.moveTo(x, pad + h);

    x += aW;
    ctx.lineTo(x, pad);

    const d2Decay = d2r > 0 ? slNorm * 0.15 * h : 0;

    x += d1W;
    ctx.lineTo(x, slY);

    x += susW;
    ctx.lineTo(x, slY + d2Decay);

    x += d2W;
    ctx.lineTo(x, slY + d2Decay);

    x += rW;
    ctx.lineTo(x, pad + h);

    ctx.strokeStyle = lineColor;
    ctx.lineWidth = 1.5;
    ctx.stroke();

    ctx.lineTo(x, pad + h);
    ctx.lineTo(pad, pad + h);
    ctx.closePath();
    ctx.fillStyle = fillColor;
    ctx.fill();
  }, [attackRate, d1r, d2r, sustainLevel, releaseRate, width, height]);

  return <canvas ref={canvasRef} style={{ width, height, display: "block" }} />;
}
