// NOTE: this file must stay under src/ so the jest-dom type augmentation is inside tsconfig's include glob.
import "@testing-library/jest-dom/vitest";
import { afterEach, afterAll } from "vitest";
import { cleanup } from "@testing-library/react";

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

// ...but switching the warning on is not enough for it to be *read*. vitest 4
// picks its reporter with `isAgent ? "agent" : "default"`, and std-env's isAgent
// is true whenever AI_AGENT / CLAUDECODE is set. The agent reporter runs with
// silent: "passed-only", which drops console output from passing tests outright
// -- a config-level `silent: false` does not override it. So on a green run the
// act() warning is visible to a human and invisible to every automated reader,
// which is precisely the population most likely to add an unsynchronised update.
// This parcel measured "0 warnings" that way before noticing; the real count
// was 6.
//
// So do not trust the reporter. Re-emit to process.stderr, which vitest does not
// intercept, and fail the test that caused it -- failures are printed by every
// reporter. A warning nobody sees is the same as no warning at all.
// This repo has no @types/node, so reach process.stderr through a local type
// rather than adding a dependency or declaring `process` globally.
function writeStderr(line: string): void {
  const proc = (globalThis as unknown as {
    process?: { stderr?: { write?: (s: string) => void } };
  }).process;
  proc?.stderr?.write?.(line);
}

const ACT_WARNING = "was not wrapped in act(...)";
let actWarnings: string[] = [];
const passThroughConsoleError = console.error.bind(console);
console.error = (...args: unknown[]) => {
  const template = typeof args[0] === "string" ? args[0] : "";
  if (template.includes(ACT_WARNING)) {
    // React formats this one as "An update to %s ...", component name in args[1].
    const component = typeof args[1] === "string" ? args[1] : "an unknown component";
    actWarnings.push(component);
    writeStderr(`act() warning: an update to ${component} was not wrapped in act(...)\n`);
    return;
  }
  passThroughConsoleError(...args);
};

function drainActWarnings(): string[] {
  const seen = actWarnings;
  actWarnings = [];
  return seen;
}

// RTL 16 only auto-registers cleanup when a global afterEach exists; we run with
// globals: false (explicit imports), so we unmount here to avoid leaking mounted
// DOM between tests.
//
// Cleanup and the act() check share one hook, with cleanup in a finally, on
// purpose. As two hooks this deadlocked in an ugly way: vitest runs afterEach in
// reverse registration order, so the check ran *before* cleanup and its throw
// skipped the unmount entirely -- one warning then leaked a mounted tree into
// every later test in the file ("Found multiple elements..."). Draining after
// cleanup also means updates triggered by the unmount itself are counted.
afterEach(() => {
  try {
    cleanup();
  } finally {
    reportActWarnings();
  }
});

function reportActWarnings(): void {
  const seen = drainActWarnings();
  if (seen.length === 0) return;
  throw new Error(
    `React reported ${seen.length} state update(s) outside act(...): ${seen.join(", ")}.\n` +
      "This means the test finished while the component was still updating -- usually " +
      "an in-flight promise or timer from a mount effect. Wait for the real precondition " +
      "(findBy*/waitFor on what the update actually produces) rather than wrapping the " +
      "symptom in act()."
  );
}

// A warning landing after a file's last test would miss every afterEach.
afterAll(() => {
  const seen = drainActWarnings();
  if (seen.length > 0) {
    throw new Error(
      `React reported ${seen.length} state update(s) outside act(...) after the last test ` +
        `in this file: ${seen.join(", ")}.`
    );
  }
});

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
