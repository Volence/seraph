import { useState, useEffect } from "react";
import styles from "./PianoKeys.module.css";

interface PianoKeysProps {
  onNoteOn: (midiNote: number) => void;
  onNoteOff?: () => void;
}

const WHITE_SEMITONES = [0, 2, 4, 5, 7, 9, 11];
const BLACK_SEMITONES = [1, 3, 6, 8, 10];
const BLACK_POSITIONS = [0, 1, 3, 4, 5];

export function PianoKeys({ onNoteOn, onNoteOff }: PianoKeysProps) {
  const [octave, setOctave] = useState(4);
  const [activeKey, setActiveKey] = useState<number | null>(null);
  const [held, setHeld] = useState(false);

  function handleKey(semitone: number, e: React.MouseEvent) {
    let oct = octave;
    if (e.shiftKey) oct += 1;
    if (e.ctrlKey) oct -= 1;
    const midiNote = Math.max(0, Math.min(127, (oct + 1) * 12 + semitone));
    onNoteOn(midiNote);
    setActiveKey(semitone);
    setHeld(true);
  }

  useEffect(() => {
    if (!held) return;
    function handleUp() {
      setHeld(false);
      setActiveKey(null);
      onNoteOff?.();
    }
    window.addEventListener("mouseup", handleUp);
    return () => window.removeEventListener("mouseup", handleUp);
  }, [held, onNoteOff]);

  return (
    <div className={styles.container}>
      <div className={styles.controls}>
        <button className={styles.octBtn} onClick={() => setOctave(Math.max(0, octave - 1))}>-</button>
        <span className={styles.octLabel}>C{octave}</span>
        <button className={styles.octBtn} onClick={() => setOctave(Math.min(8, octave + 1))}>+</button>
      </div>
      <div className={styles.keyboard}>
        {WHITE_SEMITONES.map((semi) => (
          <div
            key={semi}
            className={`${styles.whiteKey} ${activeKey === semi ? styles.activeWhite : ""}`}
            onMouseDown={(e) => handleKey(semi, e)}
          />
        ))}
        {BLACK_SEMITONES.map((semi, i) => (
          <div
            key={semi}
            className={`${styles.blackKey} ${activeKey === semi ? styles.activeBlack : ""}`}
            style={{ left: `${(BLACK_POSITIONS[i] + 0.65) * (100 / 7)}%` }}
            onMouseDown={(e) => handleKey(semi, e)}
          />
        ))}
      </div>
    </div>
  );
}
