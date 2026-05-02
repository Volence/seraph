import { useState, useEffect } from "react";
import type { ChannelLayout, ChannelAssignment } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./AddTrackDialog.module.css";

interface AddTrackDialogProps {
  driverId: string;
  onClose: () => void;
  onCreated: () => void;
}

export function AddTrackDialog({ driverId, onClose, onCreated }: AddTrackDialogProps) {
  const [layout, setLayout] = useState<ChannelLayout | null>(null);
  const [name, setName] = useState("");
  const [channelKey, setChannelKey] = useState("fm_0");

  useEffect(() => {
    ipc.getDriverInfo(driverId).then((d) => {
      setLayout(d.layout);
    });
  }, [driverId]);

  function parseChannel(key: string): ChannelAssignment {
    if (key === "psg_noise") return "PsgNoise";
    const [type_, idx] = key.split("_");
    const n = parseInt(idx);
    if (type_ === "fm") return { Fm: n };
    if (type_ === "psg") return { Psg: n };
    return { Dac: n };
  }

  function suggestName(key: string): string {
    const ch = parseChannel(key);
    if (ch === "PsgNoise") return "PSG Noise - Untitled";
    if (typeof ch === "object" && "Fm" in ch) return `FM${ch.Fm + 1} - Untitled`;
    if (typeof ch === "object" && "Psg" in ch) return `PSG${ch.Psg + 1} - Untitled`;
    return "DAC - Untitled";
  }

  async function handleCreate() {
    const trackName = name.trim() || suggestName(channelKey);
    const channel = parseChannel(channelKey);
    await ipc.addTrack(trackName, channel, null);
    onCreated();
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <h3 className={styles.title}>Add Track</h3>
        <label className={styles.label}>
          Name
          <input
            className={styles.input}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={suggestName(channelKey)}
            autoFocus
          />
        </label>
        <label className={styles.label}>
          Channel
          <select
            className={styles.select}
            value={channelKey}
            onChange={(e) => setChannelKey(e.target.value)}
          >
            {layout && (
              <>
                <optgroup label="FM">
                  {layout.fmChannels.map((ch) => (
                    <option key={`fm_${ch.index}`} value={`fm_${ch.index}`}>{ch.name}</option>
                  ))}
                </optgroup>
                <optgroup label="PSG">
                  {layout.psgChannels.map((ch) => (
                    <option
                      key={ch.isNoise ? "psg_noise" : `psg_${ch.index}`}
                      value={ch.isNoise ? "psg_noise" : `psg_${ch.index}`}
                    >
                      {ch.name}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="DAC">
                  {layout.dacChannels.map((ch) => (
                    <option key={`dac_${ch.index}`} value={`dac_${ch.index}`}>{ch.name}</option>
                  ))}
                </optgroup>
              </>
            )}
          </select>
        </label>
        <div className={styles.buttons}>
          <button className={styles.cancelBtn} onClick={onClose}>Cancel</button>
          <button className={styles.createBtn} onClick={handleCreate}>Create</button>
        </div>
      </div>
    </div>
  );
}
