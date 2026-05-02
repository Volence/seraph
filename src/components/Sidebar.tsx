import { useState } from "react";
import type { SongMetadata, SelectedInstrument } from "../types/model";
import styles from "./Sidebar.module.css";

interface SidebarProps {
  projectMeta: SongMetadata;
  selectedInstrument: SelectedInstrument | null;
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
}

export function Sidebar({ projectMeta: _projectMeta, selectedInstrument: _selectedInstrument, onSelectInstrument: _onSelectInstrument }: SidebarProps) {
  const [activeTab, setActiveTab] = useState<"tracks" | "instruments">("instruments");

  return (
    <div className={styles.sidebar}>
      <div className={styles.tabs}>
        <button
          className={`${styles.tab} ${activeTab === "tracks" ? styles.active : ""}`}
          onClick={() => setActiveTab("tracks")}
        >
          Tracks
        </button>
        <button
          className={`${styles.tab} ${activeTab === "instruments" ? styles.active : ""}`}
          onClick={() => setActiveTab("instruments")}
        >
          Instruments
        </button>
      </div>
      <div className={styles.content}>
        {activeTab === "tracks" && (
          <p className={styles.placeholder}>Track list — Task 6</p>
        )}
        {activeTab === "instruments" && (
          <p className={styles.placeholder}>Instrument browser — Task 6</p>
        )}
      </div>
    </div>
  );
}
