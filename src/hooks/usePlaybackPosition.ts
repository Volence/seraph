import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";

interface PlaybackPosition {
  currentTick: number;
  interpolatedTick: number;
}

export function usePlaybackPosition(
  playing: boolean,
  tempoBpm: number,
  ticksPerBeat: number,
): PlaybackPosition {
  const [currentTick, setCurrentTick] = useState(0);
  const lastEventRef = useRef<{ tick: number; time: number }>({ tick: 0, time: 0 });
  const interpolatedRef = useRef(0);
  const animFrameRef = useRef(0);

  useEffect(() => {
    const unlisten = listen<number>("playback-position", (event) => {
      const tick = event.payload;
      setCurrentTick(tick);
      lastEventRef.current = { tick, time: performance.now() };
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const ticksPerMs = (tempoBpm / 60000) * ticksPerBeat;

  const animate = useCallback(() => {
    if (playing) {
      const elapsed = performance.now() - lastEventRef.current.time;
      interpolatedRef.current = lastEventRef.current.tick + elapsed * ticksPerMs;
    }
    animFrameRef.current = requestAnimationFrame(animate);
  }, [playing, ticksPerMs]);

  useEffect(() => {
    animFrameRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [animate]);

  return {
    currentTick,
    interpolatedTick: playing ? interpolatedRef.current : currentTick,
  };
}
