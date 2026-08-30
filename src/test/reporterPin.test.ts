import { describe, it, expect } from "vitest";
// Read the config as text. Importing it for real would pull in
// @vitejs/plugin-react -> esbuild, which throws under the jsdom environment;
// node:fs is unavailable because this project has no @types/node.
import configSource from "../../vitest.config.ts?raw";

/**
 * F45. Vitest 4 selects its reporter as `isAgent ? "agent" : "default"`, and
 * std-env's `isAgent` is true whenever CLAUDECODE or AI_AGENT is set. The
 * "agent" reporter is an alias for MinimalReporter, which hard-codes
 * `silent: "passed-only"` and therefore drops console output from passing
 * tests — so every diagnostic printed by a passing test (React key warnings,
 * deprecation notices, anything a library prints) was invisible to agent
 * sessions while remaining visible to humans. A config-level `silent: false`
 * does NOT override it: the reporter supplies its own `silent`, and the config
 * value is only a `??=` fallback.
 *
 * The fix is an explicit `reporters` pin in vitest.config.ts. This guard fails
 * if that pin is ever removed or pointed back at the silencing reporter.
 */
describe("F45: the test reporter is pinned, not chosen by environment", () => {
  // Loud, not silent: a check that cannot read its subject must fail.
  if (typeof configSource !== "string" || configSource.trim() === "") {
    throw new Error(
      "F45 guard could not read vitest.config.ts as text. This check cannot " +
        "measure and must not be treated as passing.",
    );
  }

  // Strip comments so the explanatory prose above the pin cannot satisfy it.
  const code = configSource
    .replace(/\/\*[\s\S]*?\*\//g, "")
    .replace(/^\s*\/\/.*$/gm, "");
  const pin = /\breporters\s*:\s*\[([^\]]*)\]/.exec(code);

  it("vitest.config.ts pins `reporters` explicitly", () => {
    expect(
      pin,
      "vitest.config.ts must pin `reporters`, or vitest 4 falls back to " +
        '`isAgent ? "agent" : "default"` and agent sessions lose all console ' +
        "output from passing tests (F45).",
    ).not.toBeNull();
    expect(pin?.[1].trim()).not.toBe("");
  });

  it("the pinned reporter is not the console-dropping agent/minimal reporter", () => {
    const names = pin?.[1] ?? "";
    expect(names).not.toMatch(/["']agent["']/);
    expect(names).not.toMatch(/["']minimal["']/);
  });

  it("console output from a passing test is not silenced", () => {
    // Visible canary in captured logs: if this line is absent from a run's
    // output, the reporter is dropping passing-test console output again.
    console.warn("F45 canary: passing-test console output is visible");
    expect(true).toBe(true);
  });
});
