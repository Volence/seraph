import { useState, useEffect, useCallback } from "react";
import type { SongMetadata, SelectedInstrument, SelectedRegion } from "./types/model";
import * as ipc from "./api/ipc";
import { TopBar } from "./components/TopBar";
import { MainArea } from "./components/MainArea";
import { BottomPanel } from "./components/BottomPanel";
import { NewProjectDialog } from "./components/NewProjectDialog";
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

  const projectOpen = projectMeta !== null;

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
        e.preventDefault();
        handleSave();
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
  }, [handleSave, playing, loopEnabled, projectMeta]);

  async function handleOpenProject() {
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

  async function handleImport() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const sourcePath = await open({
      title: "Select SMPS Assembly File",
      filters: [{ name: "SMPS Assembly", extensions: ["asm"] }],
    });
    if (!sourcePath) return;

    const parentDir = await open({
      directory: true,
      title: "Choose Where to Save Imported Project",
    });
    if (!parentDir) return;

    try {
      if (projectOpen) await ipc.closeProject();
      setPlaying(false);

      const result = await ipc.importSong(sourcePath as string, parentDir as string);
      const song = await ipc.openProject(result.projectDir);
      setProjectMeta(song.metadata);
      setSelectedInstrument(null);
      setSelectedRegions([]);

      if (result.warnings.length > 0) {
        setImportWarnings(result.warnings);
      }
    } catch (e: any) {
      console.error("Import failed:", e);
      alert(`Import failed: ${e?.message ?? e}`);
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
        onImport={handleImport}
        showSaved={showSaved}
        playing={playing}
        loopEnabled={loopEnabled}
        onPlayingChange={setPlaying}
        onLoopChange={setLoopEnabled}
      />
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
        <MainArea
          projectOpen={projectOpen}
          projectMeta={projectMeta}
          playing={playing}
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
        />
      )}
      {showNewProject && (
        <NewProjectDialog
          onClose={() => setShowNewProject(false)}
          onCreated={handleProjectCreated}
        />
      )}
    </div>
  );
}
