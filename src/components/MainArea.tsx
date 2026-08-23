import type { SongMetadata, SelectedRegion, SelectedInstrument } from "../types/model";
import type { PreviewLoopRange } from "../utils/previewLoop";
import { ArrangementView } from "./ArrangementView";
import styles from "./MainArea.module.css";

interface MainAreaProps {
  projectOpen: boolean;
  projectMeta: SongMetadata | null;
  /** Open project's directory; keys the remembered view state (F15). */
  projectPath: string | null;
  playing: boolean;
  /** Seek cursor position; owned by App so transport shortcuts and the
   *  stop-sync (G29) keep it truthful. */
  seekTick: number;
  onSeek: (tick: number) => void;
  /** Preview loop (transport-scratch, App-owned); drawn on the ruler. */
  previewLoop: PreviewLoopRange | null;
  loopEnabled: boolean;
  onPreviewLoopSet: (startTick: number, endTick: number) => void;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSelectRegions: (regions: SelectedRegion[]) => void;
  selectedRegions: SelectedRegion[];
  onSelectInstrument: (inst: SelectedInstrument | null) => void;
  selectedInstrument: SelectedInstrument | null;
}

export function MainArea({
  projectOpen,
  projectMeta,
  projectPath,
  playing,
  seekTick,
  onSeek,
  previewLoop,
  loopEnabled,
  onPreviewLoopSet,
  onNewProject,
  onOpenProject,
  onSelectRegions,
  selectedRegions,
  onSelectInstrument,
  selectedInstrument,
}: MainAreaProps) {
  if (!projectOpen || !projectMeta) {
    return (
      <div className={styles.welcome}>
        <h1 className={styles.title}>Seraph</h1>
        <p className={styles.subtitle}>Mega Drive Digital Audio Workstation</p>
        <div className={styles.welcomeActions}>
          <button className={styles.welcomeBtn} onClick={onNewProject}>New Project</button>
          <button className={styles.welcomeBtn} onClick={onOpenProject}>Open Project</button>
        </div>
      </div>
    );
  }

  return (
    <ArrangementView
      // A different project gets a fresh arrangement: zoom, scroll, snap and
      // collapsed channel groups are seeded from ITS remembered view at mount.
      key={projectPath ?? "no-project"}
      projectMeta={projectMeta}
      projectPath={projectPath}
      playing={playing}
      seekTick={seekTick}
      onSeek={onSeek}
      previewLoop={previewLoop}
      loopEnabled={loopEnabled}
      onPreviewLoopSet={onPreviewLoopSet}
      onSelectRegions={onSelectRegions}
      selectedRegions={selectedRegions}
      onSelectInstrument={onSelectInstrument}
      selectedInstrument={selectedInstrument}
    />
  );
}
