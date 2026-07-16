import { useCallback, useEffect, useRef, useState } from "react";
import type { LibraryEntryDetail, LibraryListEntry } from "../bindings";
import * as lib from "../api/library";
import { formatTags } from "../lib/formatTags";
import { LibraryRootsDialog } from "./LibraryRootsDialog";
import { PianoKeys } from "../widgets/PianoKeys";
import styles from "./LibraryPanel.module.css";

const RENDER_CAP = 400; // plain .map per codebase idiom; search narrows results

interface LibraryPanelProps {
  /** Bump to refresh (e.g. after save-from-project). */
  refreshToken?: number;
  onInstrumentAdded: () => void;
}

/** Tiny volume-envelope sparkline; loop point marked with a vertical line. */
function PsgSparkline({ seq, loopPoint }: { seq: number[]; loopPoint: number | null }) {
  const w = 240;
  const h = 36;
  const n = Math.max(seq.length, 1);
  const stepX = w / n;
  const y = (v: number) => h - 2 - (Math.min(Math.max(v, 0), 15) / 15) * (h - 4);
  const points = seq.map((v, i) => `${((i + 0.5) * stepX).toFixed(1)},${y(v).toFixed(1)}`).join(" ");
  return (
    <svg className={styles.sparkline} viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none">
      {loopPoint !== null && loopPoint < n && (
        <line
          className={styles.sparkLoop}
          x1={(loopPoint + 0.5) * stepX} y1={0}
          x2={(loopPoint + 0.5) * stepX} y2={h}
        />
      )}
      <polyline className={styles.sparkLine} points={points} fill="none" />
    </svg>
  );
}

function DetailCard({ detail }: { detail: LibraryEntryDetail }) {
  const inst = detail.instrument;
  return (
    <>
      <div className={styles.detailHead}>
        <span className={styles.detailName}>{detail.name}</span>
        <span className={styles.detailGame}>{detail.game}</span>
      </div>
      {inst.type === "fm" ? (
        <>
          <div className={styles.detailMeta}>
            ALG {inst.instrument.algorithm} · FB {inst.instrument.feedback}
          </div>
          <table className={styles.opTable}>
            <thead>
              <tr>
                <th>Op</th><th>MUL</th><th>DT</th><th>TL</th><th>AR</th>
                <th>D1R</th><th>D2R</th><th>SL</th><th>RR</th>
              </tr>
            </thead>
            <tbody>
              {inst.instrument.operators.map((op, i) => (
                <tr key={i}>
                  <td>{i + 1}</td>
                  <td>{op.multiple}</td>
                  <td>{op.detune}</td>
                  <td>{op.totalLevel}</td>
                  <td>{op.attackRate}</td>
                  <td>{op.d1r}</td>
                  <td>{op.d2r}</td>
                  <td>{op.sustainLevel}</td>
                  <td>{op.releaseRate}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </>
      ) : (
        <>
          <PsgSparkline seq={inst.instrument.volumeSequence} loopPoint={inst.instrument.loopPoint} />
          <div className={styles.detailMeta}>
            {inst.instrument.loopPoint !== null ? `loop @ ${inst.instrument.loopPoint}` : "no loop"}
            {inst.instrument.silenceOnEnd ? " · silence on end" : ""}
            {inst.instrument.smpsEnvelopeIndex != null ? ` · SMPS env ${inst.instrument.smpsEnvelopeIndex}` : ""}
          </div>
        </>
      )}
    </>
  );
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
  const [selectedHash, setSelectedHash] = useState<string | null>(null);
  const [detail, setDetail] = useState<LibraryEntryDetail | null>(null);
  // Warnings strip: scan/quarantine warnings (re-fetched on every refresh) +
  // locally-appended IPC/import errors. Dismissing remembers the scan set so
  // it stays hidden until a rescan produces something new.
  const [scanWarnings, setScanWarnings] = useState<string[]>([]);
  const [localErrors, setLocalErrors] = useState<string[]>([]);
  const [dismissedScanKey, setDismissedScanKey] = useState<string | null>(null);
  // True between a successful audition start and its stop — guards
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

  async function selectEntry(hash: string) {
    setSelectedHash(hash);
    setDetail(null);
    try {
      setDetail(await lib.libraryGetEntry(hash));
    } catch (e) {
      pushError(e);
    }
  }

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

  async function startAudition(hash: string, midiNote: number) {
    auditioningRef.current = true;
    try {
      await lib.libraryAudition(hash, midiNote);
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
          <div
            key={e.hash}
            className={e.hash === selectedHash ? `${styles.item} ${styles.itemSelected}` : styles.item}
            onClick={() => selectEntry(e.hash)}
          >
            <span
              className={`${styles.dot} ${e.kind === "fm" ? styles.fmDot : styles.psgDot}`}
            />
            <span
              className={styles.itemName}
              title={`${e.game} — audition (hold)`}
              onMouseDown={() => startAudition(e.hash, 60)}
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
              onClick={(ev) => { ev.stopPropagation(); toggleFavorite(e); }}
            >★</button>
            <button
              className={styles.addBtn}
              title="Add to project"
              onClick={(ev) => { ev.stopPropagation(); addToProject(e.hash); }}
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
      {selectedHash !== null && detail !== null && (
        <div className={styles.detail}>
          <DetailCard detail={detail} />
          <PianoKeys
            onNoteOn={(note) => startAudition(selectedHash, note)}
            onNoteOff={stopAudition}
          />
        </div>
      )}
      {rootsOpen && <LibraryRootsDialog onClose={async () => { setRootsOpen(false); await refresh(); }} />}
    </div>
  );
}
