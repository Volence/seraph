import { useState, useEffect, useCallback } from "react";
import type { DacInstrument } from "../types/model";
import * as ipc from "../api/ipc";
import { WaveformViewer } from "../widgets/WaveformViewer";
import styles from "./DacEditor.module.css";

interface DacEditorProps {
  instrumentId: string;
}

const SAMPLE_RATES = [8000, 11025, 16000, 22050, 32000];

export function DacEditor({ instrumentId }: DacEditorProps) {
  const [instrument, setInstrument] = useState<DacInstrument | null>(null);
  const [pcmData, setPcmData] = useState<number[]>([]);

  const load = useCallback(async () => {
    const list = await ipc.listDacInstruments();
    const inst = list.find((i) => i.id === instrumentId);
    setInstrument(inst ?? null);
    if (inst) {
      try {
        const data = await ipc.getDacPcmData(instrumentId);
        setPcmData(data);
      } catch {
        setPcmData([]);
      }
    }
  }, [instrumentId]);

  useEffect(() => { load(); }, [load]);

  if (!instrument) return null;

  async function handleNameChange(name: string) {
    const updated = { ...instrument!, name };
    setInstrument(updated);
    try {
      await ipc.updateDacInstrument(instrumentId, updated);
    } catch {
      load();
    }
  }

  async function handleRateChange(newRate: number) {
    try {
      await ipc.reconvertDac(instrumentId, newRate);
      await load();
    } catch (e) {
      console.error("Reconvert failed:", e);
    }
  }

  return (
    <div className={styles.dacEditor}>
      <div className={styles.waveformSection}>
        <WaveformViewer data={pcmData} width={500} height={120} />
        <span className={styles.sampleCount}>{pcmData.length.toLocaleString()} samples</span>
      </div>

      <div className={styles.settingsSection}>
        <div className={styles.field}>
          <span className={styles.fieldLabel}>Name</span>
          <input
            className={styles.nameInput}
            value={instrument.name}
            onChange={(e) => handleNameChange(e.target.value)}
            onBlur={() => ipc.updateDacInstrument(instrumentId, instrument)}
          />
        </div>

        <div className={styles.field}>
          <span className={styles.fieldLabel}>Source</span>
          <span className={styles.sourceInfo}>
            {instrument.sourceIsRaw ? "Raw PCM" : "WAV"} — {instrument.originalFile}
          </span>
        </div>

        <div className={styles.field}>
          <span className={styles.fieldLabel}>Sample Rate</span>
          <select
            className={styles.rateSelect}
            value={instrument.targetSampleRate}
            onChange={(e) => handleRateChange(Number(e.target.value))}
            disabled={instrument.sourceIsRaw}
          >
            {SAMPLE_RATES.map((rate) => (
              <option key={rate} value={rate}>{rate} Hz</option>
            ))}
          </select>
          {instrument.sourceIsRaw && (
            <span className={styles.hint}>Cannot reconvert raw imports</span>
          )}
        </div>

        <button
          className={styles.previewBtn}
          onClick={() => ipc.previewDac(instrumentId)}
        >
          Preview
        </button>
      </div>
    </div>
  );
}
