import { useState, useCallback } from "react";
import type { SongMetadata } from "../types/model";
import { TransportControls } from "./TransportControls";
import * as ipc from "../api/ipc";
import styles from "./TopBar.module.css";
import { SeraphMark } from "../assets/SeraphMark";

interface TopBarProps {
  projectMeta: SongMetadata | null;
  /** Tempo/time-sig were edited inline; App refreshes its projectMeta so
   *  every grid consumer picks up the new bar math. */
  onProjectMetaChange: (meta: SongMetadata) => void;
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
  onProjectMetaChange,
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
  // Inline tempo/time-sig editing (Knob edit-field idiom: Enter commits,
  // Esc cancels). Text state so partial input never round-trips to the IPC.
  const [editingMeta, setEditingMeta] = useState(false);
  const [tempoText, setTempoText] = useState("");
  const [sigNumText, setSigNumText] = useState("");
  const [sigDen, setSigDen] = useState(4);

  function beginMetaEdit() {
    if (!projectMeta) return;
    setTempoText(String(projectMeta.tempo));
    setSigNumText(String(projectMeta.timeSignature[0]));
    setSigDen(projectMeta.timeSignature[1]);
    setEditingMeta(true);
  }

  async function commitMetaEdit() {
    if (!projectMeta) return;
    const tempo = parseFloat(tempoText);
    const num = parseInt(sigNumText, 10);
    if (isNaN(tempo) || isNaN(num)) {
      setEditingMeta(false);
      return;
    }
    try {
      const meta = await ipc.updateProjectMetadata(tempo, num, sigDen);
      onProjectMetaChange(meta);
      // Tempo/time-sig feed the sequencer snapshot — rebuild it so the
      // next playback runs at the new tempo.
      await ipc.reloadSequence();
    } catch (e) {
      alert(`Update failed: ${e}`);
    }
    setEditingMeta(false);
  }

  function handleMetaKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") commitMetaEdit();
    else if (e.key === "Escape") setEditingMeta(false);
  }
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
            {editingMeta ? (
              <span className={styles.metaEdit} onKeyDown={handleMetaKeyDown}>
                <input
                  className={styles.metaInput}
                  type="number"
                  min={20}
                  max={300}
                  value={tempoText}
                  onChange={(e) => setTempoText(e.target.value)}
                  title="Tempo (BPM), Enter commits, Esc cancels"
                  autoFocus
                />
                <span className={styles.detail}>BPM</span>
                <input
                  className={styles.metaInput}
                  type="number"
                  min={1}
                  max={16}
                  value={sigNumText}
                  onChange={(e) => setSigNumText(e.target.value)}
                  title="Time signature numerator (1-16)"
                />
                <span className={styles.detail}>/</span>
                <select
                  className={styles.metaSelect}
                  value={sigDen}
                  onChange={(e) => setSigDen(Number(e.target.value))}
                  title="Time signature denominator"
                >
                  <option value={2}>2</option>
                  <option value={4}>4</option>
                  <option value={8}>8</option>
                  <option value={16}>16</option>
                </select>
              </span>
            ) : (
              <>
                <span
                  className={`${styles.detail} ${styles.editable}`}
                  onClick={beginMetaEdit}
                  title="Click to edit tempo / time signature"
                >
                  {projectMeta.tempo} BPM
                </span>
                <span
                  className={`${styles.detail} ${styles.editable}`}
                  onClick={beginMetaEdit}
                  title="Click to edit tempo / time signature"
                >
                  {projectMeta.timeSignature[0]}/{projectMeta.timeSignature[1]}
                </span>
              </>
            )}
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
