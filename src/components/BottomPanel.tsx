import { useState } from "react";
import type { SelectedInstrument, SelectedRegion } from "../types/model";
import { FmEditor } from "./FmEditor";
import { PsgEditor } from "./PsgEditor";
import { DacEditor } from "./DacEditor";
import { PianoRoll } from "./PianoRoll";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  selectedInstrument: SelectedInstrument | null;
  selectedRegion: SelectedRegion | null;
  onCloseRegion: () => void;
}

export function BottomPanel({ selectedInstrument, selectedRegion, onCloseRegion }: BottomPanelProps) {
  const [collapsed, setCollapsed] = useState(false);

  const showPianoRoll = selectedRegion !== null;
  const headerText = showPianoRoll ? "Piano Roll" : "Instrument Editor";

  return (
    <div className={`${styles.panel} ${collapsed ? styles.collapsed : ""}`}>
      <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
        <span>{headerText}</span>
      </div>
      {!collapsed && (
        <div className={styles.editor}>
          {showPianoRoll ? (
            <PianoRoll region={selectedRegion} onClose={onCloseRegion} />
          ) : (
            <>
              {!selectedInstrument && (
                <div className={styles.empty}>Select an instrument to edit</div>
              )}
              {selectedInstrument?.type === "fm" && (
                <FmEditor instrumentId={selectedInstrument.id} />
              )}
              {selectedInstrument?.type === "psg" && (
                <PsgEditor instrumentId={selectedInstrument.id} />
              )}
              {selectedInstrument?.type === "dac" && (
                <DacEditor instrumentId={selectedInstrument.id} />
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
