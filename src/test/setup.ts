// NOTE: this file must stay under src/ so the jest-dom type augmentation is inside tsconfig's include glob.
import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";

// RTL 16 only auto-registers cleanup when a global afterEach exists; we run with
// globals: false (explicit imports), so register it ourselves to avoid leaking
// mounted DOM between tests.
afterEach(cleanup);

// React only emits its "update not wrapped in act(...)" warning when this global
// is true. RTL sets it itself, but only from a beforeAll/afterAll block that it
// registers just if those names exist as globals -- and we run globals: false, so
// that block never ran and the warning was silenced across the whole suite.
//
// We set the global here rather than flipping `globals: true` in vitest.config.ts:
// that would also inject describe/it/expect into every test file's scope and make
// RTL auto-register its own afterEach(cleanup) on top of the one above (double
// cleanup), to buy one boolean. This line has no blast radius beyond the flag.
//
// React ships no type for the global, so declare it rather than cast: a cast would
// hide a typo in the name, and a typo silently restores the old silence.
declare global {
  // eslint-disable-next-line no-var
  var IS_REACT_ACT_ENVIRONMENT: boolean;
}
globalThis.IS_REACT_ACT_ENVIRONMENT = true;

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
