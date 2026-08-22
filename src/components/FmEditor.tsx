import { useState, useEffect, useCallback } from "react";
import type { FmInstrument, FmOperator } from "../types/model";
import { isCarrier } from "../types/model";
import * as ipc from "../api/ipc";
import { librarySaveFromProject } from "../api/library";
import { scheduleReloadSequence } from "../utils/liveReload";
import { Knob } from "../widgets/Knob";
import { AlgorithmDiagram } from "../widgets/AlgorithmDiagram";
import { EnvelopeDisplay } from "../widgets/EnvelopeDisplay";
import { PianoKeys } from "../widgets/PianoKeys";
import styles from "./FmEditor.module.css";

interface FmEditorProps {
  instrumentId: string;
  /** Called after a successful save-to-library (refreshes the panel). */
  onSavedToLibrary?: () => void;
}

export function FmEditor({ instrumentId, onSavedToLibrary }: FmEditorProps) {
  const [instrument, setInstrument] = useState<FmInstrument | null>(null);

  const load = useCallback(async () => {
    const list = await ipc.listFmInstruments();
    setInstrument(list.find((i) => i.id === instrumentId) ?? null);
  }, [instrumentId]);

  useEffect(() => { load(); }, [load]);

  if (!instrument) return null;

  async function updateInstrument(updates: Partial<FmInstrument>) {
    const updated = { ...instrument!, ...updates };
    setInstrument(updated);
    try {
      await ipc.updateFmInstrument(instrumentId, updated);
      // The running snapshot embeds patch bytes per NoteOn, so an edit is
      // inaudible until the sequence is rebuilt. Reloading is now safe
      // mid-playback (it carries sounding notes across and reprograms them
      // in place), which is what makes tweak-while-looping work — audit F3.
      // Coalesced: the knobs fire one change per mousemove.
      scheduleReloadSequence();
    } catch {
      load();
    }
  }

  async function updateOp(opIndex: number, updates: Partial<FmOperator>) {
    const ops = [...instrument!.operators] as [FmOperator, FmOperator, FmOperator, FmOperator];
    ops[opIndex] = { ...ops[opIndex], ...updates };
    await updateInstrument({ operators: ops });
  }

  return (
    <div className={styles.fmEditor}>
      <div className={styles.leftSection}>
        <AlgorithmDiagram
          algorithm={instrument.algorithm}
          onSelect={(a) => updateInstrument({ algorithm: a })}
        />
        <div className={styles.fbKnob}>
          <Knob min={0} max={7} value={instrument.feedback} onChange={(v) => updateInstrument({ feedback: v })} label="FB" size={44} />
        </div>
      </div>

      {instrument.operators.map((op, i) => (
        <div key={i} className={`${styles.opPanel} ${isCarrier(instrument.algorithm, i) ? styles.carrier : ""}`}>
          <div className={styles.opHeader}>
            <span>Op {i + 1}</span>
            {isCarrier(instrument.algorithm, i) && <span className={styles.carrierBadge}>C</span>}
          </div>
          <EnvelopeDisplay
            attackRate={op.attackRate}
            d1r={op.d1r}
            d2r={op.d2r}
            sustainLevel={op.sustainLevel}
            releaseRate={op.releaseRate}
          />
          <div className={styles.knobGrid}>
            <Knob min={0} max={7} value={op.detune} onChange={(v) => updateOp(i, { detune: v })} label="DT" size={36} />
            <Knob min={0} max={15} value={op.multiple} onChange={(v) => updateOp(i, { multiple: v })} label="MUL" size={36} />
            <Knob min={0} max={3} value={op.rateScale} onChange={(v) => updateOp(i, { rateScale: v })} label="RS" size={36} />
            <Knob min={0} max={31} value={op.attackRate} onChange={(v) => updateOp(i, { attackRate: v })} label="AR" size={36} />
            <Knob min={0} max={31} value={op.d1r} onChange={(v) => updateOp(i, { d1r: v })} label="D1R" size={36} />
            <Knob min={0} max={31} value={op.d2r} onChange={(v) => updateOp(i, { d2r: v })} label="D2R" size={36} />
            <Knob min={0} max={15} value={op.sustainLevel} onChange={(v) => updateOp(i, { sustainLevel: v })} label="SL" size={36} />
            <Knob min={0} max={15} value={op.releaseRate} onChange={(v) => updateOp(i, { releaseRate: v })} label="RR" size={36} />
          </div>
          <div className={styles.tlRow}>
            <span className={styles.tlLabel}>TL</span>
            <input
              type="range"
              className={styles.tlSlider}
              min={0}
              max={127}
              value={instrument.operators[i].totalLevel}
              onChange={(e) => updateOp(i, { totalLevel: parseInt(e.target.value) })}
            />
            <span className={styles.tlValue}>{op.totalLevel}</span>
          </div>
          <label className={styles.amToggle}>
            <input type="checkbox" checked={op.ampMod} onChange={(e) => updateOp(i, { ampMod: e.target.checked })} />
            AM
          </label>
        </div>
      ))}

      <div className={styles.previewSection}>
        <PianoKeys
          onNoteOn={(note) => ipc.previewFmInstrument(instrumentId, note)}
          onNoteOff={() => ipc.stopFmPreview()}
        />
        <button
          className={styles.saveToLibrary}
          title="Save this instrument to My Library"
          onClick={async () => {
            try {
              await librarySaveFromProject("fm", instrumentId, null, []);
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
  );
}
