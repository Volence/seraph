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
import styles from "./App.module.css";

export default function App() {
  const [projectMeta, setProjectMeta] = useState<SongMetadata | null>(null);
  const [showSaved, setShowSaved] = useState(false);
  const [showNewProject, setShowNewProject] = useState(false);
  const [selectedInstrument, setSelectedInstrument] = useState<SelectedInstrument | null>(null);
  const [playing, setPlaying] = useState(false);
  const [loopEnabled, setLoopEnabled] = useState(false);
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
      if (e.key === " " && projectMeta) {
        e.preventDefault();
        if (playing) {
          ipc.transportStop();
          setPlaying(false);
        } else {
          ipc.transportPlay();
          setPlaying(true);
        }
      }
      if (e.key === "l" && projectMeta && !e.ctrlKey && !e.metaKey) {
        if (loopEnabled) {
          ipc.transportClearLoop();
          setLoopEnabled(false);
        } else {
          const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
          ipc.transportSetLoop(0, ticksPerBar * 4);
          setLoopEnabled(true);
        }
      }
      if (e.key === "Home" && projectMeta) {
        ipc.transportSeek(0);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, handleRevert, playing, loopEnabled, projectMeta]);

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
        onLoopChange={setLoopEnabled}
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
