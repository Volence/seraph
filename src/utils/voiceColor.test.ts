import { describe, it, expect } from "vitest";
import { voiceColor } from "./voiceColor";

// The piano-roll grid rows the color must stand out against (see
// PianoRollCanvas draw(): "#1a1a1a" black-key rows, "#1e1e1e" white-key rows).
const GRID_ROW_COLORS = ["#1a1a1a", "#1e1e1e"];

function rgb(hex: string): [number, number, number] {
  expect(hex).toMatch(/^#[0-9a-f]{6}$/);
  return [
    parseInt(hex.slice(1, 3), 16),
    parseInt(hex.slice(3, 5), 16),
    parseInt(hex.slice(5, 7), 16),
  ];
}

/** WCAG relative luminance. */
function luminance(hex: string): number {
  const lin = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  const [r, g, b] = rgb(hex);
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

/** WCAG contrast ratio between two colors. */
function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const SAMPLE_UUIDS = [
  "25bded43-0fd4-446c-ac40-2e1ff13781d1",
  "8f14e45f-ceea-467f-ab6e-9b2b6c3e4a01",
  "00000000-0000-0000-0000-000000000000",
  "ffffffff-ffff-ffff-ffff-ffffffffffff",
  "c56a4180-65aa-42ec-a945-5fd21dec0538",
];

describe("voiceColor", () => {
  it("is deterministic: the same uuid always maps to the same color", () => {
    for (const id of SAMPLE_UUIDS) {
      expect(voiceColor(id)).toBe(voiceColor(id));
    }
  });

  it("emits 6-digit lowercase hex so callers can append canvas alpha suffixes", () => {
    // PianoRollCanvas builds translucent fills as `color + "cc"` — the
    // format must support that.
    for (const id of SAMPLE_UUIDS) {
      expect(voiceColor(id)).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  it("keeps decent contrast against the dark note grid for every uuid", () => {
    // 3:1 is the WCAG threshold for graphical objects — notes must be
    // clearly visible on both grid row shades.
    for (const id of SAMPLE_UUIDS) {
      for (const row of GRID_ROW_COLORS) {
        expect(contrast(voiceColor(id), row)).toBeGreaterThanOrEqual(3);
      }
    }
  });

  it("distinguishes distinct uuids (hue spread over the sample set)", () => {
    const colors = new Set(SAMPLE_UUIDS.map(voiceColor));
    expect(colors.size).toBe(SAMPLE_UUIDS.length);
  });
});
