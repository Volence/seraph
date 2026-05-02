import { useState, useCallback } from "react";

interface ZoomState {
  ticksPerPixel: number;
  scrollLeft: number;
  setScrollLeft: (v: number) => void;
  handleWheel: (e: React.WheelEvent) => void;
  tickToPixel: (tick: number) => number;
  pixelToTick: (px: number) => number;
}

export function useArrangementZoom(ticksPerBeat: number): ZoomState {
  const ticksPerBar = ticksPerBeat * 4;
  const defaultTicksPerPixel = (ticksPerBar * 16) / 1200;
  const [ticksPerPixel, setTicksPerPixel] = useState(defaultTicksPerPixel);
  const [scrollLeft, setScrollLeft] = useState(0);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const zoomFactor = e.deltaY > 0 ? 1.15 : 0.87;
        setTicksPerPixel((prev) => {
          const next = prev * zoomFactor;
          return Math.max(0.05, Math.min(next, ticksPerBar));
        });
      } else if (e.shiftKey) {
        setScrollLeft((prev) => Math.max(0, prev + e.deltaY));
      } else {
        setScrollLeft((prev) => Math.max(0, prev + e.deltaX));
      }
    },
    [ticksPerBar],
  );

  const tickToPixel = useCallback(
    (tick: number) => tick / ticksPerPixel - scrollLeft,
    [ticksPerPixel, scrollLeft],
  );

  const pixelToTick = useCallback(
    (px: number) => (px + scrollLeft) * ticksPerPixel,
    [ticksPerPixel, scrollLeft],
  );

  return { ticksPerPixel, scrollLeft, setScrollLeft, handleWheel, tickToPixel, pixelToTick };
}
