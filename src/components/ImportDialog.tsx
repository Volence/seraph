import { useState } from "react";
import * as ipc from "../api/ipc";
import type { SongMetadata } from "../types/model";
import { mostRecentLocation, rememberLocation } from "../utils/recentLocations";
import styles from "./NewProjectDialog.module.css";

const ZYRINX_SONGS: { id: number; name: string }[] = [
  { id: 1, name: "Main Title" },
  { id: 2, name: "Gotham by Night" },
  { id: 3, name: "Big Boss" },
  { id: 4, name: "Joker's Theme" },
  { id: 5, name: "Moving Trucks" },
  { id: 6, name: "Two-Face's Theme" },
  { id: 7, name: "Not Much Time" },
  { id: 8, name: "Flying Over The City" },
  { id: 9, name: "Moving Boss" },
  { id: 10, name: "Extreme Boss" },
  { id: 11, name: "Dark Studio" },
  { id: 12, name: "Animal Boss" },
  { id: 13, name: "Psycho Section" },
  { id: 14, name: "Space Boss" },
  { id: 15, name: "The Lab" },
  { id: 16, name: "Big Machines" },
  { id: 17, name: "Final Boss" },
  { id: 18, name: "Ending" },
  { id: 19, name: "Continue" },
];

interface ImportDialogProps {
  onClose: () => void;
  onImported: (meta: SongMetadata, warnings: ipc.ImportWarning[]) => void;
  projectOpen: boolean;
}

type DriverChoice = "flamedriver" | "zyrinx" | "vgm";

export function ImportDialog({ onClose, onImported, projectOpen }: ImportDialogProps) {
  const [driver, setDriver] = useState<DriverChoice>("flamedriver");
  const [sourcePath, setSourcePath] = useState("");
  const [dacDir, setDacDir] = useState("");
  const [gameId, setGameId] = useState(1);
  // Prefill with the last location a project was created/opened in.
  const [parentDir, setParentDir] = useState(() => mostRecentLocation());
  const [importing, setImporting] = useState(false);
  const [error, setError] = useState("");

  async function browseSource() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    if (driver === "flamedriver") {
      const path = await open({
        title: "Select SMPS Assembly File",
        filters: [{ name: "SMPS Assembly", extensions: ["asm"] }],
      });
      if (path) setSourcePath(path as string);
    } else if (driver === "vgm") {
      const path = await open({
        title: "Select VGM File",
        filters: [{ name: "VGM Audio", extensions: ["vgm", "vgz"] }],
      });
      if (path) setSourcePath(path as string);
    } else {
      const path = await open({
        title: "Select Genesis ROM",
        filters: [{ name: "Genesis ROM", extensions: ["md", "bin", "gen"] }],
      });
      if (path) setSourcePath(path as string);
    }
  }

  async function browseDacDir() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      directory: true,
      title: "Select DAC Samples Directory (optional)",
    });
    if (path) setDacDir(path as string);
  }

  async function browseParentDir() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      directory: true,
      title: "Choose Where to Save Imported Project",
      defaultPath: mostRecentLocation() || undefined,
    });
    if (path) setParentDir(path as string);
  }

  async function handleImport() {
    if (!sourcePath.trim()) { setError("Select a source file"); return; }
    if (!parentDir.trim()) { setError("Select a project location"); return; }

    setImporting(true);
    setError("");
    try {
      if (projectOpen) await ipc.closeProject();

      let result: ipc.ImportResult;
      if (driver === "flamedriver") {
        result = await ipc.importSong(sourcePath, parentDir, dacDir || undefined);
      } else if (driver === "vgm") {
        result = await ipc.importVgm(sourcePath, parentDir);
      } else {
        result = await ipc.importZyrinxSong(sourcePath, parentDir, gameId);
      }

      const song = await ipc.openProject(result.projectDir);
      rememberLocation(parentDir);
      onImported(song.metadata, result.warnings);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setImporting(false);
    }
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <h2 className={styles.title}>Import Song</h2>

        <label className={styles.label}>
          Sound Driver
          <select
            className={styles.select}
            value={driver}
            onChange={(e) => {
              setDriver(e.target.value as DriverChoice);
              setSourcePath("");
            }}
          >
            <option value="flamedriver">Flamedriver (SMPS)</option>
            <option value="zyrinx">Zyrinx (Batman & Robin)</option>
            <option value="vgm">VGM File</option>
          </select>
        </label>

        <label className={styles.label}>
          {driver === "flamedriver" ? "SMPS Assembly File" : driver === "vgm" ? "VGM File" : "Genesis ROM"}
          <div className={styles.browseRow}>
            <input
              className={styles.input}
              value={sourcePath}
              readOnly
              placeholder={driver === "flamedriver" ? "Select .asm file" : driver === "vgm" ? "Select .vgm/.vgz file" : "Select .md/.bin ROM"}
            />
            <button className={styles.browseBtn} onClick={browseSource}>Browse</button>
          </div>
        </label>

        {driver === "flamedriver" && (
          <label className={styles.label}>
            DAC Samples Directory (optional)
            <div className={styles.browseRow}>
              <input
                className={styles.input}
                value={dacDir}
                readOnly
                placeholder="e.g. skdisasm/Sound/DAC/"
              />
              <button className={styles.browseBtn} onClick={browseDacDir}>Browse</button>
            </div>
          </label>
        )}

        {driver === "zyrinx" && (
          <label className={styles.label}>
            Song
            <select
              className={styles.select}
              value={gameId}
              onChange={(e) => setGameId(Number(e.target.value))}
            >
              {ZYRINX_SONGS.map((s) => (
                <option key={s.id} value={s.id}>{s.name}</option>
              ))}
            </select>
          </label>
        )}

        <label className={styles.label}>
          Project Location
          <div className={styles.browseRow}>
            <input
              className={styles.input}
              value={parentDir}
              readOnly
              placeholder="Where to save the project"
            />
            <button className={styles.browseBtn} onClick={browseParentDir}>Browse</button>
          </div>
        </label>

        {error && <p className={styles.error}>{error}</p>}

        <div className={styles.buttons}>
          <button className={styles.cancelBtn} onClick={onClose}>Cancel</button>
          <button className={styles.createBtn} onClick={handleImport} disabled={importing}>
            {importing ? "Importing..." : "Import"}
          </button>
        </div>
      </div>
    </div>
  );
}
