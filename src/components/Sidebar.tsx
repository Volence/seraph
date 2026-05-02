import type { SongMetadata, SelectedInstrument } from "../types/model";
import { InstrumentBrowser } from "./InstrumentBrowser";
import styles from "./Sidebar.module.css";

interface SidebarProps {
  projectMeta: SongMetadata;
  selectedInstrument: SelectedInstrument | null;
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
}

export function Sidebar({ selectedInstrument, onSelectInstrument }: SidebarProps) {
  return (
    <div className={styles.sidebar}>
      <div className={styles.content}>
        <InstrumentBrowser
          onSelect={onSelectInstrument}
          selectedInstrument={selectedInstrument}
        />
      </div>
    </div>
  );
}
