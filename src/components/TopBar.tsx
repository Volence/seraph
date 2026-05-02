import type { SongMetadata } from "../types/model";
import styles from "./TopBar.module.css";

interface TopBarProps {
  projectMeta: SongMetadata | null;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSave: () => void;
  showSaved: boolean;
}

export function TopBar({ projectMeta, onNewProject, onOpenProject, onSave, showSaved }: TopBarProps) {
  return (
    <div className={styles.topBar}>
      <div className={styles.projectInfo}>
        <span className={styles.projectName}>{projectMeta?.name ?? "MegaDAW"}</span>
        {projectMeta && (
          <>
            <span className={styles.separator}>|</span>
            <span className={styles.detail}>{projectMeta.tempo} BPM</span>
            <span className={styles.detail}>
              {projectMeta.timeSignature[0]}/{projectMeta.timeSignature[1]}
            </span>
            <span className={styles.driverBadge}>Flamedriver</span>
          </>
        )}
      </div>
      <div className={styles.actions}>
        <button className={styles.btn} onClick={onNewProject}>New</button>
        <button className={styles.btn} onClick={onOpenProject}>Open</button>
        {projectMeta && (
          <button className={styles.btn} onClick={onSave}>Save</button>
        )}
        {showSaved && <span className={styles.saved}>Saved</span>}
      </div>
      <div className={styles.transport}>
        <button className={styles.transportBtn} disabled title="Play (Phase 4)">&#9654;</button>
        <button className={styles.transportBtn} disabled title="Stop (Phase 4)">&#9632;</button>
        <button className={styles.transportBtn} disabled title="Loop (Phase 4)">&#8635;</button>
      </div>
    </div>
  );
}
