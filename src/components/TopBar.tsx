import type { SongMetadata } from "../types/model";
import { TransportControls } from "./TransportControls";
import styles from "./TopBar.module.css";

interface TopBarProps {
  projectMeta: SongMetadata | null;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSave: () => void;
  showSaved: boolean;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  onLoopChange: (enabled: boolean) => void;
}

export function TopBar({
  projectMeta,
  onNewProject,
  onOpenProject,
  onSave,
  showSaved,
  playing,
  loopEnabled,
  onPlayingChange,
  onLoopChange,
}: TopBarProps) {
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
      {projectMeta ? (
        <TransportControls
          projectMeta={projectMeta}
          playing={playing}
          loopEnabled={loopEnabled}
          onPlayingChange={onPlayingChange}
          onLoopChange={onLoopChange}
        />
      ) : (
        <div className={styles.transport}>
          <button className={styles.transportBtn} disabled>&#9654;</button>
          <button className={styles.transportBtn} disabled>&#9632;</button>
          <button className={styles.transportBtn} disabled>&#8635;</button>
        </div>
      )}
    </div>
  );
}
