import { useState } from "react";
import type { SelectedInstrument } from "../types/model";
import { FmEditor } from "./FmEditor";
import { PsgEditor } from "./PsgEditor";
import { DacEditor } from "./DacEditor";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  selectedInstrument: SelectedInstrument | null;
}

export function BottomPanel({ selectedInstrument }: BottomPanelProps) {
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className={`${styles.panel} ${collapsed ? styles.collapsed : ""}`}>
      <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
        <span>Instrument Editor</span>
      </div>
      {!collapsed && (
        <div className={styles.editor}>
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
        </div>
      )}
    </div>
  );
}
