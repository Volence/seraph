import { useCallback, useEffect, useState } from "react";
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
  const [warnings, setWarnings] = useState<string[]>([]);

  const refresh = useCallback(async () => {
    const filter = {
      text: text || null,
      kind: kind === "all" ? null : kind,
      game: game === "all" ? null : game,
      tag: null,
      favoritesOnly: favOnly,
    };
    setEntries(await lib.libraryList(filter));
    setGames(await lib.libraryGames());
  }, [text, kind, game, favOnly]);

  useEffect(() => { refresh(); }, [refresh, refreshToken]);

  // Scan/quarantine warnings from the last rescan — fetch once on mount.
  useEffect(() => {
    lib.libraryWarnings().then(setWarnings).catch((e) => console.error("Library warnings:", e));
  }, []);

  async function handleImport() {
    const { open: openFileDialog } = await import("@tauri-apps/plugin-dialog");
    const picked = await openFileDialog({
      multiple: true,
      filters: [{ name: "FM instruments", extensions: ["tfi", "vgi", "y12", "gyb"] }],
    });
    if (!picked) return;
    const paths = Array.isArray(picked) ? picked : [picked];
    const result = await lib.libraryImportFiles(paths as string[]);
    if (result.errors.length > 0) setWarnings((w) => [...w, ...result.errors]);
    await refresh();
  }

  async function saveTags(hash: string) {
    await lib.librarySetTags(hash, tagDraft.split(",").map((t) => t.trim()).filter(Boolean));
    setEditingTags(null);
    await refresh();
  }

  if (!open) {
    return (
      <button className={styles.rail} onClick={() => setOpen(true)} title="Open library">
        Lib
      </button>
    );
  }

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
            <button className={styles.warningsClose} onClick={() => setWarnings([])} title="Dismiss">x</button>
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
              onMouseDown={() => lib.libraryAudition(e.hash, 60)}
              onMouseUp={() => lib.libraryStopAudition()}
              onMouseLeave={() => lib.libraryStopAudition()}
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
              onClick={async () => { await lib.librarySetFavorite(e.hash, !e.favorite); refresh(); }}
            >★</button>
            <button
              className={styles.addBtn}
              title="Add to project"
              onClick={async () => { await lib.libraryAddToProject(e.hash); onInstrumentAdded(); }}
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
