import { useState, useEffect } from "react";
import type { SongMetadata, DriverInfo } from "../types/model";
import * as ipc from "../api/ipc";
import { getRecentLocations, mostRecentLocation, rememberLocation } from "../utils/recentLocations";
import styles from "./NewProjectDialog.module.css";

interface NewProjectDialogProps {
  onClose: () => void;
  onCreated: (meta: SongMetadata) => void;
}

export function NewProjectDialog({ onClose, onCreated }: NewProjectDialogProps) {
  const [name, setName] = useState("");
  // Prefill with the last location a project was created/opened in.
  const [location, setLocation] = useState(() => mostRecentLocation());
  const [recentLocations] = useState(() => getRecentLocations());
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [drivers, setDrivers] = useState<DriverInfo[]>([]);
  const [driverId, setDriverId] = useState("");
  const [tempo, setTempo] = useState(120);
  const [timeSigNum, setTimeSigNum] = useState(4);
  const [timeSigDen, setTimeSigDen] = useState(4);
  const [error, setError] = useState("");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    ipc.listDrivers().then((list) => {
      setDrivers(list);
      if (list.length > 0) setDriverId(list[0].id);
    });
  }, []);

  async function handleBrowse() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      title: "Choose Project Location",
      defaultPath: mostRecentLocation() || undefined,
    });
    if (selected) setLocation(selected as string);
  }

  async function handleCreate() {
    if (!name.trim()) { setError("Name is required"); return; }
    if (!location.trim()) { setError("Location is required"); return; }
    if (!driverId) { setError("Select a driver"); return; }

    setCreating(true);
    setError("");
    try {
      const fullPath = `${location}/${name.replace(/[^a-zA-Z0-9_-]/g, "_")}`;
      await ipc.createProject(fullPath, name, driverId, tempo, timeSigNum, timeSigDen);
      rememberLocation(location);
      const meta = await ipc.getProjectInfo();
      if (meta) onCreated(meta);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <h2 className={styles.title}>New Project</h2>

        <label className={styles.label}>
          Name
          <input
            className={styles.input}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="My Song"
            autoFocus
          />
        </label>

        <label className={styles.label}>
          Location
          <div className={styles.browseRow}>
            <div className={styles.locationWrap}>
              <input
                className={styles.input}
                value={location}
                onChange={(e) => setLocation(e.target.value)}
                onFocus={() => setShowSuggestions(true)}
                onBlur={() => setShowSuggestions(false)}
                placeholder="/path/to/projects"
              />
              {showSuggestions && recentLocations.length > 0 && (
                <ul className={styles.suggestions} role="listbox" aria-label="Recent locations">
                  {recentLocations.map((dir) => (
                    <li
                      key={dir}
                      role="option"
                      aria-selected={dir === location}
                      className={styles.suggestion}
                      // mouseDown (not click) so it wins over the input's blur.
                      onMouseDown={(e) => {
                        e.preventDefault();
                        setLocation(dir);
                        setShowSuggestions(false);
                      }}
                    >
                      {dir}
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <button className={styles.browseBtn} onClick={handleBrowse}>Browse</button>
          </div>
        </label>

        <label className={styles.label}>
          Driver
          <select
            className={styles.select}
            value={driverId}
            onChange={(e) => setDriverId(e.target.value)}
          >
            {drivers.map((d) => (
              <option key={d.id} value={d.id}>{d.name}</option>
            ))}
          </select>
        </label>

        <div className={styles.row}>
          <label className={styles.label}>
            Tempo
            <input
              className={styles.input}
              type="number"
              min={20}
              max={300}
              value={tempo}
              onChange={(e) => setTempo(Number(e.target.value))}
            />
          </label>
          <label className={styles.label}>
            Time Signature
            <div className={styles.timeSigRow}>
              <input
                className={styles.smallInput}
                type="number"
                min={1}
                max={12}
                value={timeSigNum}
                onChange={(e) => setTimeSigNum(Number(e.target.value))}
              />
              <span>/</span>
              <select
                className={styles.smallSelect}
                value={timeSigDen}
                onChange={(e) => setTimeSigDen(Number(e.target.value))}
              >
                <option value={2}>2</option>
                <option value={4}>4</option>
                <option value={8}>8</option>
                <option value={16}>16</option>
              </select>
            </div>
          </label>
        </div>

        {error && <p className={styles.error}>{error}</p>}

        <div className={styles.buttons}>
          <button className={styles.cancelBtn} onClick={onClose}>Cancel</button>
          <button className={styles.createBtn} onClick={handleCreate} disabled={creating}>
            {creating ? "Creating..." : "Create"}
          </button>
        </div>
      </div>
    </div>
  );
}
