import { useState, useEffect, useCallback } from "react";
import type { SongMetadata, SelectedInstrument } from "./types/model";
import * as ipc from "./api/ipc";
import { TopBar } from "./components/TopBar";
import { Sidebar } from "./components/Sidebar";
import { MainArea } from "./components/MainArea";
import { BottomPanel } from "./components/BottomPanel";
import { NewProjectDialog } from "./components/NewProjectDialog";
import styles from "./App.module.css";

export default function App() {
  const [projectMeta, setProjectMeta] = useState<SongMetadata | null>(null);
  const [showSaved, setShowSaved] = useState(false);
  const [showNewProject, setShowNewProject] = useState(false);
  const [selectedInstrument, setSelectedInstrument] = useState<SelectedInstrument | null>(null);

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
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave]);

  async function handleOpenProject() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({ directory: true, title: "Open Project" });
    if (!selected) return;
    try {
      if (projectOpen) await ipc.closeProject();
      const song = await ipc.openProject(selected as string);
      setProjectMeta(song.metadata);
      setSelectedInstrument(null);
    } catch (e) {
      console.error("Open failed:", e);
    }
  }

  function handleProjectCreated(meta: SongMetadata) {
    setProjectMeta(meta);
    setShowNewProject(false);
    setSelectedInstrument(null);
  }

  return (
    <div className={styles.app}>
      <TopBar
        projectMeta={projectMeta}
        onNewProject={() => setShowNewProject(true)}
        onOpenProject={handleOpenProject}
        onSave={handleSave}
        showSaved={showSaved}
      />
      <div className={styles.body}>
        {projectOpen && (
          <Sidebar
            projectMeta={projectMeta}
            selectedInstrument={selectedInstrument}
            onSelectInstrument={setSelectedInstrument}
          />
        )}
        <MainArea
          projectOpen={projectOpen}
          onNewProject={() => setShowNewProject(true)}
          onOpenProject={handleOpenProject}
        />
      </div>
      {projectOpen && (
        <BottomPanel selectedInstrument={selectedInstrument} />
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
