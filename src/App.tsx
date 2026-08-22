import { useState, useEffect, useCallback, useRef } from "react";
import type { SongMetadata, SelectedInstrument, SelectedRegion } from "./types/model";
import * as ipc from "./api/ipc";
import { TopBar } from "./components/TopBar";
import { SpectrumAnalyzer } from "./components/SpectrumAnalyzer";
import { MainArea } from "./components/MainArea";
import { BottomPanel } from "./components/BottomPanel";
import { StatusBar } from "./components/StatusBar";
import { NewProjectDialog } from "./components/NewProjectDialog";
import { ImportDialog } from "./components/ImportDialog";
import { LibraryPanel } from "./components/LibraryPanel";
import { isEditableTarget } from "./utils/keyboard";
import { recordPlayStart, recordStop, consumeStopDoubleTap } from "./utils/transportMemory";
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

  // The white seek cursor. Owned here (not in ArrangementView) so it can be
  // re-synced to the transport's real tick when playback stops (G29) and
  // repositioned by transport shortcuts. The generation counter discards
  // stale async stop-syncs that would otherwise clobber a newer manual seek.
  const [seekTick, setSeekTick] = useState(0);
  const seekGenRef = useRef(0);

  const projectOpen = projectMeta !== null;

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
    } catch (e) {
      console.error("Save failed:", e);
    }
  }, [projectMeta]);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        // Deliberately above the editable-target guard: saving must work
        // (and suppress the browser default) even while typing in a field.
        e.preventDefault();
        handleSave();
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
        handleSeek(0);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, handleSeek, startPlayback, playing, loopEnabled, projectMeta]);

  async function handleOpenProject() {
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
        onNewProject={() => setShowNewProject(true)}
        onOpenProject={handleOpenProject}
        onSave={handleSave}
        onExport={projectOpen ? handleExport : undefined}
        onImport={() => setShowImportDialog(true)}
        showSaved={showSaved}
        playing={playing}
        loopEnabled={loopEnabled}
        onPlayingChange={setPlaying}
        onLoopChange={setLoopEnabled}
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
          onNewProject={() => setShowNewProject(true)}
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
