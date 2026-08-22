import { useState, useCallback, useEffect, useRef } from "react";
import type { SelectedInstrument, SelectedRegion, SongMetadata } from "../types/model";
import { FmEditor } from "./FmEditor";
import { PsgEditor } from "./PsgEditor";
import { DacEditor } from "./DacEditor";
import { PianoRoll } from "./PianoRoll";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  selectedInstrument: SelectedInstrument | null;
  selectedRegion: SelectedRegion | null;
  onCloseRegion: () => void;
  playing: boolean;
  projectMeta: SongMetadata;
  /** Seek cursor (absolute ticks); the piano roll's paste anchor. */
  seekTick: number;
  /** Forwarded to the editors' "Save to library" buttons. */
  onSavedToLibrary?: () => void;
}

const MIN_HEIGHT = 120;
const MAX_HEIGHT_RATIO = 0.8;

export function BottomPanel({ selectedInstrument, selectedRegion, onCloseRegion, playing, projectMeta, seekTick, onSavedToLibrary }: BottomPanelProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [height, setHeight] = useState(300);
  const dragging = useRef(false);
  const startY = useRef(0);
  const startHeight = useRef(0);

  const onMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    dragging.current = true;
    startY.current = e.clientY;
    startHeight.current = height;
    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
  }, [height]);

  useEffect(() => {
    function onMouseMove(e: MouseEvent) {
      if (!dragging.current) return;
      const delta = startY.current - e.clientY;
      const maxH = window.innerHeight * MAX_HEIGHT_RATIO;
      setHeight(Math.max(MIN_HEIGHT, Math.min(maxH, startHeight.current + delta)));
    }
    function onMouseUp() {
      if (!dragging.current) return;
      dragging.current = false;
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }
    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    return () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
    };
  }, []);

  const showPianoRoll = selectedRegion !== null;
  const headerText = showPianoRoll ? "Piano Roll" : "Instrument Editor";

  return (
    <div
      className={`${styles.panel} ${collapsed ? styles.collapsed : ""}`}
      style={collapsed ? undefined : { height }}
    >
      {!collapsed && <div className={styles.resizeHandle} onMouseDown={onMouseDown} />}
      <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
        <span>{headerText}</span>
      </div>
      {!collapsed && (
        <div className={styles.editor}>
          {showPianoRoll ? (
            <PianoRoll region={selectedRegion} onClose={onCloseRegion} playing={playing} projectMeta={projectMeta} seekTick={seekTick} />
          ) : (
            <>
              {!selectedInstrument && (
                <div className={styles.empty}>Select an instrument to edit</div>
              )}
              {selectedInstrument?.type === "fm" && (
                <FmEditor instrumentId={selectedInstrument.id} onSavedToLibrary={onSavedToLibrary} />
              )}
              {selectedInstrument?.type === "psg" && (
                <PsgEditor instrumentId={selectedInstrument.id} onSavedToLibrary={onSavedToLibrary} />
              )}
              {selectedInstrument?.type === "dac" && (
                <DacEditor instrumentId={selectedInstrument.id} />
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
