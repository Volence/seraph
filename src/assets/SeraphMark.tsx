// src/assets/SeraphMark.tsx
// Seraph mark from megaforge/design/icons/seraph.svg. Uses currentColor — tint via `color`.
interface SeraphMarkProps {
  size?: number;
  className?: string;
}

export function SeraphMark({ size = 22, className }: SeraphMarkProps) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 96 96"
      role="img"
      aria-label="Seraph"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <title>Seraph</title>
      <circle cx="48" cy="48" r="42" strokeWidth="1.4" />
      <circle cx="48" cy="48" r="38" strokeWidth="1" opacity="0.22" />
      <g strokeWidth="1.4">
        <path d="M48 30 Q26 22 18 40" />
        <path d="M48 30 Q70 22 78 40" />
        <path d="M48 46 Q22 44 14 60" />
        <path d="M48 46 Q74 44 82 60" />
        <path d="M48 62 Q30 66 26 82" />
        <path d="M48 62 Q66 66 70 82" />
      </g>
      <g strokeWidth="1" opacity="0.42">
        <line x1="48" y1="33" x2="30" y2="29" />
        <line x1="48" y1="33" x2="66" y2="29" />
        <line x1="48" y1="48" x2="26" y2="49" />
        <line x1="48" y1="48" x2="70" y2="49" />
      </g>
      <path d="M48 22 Q42 32 48 42 Q54 32 48 22 Z" strokeWidth="1.8" />
      <path d="M38 52 Q48 45 58 52 Q48 59 38 52 Z" strokeWidth="1.4" />
      <circle cx="48" cy="52" r="3.2" fill="currentColor" stroke="none" />
      <circle cx="28" cy="40" r="2.2" strokeWidth="1" opacity="0.42" />
      <circle cx="68" cy="40" r="2.2" strokeWidth="1" opacity="0.42" />
    </svg>
  );
}
