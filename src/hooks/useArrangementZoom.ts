import { useState, useCallback, useEffect, useRef } from "react";
import { ticksPerBar as ticksPerBarOf, type GridMeta } from "../utils/grid";
import { zoomAroundPixel } from "../utils/zoomDrag";

const MIN_TICKS_PER_PIXEL = 0.05;

interface ZoomState {
  ticksPerPixel: number;
  scrollLeft: number;
  setScrollLeft: (v: number) => void;
  handleWheel: (e: React.WheelEvent) => void;
  bodyRef: React.RefObject<HTMLDivElement | null>;
  tickToPixel: (tick: number) => number;
  pixelToTick: (px: number) => number;
  /** Rescale by `factor` keeping the tick at view-x `anchorPx` fixed (ruler vertical-drag zoom). */
  zoomAtBy: (anchorPx: number, factor: number) => void;
}

export function useArrangementZoom(meta: GridMeta): ZoomState {
  const ticksPerBar = ticksPerBarOf(meta);
  const defaultTicksPerPixel = (ticksPerBar * 16) / 1200;
  const [ticksPerPixel, setTicksPerPixel] = useState(defaultTicksPerPixel);
  const [scrollLeft, setScrollLeft] = useState(0);
  const bodyRef = useRef<HTMLDivElement | null>(null);
  const ticksPerBarRef = useRef(ticksPerBar);
  ticksPerBarRef.current = ticksPerBar;
  const scrollLeftRef = useRef(scrollLeft);
  scrollLeftRef.current = scrollLeft;

  // Non-passive wheel listener on the scrollable body to block native scroll during Ctrl+zoom
  useEffect(() => {
    const el = bodyRef.current;
    if (!el) return;
    const handler = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
      }
    };
    el.addEventListener("wheel", handler, { passive: false });
    return () => el.removeEventListener("wheel", handler);
  }, []);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const zoomFactor = e.deltaY > 0 ? 1.15 : 0.87;
        setTicksPerPixel((prev) => {
          const next = prev * zoomFactor;
          return Math.max(MIN_TICKS_PER_PIXEL, Math.min(next, ticksPerBar));
        });
      } else if (e.shiftKey) {
        setScrollLeft((prev) => Math.max(0, prev + e.deltaY));
      } else {
        setScrollLeft((prev) => Math.max(0, prev + (e.deltaX || e.deltaY)));
      }
    },
    [ticksPerBar],
  );

  // Ruler vertical-drag zoom: clamp like the wheel path, then adjust
  // scrollLeft so the grabbed tick stays under the pointer. The inner set
  // uses a PLAIN value (via scrollLeftRef, fresh each render) — StrictMode
  // double-invokes updaters, and a plain value stays idempotent where a
  // functional update would double-apply the scroll shift.
  const zoomAtBy = useCallback((anchorPx: number, factor: number) => {
    setTicksPerPixel((prevTpp) => {
      const next = Math.max(MIN_TICKS_PER_PIXEL, Math.min(prevTpp * factor, ticksPerBarRef.current));
      const view = zoomAroundPixel(
        { ticksPerPixel: prevTpp, scrollLeft: scrollLeftRef.current },
        anchorPx,
        next,
      );
      setScrollLeft(view.scrollLeft);
      return next;
    });
  }, []);

  const tickToPixel = useCallback(
    (tick: number) => tick / ticksPerPixel - scrollLeft,
    [ticksPerPixel, scrollLeft],
  );

  const pixelToTick = useCallback(
    (px: number) => (px + scrollLeft) * ticksPerPixel,
    [ticksPerPixel, scrollLeft],
  );

  return { ticksPerPixel, scrollLeft, setScrollLeft, handleWheel, bodyRef, tickToPixel, pixelToTick, zoomAtBy };
}
