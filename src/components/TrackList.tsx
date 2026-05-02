import { useState, useEffect } from "react";
import type { ChannelLayout } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./TrackList.module.css";

interface TrackListProps {
  driverId: string;
}

export function TrackList({ driverId }: TrackListProps) {
  const [layout, setLayout] = useState<ChannelLayout | null>(null);

  useEffect(() => {
    ipc.getDriverInfo(driverId).then((d) => setLayout(d.layout));
  }, [driverId]);

  if (!layout) return null;

  return (
    <div className={styles.trackList}>
      {layout.fmChannels.map((ch) => (
        <div key={`fm-${ch.index}`} className={styles.track}>
          <span className={`${styles.badge} ${styles.fm}`}>FM</span>
          <span className={styles.name}>{ch.name}</span>
        </div>
      ))}
      {layout.psgChannels.map((ch) => (
        <div key={`psg-${ch.index}`} className={styles.track}>
          <span className={`${styles.badge} ${styles.psg}`}>PSG</span>
          <span className={styles.name}>{ch.name}</span>
        </div>
      ))}
      {layout.dacChannels.map((ch) => (
        <div key={`dac-${ch.index}`} className={styles.track}>
          <span className={`${styles.badge} ${styles.dac}`}>DAC</span>
          <span className={styles.name}>{ch.name}</span>
        </div>
      ))}
    </div>
  );
}
