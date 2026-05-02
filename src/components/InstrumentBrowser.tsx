import { useState, useEffect, useCallback } from "react";
import type {
  FmInstrument, PsgInstrument, DacInstrument,
  SelectedInstrument,
} from "../types/model";
import { DEFAULT_FM_MODULATOR, DEFAULT_FM_CARRIER } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./InstrumentBrowser.module.css";

interface InstrumentBrowserProps {
  onSelect: (inst: SelectedInstrument | null) => void;
  selectedInstrument: SelectedInstrument | null;
}

export function InstrumentBrowser({ onSelect, selectedInstrument }: InstrumentBrowserProps) {
  const [fmList, setFmList] = useState<FmInstrument[]>([]);
  const [psgList, setPsgList] = useState<PsgInstrument[]>([]);
  const [dacList, setDacList] = useState<DacInstrument[]>([]);
  const [fmExpanded, setFmExpanded] = useState(true);
  const [psgExpanded, setPsgExpanded] = useState(true);
  const [dacExpanded, setDacExpanded] = useState(true);
  const [contextMenu, setContextMenu] = useState<{
    x: number; y: number; type: "fm" | "psg" | "dac"; id: string;
  } | null>(null);
  const [renaming, setRenaming] = useState<{
    type: "fm" | "psg" | "dac"; id: string; name: string;
  } | null>(null);

  const refreshAll = useCallback(async () => {
    const [fm, psg, dac] = await Promise.all([
      ipc.listFmInstruments(),
      ipc.listPsgInstruments(),
      ipc.listDacInstruments(),
    ]);
    setFmList(fm);
    setPsgList(psg);
    setDacList(dac);
  }, []);

  useEffect(() => { refreshAll(); }, [refreshAll]);

  useEffect(() => {
    if (!contextMenu) return;
    function close() { setContextMenu(null); }
    window.addEventListener("click", close);
    return () => window.removeEventListener("click", close);
  }, [contextMenu]);

  async function addFm() {
    const inst: FmInstrument = {
      id: "00000000-0000-0000-0000-000000000000",
      name: "New FM Patch",
      algorithm: 0,
      feedback: 0,
      operators: [DEFAULT_FM_MODULATOR, DEFAULT_FM_MODULATOR, DEFAULT_FM_MODULATOR, DEFAULT_FM_CARRIER],
      metadata: { category: "", author: "", tags: [] },
    };
    const id = await ipc.addFmInstrument(inst);
    await refreshAll();
    onSelect({ type: "fm", id });
  }

  async function addPsg() {
    const inst: PsgInstrument = {
      id: "00000000-0000-0000-0000-000000000000",
      name: "New PSG Envelope",
      volumeSequence: [15, 14, 13, 12, 10, 8, 6, 4, 2, 0],
      loopPoint: null,
      noiseMode: null,
      metadata: { category: "", author: "", tags: [] },
    };
    const id = await ipc.addPsgInstrument(inst);
    await refreshAll();
    onSelect({ type: "psg", id });
  }

  async function addDac() {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      filters: [
        { name: "Audio", extensions: ["wav"] },
        { name: "Raw PCM", extensions: ["pcm", "raw"] },
      ],
      title: "Import DAC Sample",
    });
    if (!selected) return;
    const path = selected as string;
    const ext = path.split(".").pop()?.toLowerCase();
    let id: string;
    if (ext === "wav") {
      id = await ipc.importDacWav(path, 16000);
    } else {
      id = await ipc.importDacRaw(path, 16000);
    }
    await refreshAll();
    onSelect({ type: "dac", id });
  }

  function handleContextMenu(e: React.MouseEvent, type: "fm" | "psg" | "dac", id: string) {
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY, type, id });
  }

  function handleRename(type: "fm" | "psg" | "dac", id: string) {
    setContextMenu(null);
    const currentName =
      type === "fm" ? fmList.find((i) => i.id === id)?.name :
      type === "psg" ? psgList.find((i) => i.id === id)?.name :
      dacList.find((i) => i.id === id)?.name;
    setRenaming({ type, id, name: currentName ?? "" });
  }

  async function commitRename() {
    if (!renaming || !renaming.name.trim()) { setRenaming(null); return; }
    const { type, id, name } = renaming;
    if (type === "fm") {
      const inst = fmList.find((i) => i.id === id);
      if (inst) await ipc.updateFmInstrument(id, { ...inst, name });
    } else if (type === "psg") {
      const inst = psgList.find((i) => i.id === id);
      if (inst) await ipc.updatePsgInstrument(id, { ...inst, name });
    } else {
      const inst = dacList.find((i) => i.id === id);
      if (inst) await ipc.updateDacInstrument(id, { ...inst, name });
    }
    setRenaming(null);
    await refreshAll();
  }

  async function handleDuplicate(type: "fm" | "psg" | "dac", id: string) {
    setContextMenu(null);
    if (type === "fm") {
      const inst = fmList.find((i) => i.id === id);
      if (inst) {
        const newId = await ipc.addFmInstrument({
          ...inst,
          id: "00000000-0000-0000-0000-000000000000",
          name: `${inst.name} (Copy)`,
        });
        await refreshAll();
        onSelect({ type: "fm", id: newId });
      }
    } else if (type === "psg") {
      const inst = psgList.find((i) => i.id === id);
      if (inst) {
        const newId = await ipc.addPsgInstrument({
          ...inst,
          id: "00000000-0000-0000-0000-000000000000",
          name: `${inst.name} (Copy)`,
        });
        await refreshAll();
        onSelect({ type: "psg", id: newId });
      }
    }
  }

  async function handleDelete(type: "fm" | "psg" | "dac", id: string) {
    setContextMenu(null);
    if (type === "fm") await ipc.deleteFmInstrument(id);
    else if (type === "psg") await ipc.deletePsgInstrument(id);
    else await ipc.deleteDacInstrument(id);
    if (selectedInstrument?.id === id) onSelect(null);
    await refreshAll();
  }

  function isSelected(type: string, id: string) {
    return selectedInstrument?.type === type && selectedInstrument?.id === id;
  }

  return (
    <div className={styles.browser}>
      <div className={styles.section}>
        <div className={styles.sectionHeader} onClick={() => setFmExpanded(!fmExpanded)}>
          <span>{fmExpanded ? "▼" : "▶"}</span>
          <span className={styles.sectionTitle}>FM</span>
          <span className={styles.count}>{fmList.length}</span>
          <button className={styles.addBtn} onClick={(e) => { e.stopPropagation(); addFm(); }}>+</button>
        </div>
        {fmExpanded && fmList.map((inst) => (
          <div
            key={inst.id}
            className={`${styles.item} ${isSelected("fm", inst.id) ? styles.selected : ""}`}
            onClick={() => onSelect({ type: "fm", id: inst.id })}
            onContextMenu={(e) => handleContextMenu(e, "fm", inst.id)}
          >
            <span className={`${styles.dot} ${styles.fmDot}`} />
            <span className={styles.itemName}>{inst.name}</span>
          </div>
        ))}
      </div>

      <div className={styles.section}>
        <div className={styles.sectionHeader} onClick={() => setPsgExpanded(!psgExpanded)}>
          <span>{psgExpanded ? "▼" : "▶"}</span>
          <span className={styles.sectionTitle}>PSG</span>
          <span className={styles.count}>{psgList.length}</span>
          <button className={styles.addBtn} onClick={(e) => { e.stopPropagation(); addPsg(); }}>+</button>
        </div>
        {psgExpanded && psgList.map((inst) => (
          <div
            key={inst.id}
            className={`${styles.item} ${isSelected("psg", inst.id) ? styles.selected : ""}`}
            onClick={() => onSelect({ type: "psg", id: inst.id })}
            onContextMenu={(e) => handleContextMenu(e, "psg", inst.id)}
          >
            <span className={`${styles.dot} ${styles.psgDot}`} />
            <span className={styles.itemName}>{inst.name}</span>
          </div>
        ))}
      </div>

      <div className={styles.section}>
        <div className={styles.sectionHeader} onClick={() => setDacExpanded(!dacExpanded)}>
          <span>{dacExpanded ? "▼" : "▶"}</span>
          <span className={styles.sectionTitle}>DAC</span>
          <span className={styles.count}>{dacList.length}</span>
          <button className={styles.addBtn} onClick={(e) => { e.stopPropagation(); addDac(); }}>+</button>
        </div>
        {dacExpanded && dacList.map((inst) => (
          <div
            key={inst.id}
            className={`${styles.item} ${isSelected("dac", inst.id) ? styles.selected : ""}`}
            onClick={() => onSelect({ type: "dac", id: inst.id })}
            onContextMenu={(e) => handleContextMenu(e, "dac", inst.id)}
          >
            <span className={`${styles.dot} ${styles.dacDot}`} />
            <span className={styles.itemName}>{inst.name}</span>
          </div>
        ))}
      </div>

      {contextMenu && (
        <div className={styles.contextMenu} style={{ left: contextMenu.x, top: contextMenu.y }}>
          <button className={styles.menuItem} onClick={() => handleRename(contextMenu.type, contextMenu.id)}>Rename</button>
          {contextMenu.type !== "dac" && (
            <button className={styles.menuItem} onClick={() => handleDuplicate(contextMenu.type, contextMenu.id)}>Duplicate</button>
          )}
          <button className={`${styles.menuItem} ${styles.danger}`} onClick={() => handleDelete(contextMenu.type, contextMenu.id)}>Delete</button>
        </div>
      )}

      {renaming && (
        <div className={styles.renameOverlay} onClick={() => setRenaming(null)}>
          <div className={styles.renameDialog} onClick={(e) => e.stopPropagation()}>
            <h3 className={styles.renameTitle}>Rename Instrument</h3>
            <input
              className={styles.renameInput}
              value={renaming.name}
              onChange={(e) => setRenaming({ ...renaming, name: e.target.value })}
              onKeyDown={(e) => e.key === "Enter" && commitRename()}
              autoFocus
            />
            <div className={styles.renameButtons}>
              <button className={styles.renameCancelBtn} onClick={() => setRenaming(null)}>Cancel</button>
              <button className={styles.renameOkBtn} onClick={commitRename}>Rename</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
