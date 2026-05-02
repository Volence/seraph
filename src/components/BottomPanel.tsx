import { useState } from "react";
import type { SelectedInstrument } from "../types/model";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  selectedInstrument: SelectedInstrument | null;
}

export function BottomPanel({ selectedInstrument }: BottomPanelProps) {
  const [collapsed, setCollapsed] = useState(false);

  if (!selectedInstrument) {
    return (
      <div className={styles.panel}>
        <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
          <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
          <span>Instrument Editor</span>
        </div>
        {!collapsed && (
          <div className={styles.empty}>Select an instrument to edit</div>
        )}
      </div>
    );
  }

  return (
    <div className={`${styles.panel} ${collapsed ? styles.collapsed : ""}`}>
      <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
        <span>Instrument Editor</span>
      </div>
      {!collapsed && (
        <div className={styles.editor}>
          <p style={{ color: "var(--text-secondary)", textAlign: "center", marginTop: 40 }}>
            {selectedInstrument.type.toUpperCase()} editor — Tasks 9-11
          </p>
        </div>
      )}
    </div>
  );
}
