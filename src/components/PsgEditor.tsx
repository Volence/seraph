import { useState, useEffect, useCallback } from "react";
import type { PsgInstrument, NoiseMode } from "../types/model";
import * as ipc from "../api/ipc";
import { librarySaveFromProject } from "../api/library";
import { scheduleReloadSequence } from "../utils/liveReload";
import { StepGraphEditor } from "../widgets/StepGraphEditor";
import styles from "./PsgEditor.module.css";

interface PsgEditorProps {
  instrumentId: string;
  /** Called after a successful save-to-library (refreshes the panel). */
  onSavedToLibrary?: () => void;
}

export function PsgEditor({ instrumentId, onSavedToLibrary }: PsgEditorProps) {
  const [instrument, setInstrument] = useState<PsgInstrument | null>(null);

  const load = useCallback(async () => {
    const list = await ipc.listPsgInstruments();
    setInstrument(list.find((i) => i.id === instrumentId) ?? null);
  }, [instrumentId]);

  useEffect(() => { load(); }, [load]);

  if (!instrument) return null;

  async function update(updates: Partial<PsgInstrument>) {
    const updated = { ...instrument!, ...updates };
    setInstrument(updated);
    try {
      await ipc.updatePsgInstrument(instrumentId, updated);
      // Same seam as FmEditor (audit F3): the envelope is embedded per NoteOn,
      // so without a reload an edit is only heard after stop/play. A reload
      // now swaps the envelope under a sounding note without re-attacking it.
      // Coalesced: the step-graph editor drags.
      scheduleReloadSequence();
    } catch {
      load();
    }
  }

  function getNoiseType(): "off" | "periodic" | "white" {
    if (!instrument!.noiseMode) return "off";
    if ("Periodic" in instrument!.noiseMode) return "periodic";
    return "white";
  }

  function getNoisePeriod(): number {
    if (!instrument!.noiseMode) return 0;
    if ("Periodic" in instrument!.noiseMode) return instrument!.noiseMode.Periodic;
    return instrument!.noiseMode.White;
  }

  function setNoiseType(type: "off" | "periodic" | "white") {
    if (type === "off") {
      update({ noiseMode: null });
    } else {
      const period = getNoisePeriod() || 100;
      const mode: NoiseMode = type === "periodic" ? { Periodic: period } : { White: period };
      update({ noiseMode: mode });
    }
  }

  function setNoisePeriod(period: number) {
    const type = getNoiseType();
    if (type === "periodic") update({ noiseMode: { Periodic: period } });
    else if (type === "white") update({ noiseMode: { White: period } });
  }

  return (
    <div className={styles.psgEditor}>
      <div className={styles.graphSection}>
        <div className={styles.graphHeader}>
          <span className={styles.sectionTitle}>Volume Envelope</span>
          <div className={styles.graphActions}>
            <button
              className={styles.smallBtn}
              onClick={() => update({ volumeSequence: [...instrument.volumeSequence, 0] })}
            >
              + Tick
            </button>
            <button
              className={styles.smallBtn}
              onClick={() => {
                if (instrument.volumeSequence.length > 1) {
                  update({ volumeSequence: instrument.volumeSequence.slice(0, -1) });
                }
              }}
            >
              - Tick
            </button>
          </div>
        </div>
        <StepGraphEditor
          values={instrument.volumeSequence}
          max={15}
          onChange={(values) => update({ volumeSequence: values })}
          loopPoint={instrument.loopPoint}
          onLoopChange={(point) => update({ loopPoint: point })}
        />
      </div>

      <div className={styles.settingsSection}>
        <div className={styles.settingGroup}>
          <span className={styles.settingLabel}>Loop</span>
          <label className={styles.toggle}>
            <input
              type="checkbox"
              checked={instrument.loopPoint !== null}
              onChange={(e) => update({ loopPoint: e.target.checked ? 0 : null })}
            />
            {instrument.loopPoint !== null ? `Point: ${instrument.loopPoint}` : "Off"}
          </label>
        </div>

        <div className={styles.settingGroup}>
          <span className={styles.settingLabel}>Noise Mode</span>
          <div className={styles.noiseButtons}>
            {(["off", "periodic", "white"] as const).map((type) => (
              <button
                key={type}
                className={`${styles.noiseBtn} ${getNoiseType() === type ? styles.activeNoise : ""}`}
                onClick={() => setNoiseType(type)}
              >
                {type.charAt(0).toUpperCase() + type.slice(1)}
              </button>
            ))}
          </div>
          {getNoiseType() !== "off" && (
            <div className={styles.periodRow}>
              <span className={styles.periodLabel}>Period</span>
              <input
                className={styles.periodInput}
                type="number"
                min={0}
                max={1023}
                value={getNoisePeriod()}
                onChange={(e) => setNoisePeriod(Number(e.target.value))}
              />
            </div>
          )}
        </div>

        <div className={styles.settingGroup}>
          <button
            className={styles.previewBtn}
            onClick={() => ipc.previewPsgInstrument(instrumentId, 60)}
          >
            Preview
          </button>
          <button
            className={styles.saveToLibrary}
            title="Save this instrument to My Library"
            onClick={async () => {
              try {
                await librarySaveFromProject("psg", instrumentId, null, []);
                onSavedToLibrary?.();
              } catch (e) {
                console.error("Save to library failed:", e);
              }
            }}
          >
            Save to library
          </button>
        </div>
      </div>
    </div>
  );
}
