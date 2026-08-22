// NOTE: this file must stay under src/ so the jest-dom type augmentation is inside tsconfig's include glob.
import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// RTL 16 only auto-registers cleanup when a global afterEach exists; we run with
// globals: false (explicit imports), so register it ourselves to avoid leaking
// mounted DOM between tests.
afterEach(cleanup);

// jsdom has no ResizeObserver; components that observe layout (e.g.
// SpectrumAnalyzer) just get no resize callbacks under test.
if (typeof globalThis.ResizeObserver === "undefined") {
  class ResizeObserverStub {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  globalThis.ResizeObserver = ResizeObserverStub as unknown as typeof ResizeObserver;
}
