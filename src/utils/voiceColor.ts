// Deterministic per-voice note color for the piano roll (S4's "sub-voice
// lane" vocabulary renders per-note voices as colored lanes later; this is
// the same mapping applied to note fills today).
//
// The color derives from the instrument UUID alone — stable across sessions
// and machines, no palette registry to persist. Hue comes from an FNV-1a
// hash of the uuid string; saturation/lightness are fixed at values whose
// WORST hue (pure blue) still clears the 3:1 WCAG graphical-object contrast
// ratio against the darkest grid row (#1a1a1a) — see voiceColor.test.ts.

/** 32-bit FNV-1a over the UTF-16 code units of `s`. */
function fnv1a(s: string): number {
  let h = 0x811c9dc5;
  for (let i = 0; i < s.length; i++) {
    h ^= s.charCodeAt(i);
    h = Math.imul(h, 0x01000193);
  }
  return h >>> 0;
}

/** HSL (h in degrees, s/l in 0..1) to "#rrggbb". */
function hslToHex(h: number, s: number, l: number): string {
  const c = (1 - Math.abs(2 * l - 1)) * s;
  const hp = ((h % 360) + 360) % 360 / 60;
  const x = c * (1 - Math.abs((hp % 2) - 1));
  let r = 0, g = 0, b = 0;
  if (hp < 1) [r, g, b] = [c, x, 0];
  else if (hp < 2) [r, g, b] = [x, c, 0];
  else if (hp < 3) [r, g, b] = [0, c, x];
  else if (hp < 4) [r, g, b] = [0, x, c];
  else if (hp < 5) [r, g, b] = [x, 0, c];
  else [r, g, b] = [c, 0, x];
  const m = l - c / 2;
  const to2 = (v: number) =>
    Math.round((v + m) * 255).toString(16).padStart(2, "0");
  return `#${to2(r)}${to2(g)}${to2(b)}`;
}

/**
 * Deterministic color for a per-note voice override, keyed by the
 * instrument's UUID. 6-digit lowercase hex so canvas call sites can append
 * alpha suffixes (`voiceColor(id) + "cc"`), matching how channelColor is
 * used in PianoRollCanvas.
 */
export function voiceColor(instrumentId: string): string {
  const hue = fnv1a(instrumentId) % 360;
  return hslToHex(hue, 0.65, 0.65);
}
