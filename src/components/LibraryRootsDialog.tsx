import { useCallback, useEffect, useState } from "react";
import type { RootInfo } from "../bindings";
import * as lib from "../api/library";
import styles from "./LibraryRootsDialog.module.css";

interface LibraryRootsDialogProps {
  onClose: () => void;
}

export function LibraryRootsDialog({ onClose }: LibraryRootsDialogProps) {
  const [roots, setRoots] = useState<RootInfo[]>([]);
  const [error, setError] = useState("");

  const refresh = useCallback(async () => {
    try {
      setRoots(await lib.libraryRootsGet());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  async function handleAdd() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, title: "Add Library Folder" });
    if (!selected) return;
    try {
      setError("");
      await lib.libraryRootAdd(selected as string);
      await lib.libraryRescan();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function handleRemove(path: string) {
    try {
      setError("");
      await lib.libraryRootRemove(path);
      await lib.libraryRescan();
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <h2 className={styles.title}>Library folders</h2>

        <div className={styles.rootList}>
          {roots.map((r) => (
            <div key={r.path} className={styles.rootRow}>
              <span className={styles.rootLabel} title={r.path}>{r.label}</span>
              <span className={styles.kindBadge}>{r.kind}</span>
              {r.kind === "custom" && (
                <button className={styles.removeBtn} onClick={() => handleRemove(r.path)}>
                  Remove
                </button>
              )}
            </div>
          ))}
          {roots.length === 0 && <p className={styles.empty}>No library folders.</p>}
        </div>

        {error && <p className={styles.error}>{error}</p>}

        <div className={styles.buttons}>
          <button className={styles.addBtn} onClick={handleAdd}>Add folder…</button>
          <button className={styles.closeBtn} onClick={onClose}>Close</button>
        </div>
      </div>
    </div>
  );
}
