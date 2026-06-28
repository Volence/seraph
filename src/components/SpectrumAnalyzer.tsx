import { useRef, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import styles from "./SpectrumAnalyzer.module.css";

interface SpectrumAnalyzerProps {
  height?: number;
}

const NUM_BINS = 128;
const DB_MIN = -90;
const DB_MAX = 0;
const DB_RANGE = DB_MAX - DB_MIN;

const FREQ_LABELS = [
  { freq: 30, label: "30" },
  { freq: 50, label: "50" },
  { freq: 100, label: "100" },
  { freq: 200, label: "200" },
  { freq: 500, label: "500" },
  { freq: 1000, label: "1k" },
  { freq: 2000, label: "2k" },
  { freq: 5000, label: "5k" },
  { freq: 10000, label: "10k" },
  { freq: 20000, label: "20k" },
];

const DB_GRID = [-12, -24, -36, -48, -60, -72, -84];

const FREQ_MIN = 20;
const FREQ_MAX = 22050;
const LOG_MIN = Math.log(FREQ_MIN);
const LOG_MAX = Math.log(FREQ_MAX);
const LOG_RANGE = LOG_MAX - LOG_MIN;

const LEFT_MARGIN_CSS = 32;

export function SpectrumAnalyzer({ height = 100 }: SpectrumAnalyzerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const rawBins = useRef(new Float32Array(NUM_BINS).fill(DB_MIN));
  const smoothedBins = useRef(new Float32Array(NUM_BINS).fill(DB_MIN));
  const peakBins = useRef(new Float32Array(NUM_BINS).fill(DB_MIN));
  const animFrameRef = useRef(0);
  const [containerWidth, setContainerWidth] = useState(800);

  useEffect(() => {
    const unlisten = listen<number[]>("spectrum-data", (event) => {
      const data = event.payload;
      for (let i = 0; i < NUM_BINS && i < data.length; i++) {
        rawBins.current[i] = data[i];
      }
    });
    return () => { unlisten.then((fn) => fn()); };
  }, []);

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;
    const obs = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setContainerWidth(entry.contentRect.width);
      }
    });
    obs.observe(container);
    setContainerWidth(container.clientWidth);
    return () => obs.disconnect();
  }, []);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const dpr = window.devicePixelRatio || 1;
    const W = Math.floor(containerWidth * dpr);
    const H = Math.floor(height * dpr);
    canvas.width = W;
    canvas.height = H;

    const lm = LEFT_MARGIN_CSS * dpr;
    const plotW = W - lm;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const gradient = ctx.createLinearGradient(0, 0, 0, H);
    gradient.addColorStop(0, "rgba(180, 120, 255, 0.9)");
    gradient.addColorStop(0.3, "rgba(140, 70, 220, 0.6)");
    gradient.addColorStop(0.6, "rgba(80, 30, 160, 0.3)");
    gradient.addColorStop(1, "rgba(40, 15, 100, 0.05)");

    function dbToY(db: number): number {
      const clamped = Math.max(DB_MIN, Math.min(DB_MAX, db));
      return H * (1 - (clamped - DB_MIN) / DB_RANGE);
    }

    function freqToCanvasX(freq: number): number {
      return lm + ((Math.log(freq) - LOG_MIN) / LOG_RANGE) * plotW;
    }

    function binToX(i: number): number {
      return lm + (i / (NUM_BINS - 1)) * plotW;
    }

    function drawFrame() {
      if (!ctx) return;

      const smoothed = smoothedBins.current;
      const peaks = peakBins.current;
      const raw = rawBins.current;

      for (let i = 0; i < NUM_BINS; i++) {
        const target = raw[i];
        const current = smoothed[i];
        const alpha = target > current ? 0.45 : 0.1;
        smoothed[i] = current + alpha * (target - current);
      }

      for (let i = 0; i < NUM_BINS; i++) {
        if (smoothed[i] > peaks[i]) {
          peaks[i] = smoothed[i];
        } else {
          peaks[i] -= 0.35;
          if (peaks[i] < DB_MIN) peaks[i] = DB_MIN;
        }
      }

      ctx.clearRect(0, 0, W, H);

      // Plot area background
      ctx.fillStyle = "rgba(0, 0, 0, 0.15)";
      ctx.fillRect(lm, 0, plotW, H);

      // Frequency grid lines + labels
      ctx.lineWidth = dpr;
      for (const { freq, label } of FREQ_LABELS) {
        const x = freqToCanvasX(freq);
        ctx.strokeStyle = "rgba(255, 255, 255, 0.07)";
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, H);
        ctx.stroke();
        ctx.font = `${10 * dpr}px monospace`;
        ctx.fillStyle = "rgba(255, 255, 255, 0.35)";
        ctx.textAlign = "center";
        ctx.fillText(label, x, H - 3 * dpr);
      }

      // dB grid lines + labels
      for (const db of DB_GRID) {
        const y = dbToY(db);
        ctx.strokeStyle = "rgba(255, 255, 255, 0.05)";
        ctx.beginPath();
        ctx.moveTo(lm, y);
        ctx.lineTo(W, y);
        ctx.stroke();
        ctx.font = `${9 * dpr}px monospace`;
        ctx.fillStyle = "rgba(255, 255, 255, 0.3)";
        ctx.textAlign = "right";
        ctx.fillText(`${db}`, lm - 4 * dpr, y + 3 * dpr);
      }
      // 0 dB label
      ctx.font = `${9 * dpr}px monospace`;
      ctx.fillStyle = "rgba(255, 255, 255, 0.3)";
      ctx.textAlign = "right";
      ctx.fillText("0", lm - 4 * dpr, dbToY(0) + 10 * dpr);

      // Clip to plot area for spectrum drawing
      ctx.save();
      ctx.beginPath();
      ctx.rect(lm, 0, plotW, H);
      ctx.clip();

      // Filled spectrum curve
      ctx.beginPath();
      ctx.moveTo(binToX(0), H);
      for (let i = 0; i < NUM_BINS; i++) {
        ctx.lineTo(binToX(i), dbToY(smoothed[i]));
      }
      ctx.lineTo(binToX(NUM_BINS - 1), H);
      ctx.closePath();
      ctx.fillStyle = gradient;
      ctx.fill();

      // Spectrum line (white)
      ctx.beginPath();
      for (let i = 0; i < NUM_BINS; i++) {
        const x = binToX(i);
        const y = dbToY(smoothed[i]);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = "rgba(255, 255, 255, 0.9)";
      ctx.lineWidth = 1.5 * dpr;
      ctx.stroke();

      // Peak-hold line (cyan)
      ctx.beginPath();
      for (let i = 0; i < NUM_BINS; i++) {
        const x = binToX(i);
        const y = dbToY(peaks[i]);
        if (i === 0) ctx.moveTo(x, y);
        else ctx.lineTo(x, y);
      }
      ctx.strokeStyle = "rgba(100, 200, 255, 0.4)";
      ctx.lineWidth = 1 * dpr;
      ctx.stroke();

      ctx.restore();

      animFrameRef.current = requestAnimationFrame(drawFrame);
    }

    animFrameRef.current = requestAnimationFrame(drawFrame);
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [containerWidth, height]);

  return (
    <div ref={containerRef} className={styles.container} style={{ height: `${height}px` }}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        style={{ width: `${containerWidth}px`, height: `${height}px` }}
      />
    </div>
  );
}
