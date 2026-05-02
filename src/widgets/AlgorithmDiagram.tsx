import { useRef, useEffect } from "react";
import { CARRIER_MASKS } from "../types/model";
import styles from "./AlgorithmDiagram.module.css";

interface AlgorithmDiagramProps {
  algorithm: number;
  onSelect: (algo: number) => void;
}

interface AlgoLayout {
  ops: { x: number; y: number }[];
  connections: [number, number][];
}

const W = 86;
const H = 44;
const BOX_W = 14;
const BOX_H = 12;

const LAYOUTS: AlgoLayout[] = [
  { ops: [{x:4,y:16},{x:24,y:16},{x:44,y:16},{x:64,y:16}], connections: [[0,1],[1,2],[2,3]] },
  { ops: [{x:4,y:6},{x:4,y:28},{x:34,y:16},{x:64,y:16}], connections: [[0,2],[1,2],[2,3]] },
  { ops: [{x:4,y:6},{x:4,y:28},{x:34,y:28},{x:64,y:16}], connections: [[0,3],[1,2],[2,3]] },
  { ops: [{x:4,y:6},{x:34,y:6},{x:4,y:28},{x:64,y:16}], connections: [[0,1],[1,3],[2,3]] },
  { ops: [{x:4,y:6},{x:34,y:6},{x:4,y:28},{x:34,y:28}], connections: [[0,1],[2,3]] },
  { ops: [{x:4,y:16},{x:34,y:2},{x:34,y:16},{x:34,y:30}], connections: [[0,1],[0,2],[0,3]] },
  { ops: [{x:4,y:16},{x:34,y:16},{x:56,y:6},{x:56,y:28}], connections: [[0,1]] },
  { ops: [{x:4,y:16},{x:26,y:16},{x:48,y:16},{x:70,y:16}], connections: [] },
];

function drawAlgo(canvas: HTMLCanvasElement, algoIndex: number, isActive: boolean) {
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const dpr = window.devicePixelRatio || 1;
  canvas.width = W * dpr;
  canvas.height = H * dpr;
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, W, H);

  const layout = LAYOUTS[algoIndex];
  const carriers = CARRIER_MASKS[algoIndex];
  const style = getComputedStyle(canvas);
  const fmColor = style.getPropertyValue("--accent-fm").trim() || "#4a9eff";
  const carrierColor = style.getPropertyValue("--carrier-highlight").trim() || "#ffcc44";
  const textColor = style.getPropertyValue("--text-primary").trim() || "#e0e0e0";
  const dimColor = style.getPropertyValue("--text-secondary").trim() || "#888";

  for (const [from, to] of layout.connections) {
    const fx = layout.ops[from].x + BOX_W / 2;
    const fy = layout.ops[from].y + BOX_H / 2;
    const tx = layout.ops[to].x + BOX_W / 2;
    const ty = layout.ops[to].y + BOX_H / 2;
    ctx.beginPath();
    ctx.moveTo(fx, fy);
    ctx.lineTo(tx, ty);
    ctx.strokeStyle = isActive ? fmColor : dimColor;
    ctx.lineWidth = 1;
    ctx.stroke();
  }

  for (let i = 0; i < 4; i++) {
    const { x, y } = layout.ops[i];
    const isCarrier = (carriers & (1 << i)) !== 0;
    ctx.fillStyle = isCarrier && isActive ? carrierColor : (isActive ? fmColor : dimColor);
    ctx.globalAlpha = isActive ? 0.25 : 0.1;
    ctx.fillRect(x, y, BOX_W, BOX_H);
    ctx.globalAlpha = 1;
    ctx.strokeStyle = isCarrier && isActive ? carrierColor : (isActive ? fmColor : dimColor);
    ctx.lineWidth = 1;
    ctx.strokeRect(x, y, BOX_W, BOX_H);
    ctx.fillStyle = isActive ? textColor : dimColor;
    ctx.font = "9px sans-serif";
    ctx.textAlign = "center";
    ctx.textBaseline = "middle";
    ctx.fillText(String(i + 1), x + BOX_W / 2, y + BOX_H / 2);
  }
}

export function AlgorithmDiagram({ algorithm, onSelect }: AlgorithmDiagramProps) {
  const canvasRefs = useRef<(HTMLCanvasElement | null)[]>([]);

  useEffect(() => {
    for (let i = 0; i < 8; i++) {
      const canvas = canvasRefs.current[i];
      if (canvas) drawAlgo(canvas, i, i === algorithm);
    }
  }, [algorithm]);

  return (
    <div className={styles.grid}>
      {Array.from({ length: 8 }, (_, i) => (
        <div
          key={i}
          className={`${styles.cell} ${i === algorithm ? styles.active : ""}`}
          onClick={() => onSelect(i)}
        >
          <canvas
            ref={(el) => { canvasRefs.current[i] = el; }}
            style={{ width: W, height: H }}
          />
          <span className={styles.label}>{i}</span>
        </div>
      ))}
    </div>
  );
}
