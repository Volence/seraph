import styles from "./MainArea.module.css";

interface MainAreaProps {
  projectOpen: boolean;
  onNewProject: () => void;
  onOpenProject: () => void;
}

export function MainArea({ projectOpen, onNewProject, onOpenProject }: MainAreaProps) {
  if (!projectOpen) {
    return (
      <div className={styles.welcome}>
        <h1 className={styles.title}>MegaDAW</h1>
        <p className={styles.subtitle}>Mega Drive Digital Audio Workstation</p>
        <div className={styles.welcomeActions}>
          <button className={styles.welcomeBtn} onClick={onNewProject}>New Project</button>
          <button className={styles.welcomeBtn} onClick={onOpenProject}>Open Project</button>
        </div>
      </div>
    );
  }

  return (
    <div className={styles.placeholder}>
      <p className={styles.placeholderText}>Arrangement View — Phase 4</p>
    </div>
  );
}
