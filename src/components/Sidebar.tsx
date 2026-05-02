import { useState } from "react";
import type { SongMetadata, SelectedInstrument } from "../types/model";
import { TrackList } from "./TrackList";
import { InstrumentBrowser } from "./InstrumentBrowser";
import styles from "./Sidebar.module.css";

interface SidebarProps {
  projectMeta: SongMetadata;
  selectedInstrument: SelectedInstrument | null;
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
}

export function Sidebar({ projectMeta, selectedInstrument, onSelectInstrument }: SidebarProps) {
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
        {activeTab === "tracks" && <TrackList driverId={projectMeta.driverId} />}
        {activeTab === "instruments" && (
          <InstrumentBrowser
            onSelect={onSelectInstrument}
            selectedInstrument={selectedInstrument}
          />
        )}
      </div>
    </div>
  );
}
