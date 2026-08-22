import { useState, useCallback } from "react";
import type { SongMetadata } from "../types/model";
import { TransportControls } from "./TransportControls";
import * as ipc from "../api/ipc";
import styles from "./TopBar.module.css";
import { SeraphMark } from "../assets/SeraphMark";

interface TopBarProps {
  projectMeta: SongMetadata | null;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSave: () => void;
  onExport?: () => void;
  onImport?: () => void;
  showSaved: boolean;
  /** Unsaved changes — marks the Save button. */
  dirty?: boolean;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  /** Toggle the preview loop; App owns the range + transport calls. */
  onToggleLoop: () => void;
  /** Seek request (Home button); App owns the cursor + transport call (G29). */
  onSeek: (tick: number) => void;
}

export function TopBar({
  projectMeta,
  onNewProject,
  onOpenProject,
  onSave,
  onExport,
  onImport,
  showSaved,
  dirty,
  playing,
  loopEnabled,
  onPlayingChange,
  onToggleLoop,
  onSeek,
}: TopBarProps) {
  const [masterVol, setMasterVol] = useState(100);
  const [exporting, setExporting] = useState(false);
  const handleMasterVol = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const v = parseInt(e.target.value);
    setMasterVol(v);
    ipc.setMasterVolume(v / 100);
  }, []);

  const handleExportWav = useCallback(async () => {
    const { save } = await import("@tauri-apps/plugin-dialog");
    const path = await save({
      defaultPath: "export.wav",
      filters: [{ name: "WAV Audio", extensions: ["wav"] }],
    });
    if (!path) return;
    setExporting(true);
    try {
      await ipc.exportWav(path, 60);
      alert(`Exported to ${path}`);
    } catch (e) {
      alert(`Export failed: ${e}`);
    } finally {
      setExporting(false);
    }
  }, []);

  return (
    <div className={styles.topBar}>
      <div className={styles.projectInfo}>
        <SeraphMark className={styles.mark} />
        <span className={styles.projectName}>{projectMeta?.name ?? "Seraph"}</span>
        {projectMeta && (
          <>
            <span className={styles.separator}>|</span>
            <span className={styles.detail}>{projectMeta.tempo} BPM</span>
            <span className={styles.detail}>
              {projectMeta.timeSignature[0]}/{projectMeta.timeSignature[1]}
            </span>
            <span className={styles.driverBadge}>Flamedriver</span>
          </>
        )}
      </div>
      <div className={styles.actions}>
        <button className={styles.btn} onClick={onNewProject}>New</button>
        <button className={styles.btn} onClick={onOpenProject}>Open</button>
        <button className={styles.btn} onClick={onImport}>Import</button>
        {projectMeta && (
          <button className={styles.btn} onClick={onSave}>
            Save
            {dirty && (
              <span className={styles.dirtyDot} title="Unsaved changes">
                ●
              </span>
            )}
          </button>
        )}
        {onExport && (
          <button className={styles.btn} onClick={onExport}>Export</button>
        )}
        {projectMeta && (
          <button className={styles.btn} onClick={handleExportWav} disabled={exporting}>
            {exporting ? "Rendering..." : "Export WAV"}
          </button>
        )}
        {showSaved && <span className={styles.saved}>Saved</span>}
      </div>
      {projectMeta ? (
        <TransportControls
          projectMeta={projectMeta}
          playing={playing}
          loopEnabled={loopEnabled}
          onPlayingChange={onPlayingChange}
          onToggleLoop={onToggleLoop}
          onSeek={onSeek}
        />
      ) : (
        <div className={styles.transport}>
          <button className={styles.transportBtn} disabled>&#9654;</button>
          <button className={styles.transportBtn} disabled>&#9632;</button>
          <button className={styles.transportBtn} disabled>&#8635;</button>
        </div>
      )}
      <div className={styles.masterVol}>
        <span className={styles.masterVolLabel}>Vol</span>
        <input
          type="range"
          min="0"
          max="150"
          value={masterVol}
          onChange={handleMasterVol}
          className={styles.masterVolSlider}
          title={`Master: ${masterVol}%`}
        />
        <span className={styles.masterVolValue}>{masterVol}%</span>
      </div>
    </div>
  );
}
