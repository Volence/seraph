import { useState, useEffect, useCallback, useRef } from "react";
import type { SongMetadata, SelectedInstrument, SelectedRegion } from "./types/model";
import * as ipc from "./api/ipc";
import { SONG_REVERTED_EVENT, isEditableTarget } from "./utils/keyboard";
import { TopBar } from "./components/TopBar";
import { SpectrumAnalyzer } from "./components/SpectrumAnalyzer";
import { MainArea } from "./components/MainArea";
import { BottomPanel } from "./components/BottomPanel";
import { StatusBar } from "./components/StatusBar";
import { NewProjectDialog } from "./components/NewProjectDialog";
import { ImportDialog } from "./components/ImportDialog";
import { LibraryPanel } from "./components/LibraryPanel";
import { recordPlayStart, recordStop, consumeStopDoubleTap } from "./utils/transportMemory";
import { defaultPreviewLoop, type PreviewLoopRange } from "./utils/previewLoop";
import styles from "./App.module.css";

export default function App() {
  const [projectMeta, setProjectMeta] = useState<SongMetadata | null>(null);
  const [showSaved, setShowSaved] = useState(false);
  const [showNewProject, setShowNewProject] = useState(false);
  const [selectedInstrument, setSelectedInstrument] = useState<SelectedInstrument | null>(null);
  const [playing, setPlaying] = useState(false);
  const [loopEnabled, setLoopEnabled] = useState(false);
  // The preview loop: transport-scratch state, NOT saved into the project
  // (S4's SongSettingsPanel will own the persisted song-level loop). Holds
  // the last range set from the ruler so the L key / loop button re-arm it.
  const [previewLoop, setPreviewLoop] = useState<PreviewLoopRange | null>(null);
  const [selectedRegions, setSelectedRegions] = useState<SelectedRegion[]>([]);
  const [exportStatus, setExportStatus] = useState<
    | { type: "success"; files: string[] }
    | { type: "error"; errors: ipc.ExportError[] }
    | null
  >(null);
  const [importWarnings, setImportWarnings] = useState<ipc.ImportWarning[] | null>(null);
  const [showImportDialog, setShowImportDialog] = useState(false);
  // Bumped after save-from-project so the LibraryPanel re-queries.
  const [libraryRefresh, setLibraryRefresh] = useState(0);
  const [undoState, setUndoState] = useState<ipc.UndoState>({
    canUndo: false,
    canRedo: false,
    dirty: false,
  });
  // The window-close listener registers once; it reads dirty via this ref.
  const dirtyRef = useRef(false);
  dirtyRef.current = undoState.dirty;

  // The white seek cursor. Owned here (not in ArrangementView) so it can be
  // re-synced to the transport's real tick when playback stops (G29) and
  // repositioned by transport shortcuts. The generation counter discards
  // stale async stop-syncs that would otherwise clobber a newer manual seek.
  const [seekTick, setSeekTick] = useState(0);
  const seekGenRef = useRef(0);

  const projectOpen = projectMeta !== null;

  const refreshUndoState = useCallback(async () => {
    try {
      setUndoState(await ipc.getUndoState());
    } catch (e) {
      console.error("undo-state query failed:", e);
    }
  }, []);

  // Keep the dirty indicator honest: edits happen all over the tree, so
  // poll while a project is open (plus immediate refreshes on save/undo).
  useEffect(() => {
    if (!projectOpen) return;
    refreshUndoState();
    const interval = setInterval(refreshUndoState, 1000);
    return () => clearInterval(interval);
  }, [projectOpen, refreshUndoState]);

  const handleSeek = useCallback((tick: number) => {
    seekGenRef.current++;
    setSeekTick(tick);
    ipc.transportSeek(tick);
  }, []);

  // G29: transport_stop pauses in place, so when playback stops the cursor
  // must move to where the sequencer actually is. get_playback_state's tick
  // field is the source of truth.
  useEffect(() => {
    if (playing || !projectMeta) return;
    const gen = ++seekGenRef.current;
    ipc.getPlaybackState()
      .then((s) => {
        if (seekGenRef.current === gen) setSeekTick(s.tick);
      })
      .catch(() => {});
  }, [playing, projectMeta]);

  const resetSeekCursor = useCallback(() => {
    seekGenRef.current++;
    setSeekTick(0);
  }, []);

  /** Toggle the preview loop using the last-set range (default: one bar at
   *  the seek cursor). Shared by the L key and the transport loop button. */
  const toggleLoop = useCallback(async () => {
    if (!projectMeta) return;
    if (loopEnabled) {
      await ipc.transportClearLoop();
      setLoopEnabled(false);
    } else {
      const range = previewLoop ?? defaultPreviewLoop(seekTick, projectMeta);
      setPreviewLoop(range);
      await ipc.transportSetLoop(range.start, range.end);
      setLoopEnabled(true);
    }
  }, [projectMeta, loopEnabled, previewLoop, seekTick]);

  /** Ruler loop-drag commit: remember the range and arm the loop. */
  const handlePreviewLoopSet = useCallback(async (start: number, end: number) => {
    setPreviewLoop({ start, end });
    await ipc.transportSetLoop(start, end);
    setLoopEnabled(true);
  }, []);

  const startPlayback = useCallback(async () => {
    // Record where playback starts so a stop double-tap can return there
    // (G37). Best-effort: a failed tick read must not block playing.
    try {
      const s = await ipc.getPlaybackState();
      recordPlayStart(s.tick);
    } catch {
      // keep the previous play-start tick
    }
    await ipc.transportPlay();
  }, []);

  const handleSave = useCallback(async () => {
    if (!projectMeta) return;
    try {
      await ipc.saveProject();
      setShowSaved(true);
      setTimeout(() => setShowSaved(false), 2000);
      refreshUndoState();
    } catch (e) {
      console.error("Save failed:", e);
    }
  }, [projectMeta, refreshUndoState]);

  const handleRevert = useCallback(
    async (op: "undo" | "redo") => {
      if (!projectMeta) return;
      try {
        if (op === "undo") await ipc.undo();
        else await ipc.redo();
        // Open views (arrangement, piano roll) re-fetch on this event.
        window.dispatchEvent(new Event(SONG_REVERTED_EVENT));
        await ipc.reloadSequence();
        refreshUndoState();
      } catch (e) {
        console.error(`${op} failed:`, e);
      }
    },
    [projectMeta, refreshUndoState],
  );

  /** Confirm discarding unsaved changes; true = proceed. */
  const confirmDiscard = useCallback(async (action: string) => {
    if (!dirtyRef.current) return true;
    const { ask } = await import("@tauri-apps/plugin-dialog");
    return ask(`You have unsaved changes. ${action} and discard them?`, {
      title: "Unsaved changes",
    });
  }, []);

  // Confirm on window close while dirty (Tauri v2 onCloseRequested).
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    (async () => {
      try {
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const win = getCurrentWindow();
        const fn = await win.onCloseRequested(async (event) => {
          if (!dirtyRef.current) return;
          event.preventDefault();
          const { ask } = await import("@tauri-apps/plugin-dialog");
          const quit = await ask(
            "You have unsaved changes. Quit without saving?",
            { title: "Unsaved changes" },
          );
          if (quit) await win.destroy();
        });
        if (cancelled) fn();
        else unlisten = fn;
      } catch (e) {
        // Non-Tauri environment (tests) or missing permission.
        console.warn("close-confirm unavailable:", e);
      }
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        // Deliberately above the editable-target guard: saving must work
        // (and suppress the browser default) even while typing in a field.
        e.preventDefault();
        handleSave();
      }
      // Undo/redo: Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z, Ctrl/Cmd+Y — but never
      // while typing in a form control (inputs keep their own undo).
      if ((e.ctrlKey || e.metaKey) && projectMeta && !isEditableTarget(e.target)) {
        const key = e.key.toLowerCase();
        if (key === "z" && !e.shiftKey) {
          e.preventDefault();
          handleRevert("undo");
          return;
        }
        if ((key === "z" && e.shiftKey) || key === "y") {
          e.preventDefault();
          handleRevert("redo");
          return;
        }
      }
      // Plain-key shortcuts must not fire while the user is typing (G2).
      if (isEditableTarget(e.target)) return;
      if (e.key === " " && projectMeta) {
        e.preventDefault();
        if (playing) {
          // Space = pause in place (G37 owner ruling).
          ipc.transportStop();
          setPlaying(false);
          recordStop();
        } else {
          const returnTick = consumeStopDoubleTap();
          if (returnTick !== null) {
            // Stop double-tap: return the playhead to where the last
            // playback started, WITHOUT starting playback.
            handleSeek(returnTick);
          } else {
            startPlayback();
            setPlaying(true);
          }
        }
      }
      if (e.key === "l" && projectMeta && !e.ctrlKey && !e.metaKey) {
        toggleLoop();
      }
      if (e.key === "Home" && projectMeta) {
        handleSeek(0);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, handleRevert, handleSeek, startPlayback, toggleLoop, playing, projectMeta]);

  async function handleNewProject() {
    if (!(await confirmDiscard("Create a new project"))) return;
    setShowNewProject(true);
  }

  async function handleOpenProject() {
    if (!(await confirmDiscard("Open another project"))) return;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, title: "Open Project" });
    if (!selected) return;
    try {
      if (projectOpen) await ipc.closeProject();
      setPlaying(false);
      resetSeekCursor();
      const song = await ipc.openProject(selected as string);
      setProjectMeta(song.metadata);
      setSelectedInstrument(null);
      setSelectedRegions([]);
    } catch (e) {
      console.error("Open failed:", e);
    }
  }

  function handleProjectCreated(meta: SongMetadata) {
    setPlaying(false);
    resetSeekCursor();
    setProjectMeta(meta);
    setShowNewProject(false);
    setSelectedInstrument(null);
    setSelectedRegions([]);
  }

  async function handleExport() {
    if (!projectMeta) return;
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, title: "Export Song" });
    if (!selected) return;
    try {
      const result = await ipc.exportSong(selected as string);
      setExportStatus({ type: "success", files: result.files });
      setTimeout(() => setExportStatus(null), 4000);
    } catch (e: any) {
      if (e?.errors) {
        setExportStatus({ type: "error", errors: e.errors });
      } else {
        setExportStatus({
          type: "error",
          errors: [{ trackName: "", regionIndex: null, noteIndex: null, message: String(e) }],
        });
      }
    }
  }

  function handleImported(meta: SongMetadata, warnings: ipc.ImportWarning[]) {
    setPlaying(false);
    resetSeekCursor();
    setProjectMeta(meta);
    setSelectedInstrument(null);
    setSelectedRegions([]);
    setShowImportDialog(false);
    if (warnings.length > 0) {
      setImportWarnings(warnings);
    }
  }

  return (
    <div className={styles.app}>
      <TopBar
        projectMeta={projectMeta}
        onProjectMetaChange={setProjectMeta}
        onNewProject={handleNewProject}
        onOpenProject={handleOpenProject}
        onSave={handleSave}
        onExport={projectOpen ? handleExport : undefined}
        onImport={() => setShowImportDialog(true)}
        showSaved={showSaved}
        dirty={undoState.dirty}
        playing={playing}
        loopEnabled={loopEnabled}
        onPlayingChange={setPlaying}
        onToggleLoop={toggleLoop}
        onSeek={handleSeek}
      />
      <SpectrumAnalyzer height={100} />
      {exportStatus?.type === "success" && (
        <div className={styles.exportSuccess}>
          Exported {exportStatus.files.length} files
        </div>
      )}
      {exportStatus?.type === "error" && (
        <div className={styles.exportError}>
          <div className={styles.exportErrorHeader}>
            <span>Export failed ({exportStatus.errors.length} error{exportStatus.errors.length !== 1 ? "s" : ""})</span>
            <button onClick={() => setExportStatus(null)}>x</button>
          </div>
          <ul>
            {exportStatus.errors.map((e, i) => (
              <li key={i}>{e.trackName ? `${e.trackName}: ` : ""}{e.message}</li>
            ))}
          </ul>
        </div>
      )}
      {importWarnings && (
        <div className={styles.importWarning}>
          <div className={styles.importWarningHeader}>
            <span>Import complete ({importWarnings.length} warning{importWarnings.length !== 1 ? "s" : ""})</span>
            <button onClick={() => setImportWarnings(null)}>x</button>
          </div>
          <ul>
            {importWarnings.map((w, i) => (
              <li key={i}>{w.channel ? `${w.channel}: ` : ""}{w.message}</li>
            ))}
          </ul>
        </div>
      )}
      <div className={styles.body}>
        {projectOpen && (
          <LibraryPanel
            refreshToken={libraryRefresh}
            // No global instrument-refresh callback exists — the BottomPanel
            // editors refetch on selection.
            onInstrumentAdded={() => {}}
          />
        )}
        <MainArea
          projectOpen={projectOpen}
          projectMeta={projectMeta}
          playing={playing}
          seekTick={seekTick}
          onSeek={handleSeek}
          previewLoop={previewLoop}
          loopEnabled={loopEnabled}
          onPreviewLoopSet={handlePreviewLoopSet}
          onNewProject={handleNewProject}
          onOpenProject={handleOpenProject}
          onSelectRegions={(regions) => {
            setSelectedRegions(regions);
            if (regions.length > 0) setSelectedInstrument(null);
          }}
          selectedRegions={selectedRegions}
          onSelectInstrument={(inst) => {
            setSelectedInstrument(inst);
            setSelectedRegions([]);
          }}
          selectedInstrument={selectedInstrument}
        />
      </div>
      {projectOpen && (
        <BottomPanel
          selectedInstrument={selectedInstrument}
          selectedRegion={selectedRegions[selectedRegions.length - 1] ?? null}
          onCloseRegion={() => setSelectedRegions([])}
          playing={playing}
          projectMeta={projectMeta!}
          onSavedToLibrary={() => setLibraryRefresh((n) => n + 1)}
        />
      )}
      <StatusBar />
      {showNewProject && (
        <NewProjectDialog
          onClose={() => setShowNewProject(false)}
          onCreated={handleProjectCreated}
        />
      )}
      {showImportDialog && (
        <ImportDialog
          onClose={() => setShowImportDialog(false)}
          onImported={handleImported}
          projectOpen={projectOpen}
        />
      )}
    </div>
  );
}
