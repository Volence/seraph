// src/components/StatusBar.tsx
import styles from "./StatusBar.module.css";

export function StatusBar() {
  return (
    <div className={styles.statusBar}>
      <span className={styles.aether}>
        <span className={styles.diamond}>◇</span> Aether offline
      </span>
    </div>
  );
}
