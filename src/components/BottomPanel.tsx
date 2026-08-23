import { useState, useCallback, useEffect, useRef } from "react";
import type { SelectedInstrument, SelectedRegion, SongMetadata } from "../types/model";
import { FmEditor } from "./FmEditor";
import { PsgEditor } from "./PsgEditor";
import { DacEditor } from "./DacEditor";
import { PianoRoll } from "./PianoRoll";
import { VIEW_STATE_WRITE_DELAY_MS, clampNumber, getViewState, patchViewState } from "../utils/viewState";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  /** Open project's directory; keys the remembered view state (F15). */
  projectPath: string | null;
  selectedInstrument: SelectedInstrument | null;
  selectedRegion: SelectedRegion | null;
  onCloseRegion: () => void;
  playing: boolean;
  projectMeta: SongMetadata;
  /** Seek cursor (absolute ticks); the piano roll's paste anchor. */
  seekTick: number;
  /** Seek request from the piano-roll ruler (absolute ticks); App owns the cursor. */
  onSeek: (tick: number) => void;
  /** Preview loop armed: the piano roll suppresses follow-playhead (owner
   *  ruling — looping playback must not move the view). */
  loopEnabled?: boolean;
  /** Forwarded to the editors' "Save to library" buttons. */
  onSavedToLibrary?: () => void;
}

const MIN_HEIGHT = 120;
const MAX_HEIGHT_RATIO = 0.8;
const DEFAULT_HEIGHT = 300;
const DEFAULT_COLLAPSED = false;

/** Tallest the panel may be right now. One definition, shared by the resize
 *  drag and by the clamp on a restored height. */
function maxPanelHeight(): number {
  return window.innerHeight * MAX_HEIGHT_RATIO;
}

export function BottomPanel({ projectPath, selectedInstrument, selectedRegion, onCloseRegion, playing, projectMeta, seekTick, onSeek, loopEnabled = false, onSavedToLibrary }: BottomPanelProps) {
  // Remembered panel layout (F15), read once at mount; App remounts this
  // component per project. The height is clamped to the SAME bounds the
  // resize drag enforces below, so a stored value from a larger display can
  // never leave the panel taller than the window allows.
  const [savedView] = useState(() => (projectPath ? getViewState(projectPath) : {}));
  const [collapsed, setCollapsed] = useState(() => savedView.panel?.collapsed ?? DEFAULT_COLLAPSED);
  const [height, setHeight] = useState(() =>
    clampNumber(savedView.panel?.height, MIN_HEIGHT, maxPanelHeight(), DEFAULT_HEIGHT),
  );
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
      setHeight(Math.max(MIN_HEIGHT, Math.min(maxPanelHeight(), startHeight.current + delta)));
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

  // Write-through for the panel layout, debounced so one resize drag records
  // once rather than once per mousemove.
  useEffect(() => {
    if (!projectPath) return;
    const timer = setTimeout(() => {
      patchViewState(projectPath, { panel: { collapsed, height } });
    }, VIEW_STATE_WRITE_DELAY_MS);
    return () => clearTimeout(timer);
  }, [projectPath, collapsed, height]);

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
            <PianoRoll region={selectedRegion} projectPath={projectPath} onClose={onCloseRegion} playing={playing} projectMeta={projectMeta} seekTick={seekTick} onSeek={onSeek} loopEnabled={loopEnabled} />
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
