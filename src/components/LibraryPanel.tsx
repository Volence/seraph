import { useCallback, useEffect, useRef, useState } from "react";
import type { LibraryListEntry } from "../bindings";
import * as lib from "../api/library";
import { formatTags } from "../lib/formatTags";
import { LibraryRootsDialog } from "./LibraryRootsDialog";
import styles from "./LibraryPanel.module.css";

const RENDER_CAP = 400; // plain .map per codebase idiom; search narrows results

interface LibraryPanelProps {
  /** Bump to refresh (e.g. after save-from-project). */
  refreshToken?: number;
  onInstrumentAdded: () => void;
}

export function LibraryPanel({ refreshToken, onInstrumentAdded }: LibraryPanelProps) {
  const [open, setOpen] = useState(true);
  const [entries, setEntries] = useState<LibraryListEntry[]>([]);
  const [games, setGames] = useState<string[]>([]);
  const [text, setText] = useState("");
  const [kind, setKind] = useState<"all" | "fm" | "psg">("all");
  const [game, setGame] = useState<string>("all");
  const [favOnly, setFavOnly] = useState(false);
  const [editingTags, setEditingTags] = useState<string | null>(null); // hash
  const [tagDraft, setTagDraft] = useState("");
  const [rootsOpen, setRootsOpen] = useState(false);
  // Warnings strip: scan/quarantine warnings (re-fetched on every refresh) +
  // locally-appended IPC/import errors. Dismissing remembers the scan set so
  // it stays hidden until a rescan produces something new.
  const [scanWarnings, setScanWarnings] = useState<string[]>([]);
  const [localErrors, setLocalErrors] = useState<string[]>([]);
  const [dismissedScanKey, setDismissedScanKey] = useState<string | null>(null);
  // True between a successful audition mousedown and its stop — guards
  // onMouseUp/onMouseLeave so hovering through the list never fires stop.
  const auditioningRef = useRef(false);
  // Guards the Enter-then-blur double-fire when committing a tag edit.
  const savingTagsRef = useRef(false);

  const pushError = useCallback((e: unknown) => {
    setLocalErrors((errs) => [...errs, String(e)]);
  }, []);

  const refresh = useCallback(async () => {
    try {
      const filter = {
        text: text || null,
        kind: kind === "all" ? null : kind,
        game: game === "all" ? null : game,
        tag: null,
        favoritesOnly: favOnly,
      };
      setEntries(await lib.libraryList(filter));
      setGames(await lib.libraryGames());
      setScanWarnings(await lib.libraryWarnings());
    } catch (e) {
      pushError(e);
    }
  }, [text, kind, game, favOnly, pushError]);

  useEffect(() => { refresh(); }, [refresh, refreshToken]);

  async function handleImport() {
    try {
      const { open: openFileDialog } = await import("@tauri-apps/plugin-dialog");
      const picked = await openFileDialog({
        multiple: true,
        filters: [{ name: "FM instruments", extensions: ["tfi", "vgi", "y12", "gyb"] }],
      });
      if (!picked) return;
      const paths = Array.isArray(picked) ? picked : [picked];
      const result = await lib.libraryImportFiles(paths as string[]);
      if (result.errors.length > 0) setLocalErrors((errs) => [...errs, ...result.errors]);
      await refresh();
    } catch (e) {
      pushError(e);
    }
  }

  async function saveTags(hash: string) {
    if (savingTagsRef.current) return;
    savingTagsRef.current = true;
    try {
      await lib.librarySetTags(hash, tagDraft.split(",").map((t) => t.trim()).filter(Boolean));
      setEditingTags(null);
      await refresh();
    } catch (e) {
      pushError(e);
    } finally {
      savingTagsRef.current = false;
    }
  }

  async function startAudition(hash: string) {
    auditioningRef.current = true;
    try {
      await lib.libraryAudition(hash, 60);
    } catch (e) {
      auditioningRef.current = false;
      pushError(e);
    }
  }

  async function stopAudition() {
    if (!auditioningRef.current) return;
    auditioningRef.current = false;
    try {
      await lib.libraryStopAudition();
    } catch (e) {
      pushError(e);
    }
  }

  async function toggleFavorite(e: LibraryListEntry) {
    try {
      await lib.librarySetFavorite(e.hash, !e.favorite);
      await refresh();
    } catch (err) {
      pushError(err);
    }
  }

  async function addToProject(hash: string) {
    try {
      await lib.libraryAddToProject(hash);
      onInstrumentAdded();
    } catch (e) {
      pushError(e);
    }
  }

  if (!open) {
    return (
      <button className={styles.rail} onClick={() => setOpen(true)} title="Open library">
        Lib
      </button>
    );
  }

  const shownScan = JSON.stringify(scanWarnings) !== dismissedScanKey ? scanWarnings : [];
  const warnings = [...shownScan, ...localErrors];
  const shown = entries.slice(0, RENDER_CAP);
  return (
    <div className={styles.panel}>
      <div className={styles.header}>
        <span className={styles.title}>Library</span>
        <button className={styles.headerBtn} onClick={() => setRootsOpen(true)} title="Library folders">⚙</button>
        <button className={styles.headerBtn} onClick={handleImport} title="Import instrument files">Import</button>
        <button className={styles.headerBtn} onClick={() => setOpen(false)} title="Collapse">«</button>
      </div>
      {warnings.length > 0 && (
        <div className={styles.warnings}>
          <div className={styles.warningsHeader}>
            <span>{warnings.length} library warning{warnings.length !== 1 ? "s" : ""}</span>
            <button
              className={styles.warningsClose}
              onClick={() => {
                setDismissedScanKey(JSON.stringify(scanWarnings));
                setLocalErrors([]);
              }}
              title="Dismiss"
            >x</button>
          </div>
          <ul>
            {warnings.map((w, i) => <li key={i}>{w}</li>)}
          </ul>
        </div>
      )}
      <input
        className={styles.search}
        placeholder="Search name, game, tag…"
        value={text}
        onChange={(e) => setText(e.target.value)}
      />
      <div className={styles.filters}>
        {(["all", "fm", "psg"] as const).map((k) => (
          <button key={k} className={kind === k ? styles.chipActive : styles.chip} onClick={() => setKind(k)}>
            {k.toUpperCase()}
          </button>
        ))}
        <button
          className={favOnly ? styles.chipActive : styles.chip}
          onClick={() => setFavOnly(!favOnly)}
          title="Favorites only"
        >★</button>
        <select className={styles.gameSelect} value={game} onChange={(e) => setGame(e.target.value)}>
          <option value="all">All games</option>
          {games.map((g) => <option key={g} value={g}>{g}</option>)}
        </select>
      </div>
      <div className={styles.list}>
        {shown.map((e) => (
          <div key={e.hash} className={styles.item}>
            <span
              className={`${styles.dot} ${e.kind === "fm" ? styles.fmDot : styles.psgDot}`}
            />
            <span
              className={styles.itemName}
              title={`${e.game} — audition (hold)`}
              onMouseDown={() => startAudition(e.hash)}
              onMouseUp={stopAudition}
              onMouseLeave={stopAudition}
            >
              {e.name}
            </span>
            <span
              className={styles.itemTags}
              title="Double-click to edit tags"
              onDoubleClick={() => { setEditingTags(e.hash); setTagDraft(e.tags.join(", ")); }}
            >
              {formatTags(e.tags)}
            </span>
            <button
              className={e.favorite ? styles.starOn : styles.star}
              title={e.favorite ? "Unfavorite" : "Favorite"}
              onClick={() => toggleFavorite(e)}
            >★</button>
            <button
              className={styles.addBtn}
              title="Add to project"
              onClick={() => addToProject(e.hash)}
            >+</button>
            {editingTags === e.hash && (
              <input
                className={styles.tagInput}
                autoFocus
                value={tagDraft}
                onChange={(ev) => setTagDraft(ev.target.value)}
                onKeyDown={(ev) => { if (ev.key === "Enter") saveTags(e.hash); if (ev.key === "Escape") setEditingTags(null); }}
                onBlur={() => saveTags(e.hash)}
              />
            )}
          </div>
        ))}
        {entries.length > RENDER_CAP && (
          <div className={styles.moreNote}>{entries.length - RENDER_CAP} more — refine your search</div>
        )}
      </div>
      {rootsOpen && <LibraryRootsDialog onClose={async () => { setRootsOpen(false); await refresh(); }} />}
    </div>
  );
}
