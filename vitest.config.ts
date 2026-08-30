import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// NOTE: this config does NOT inherit vite.config.ts — any shared vite options
// (aliases etc.) must be duplicated here or the configs merged.
export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    // Pin the reporter. Vitest 4 otherwise picks `isAgent ? "agent" : "default"`,
    // and std-env's `isAgent` is true whenever CLAUDECODE or AI_AGENT is set — so
    // every agent session silently got a different reporter than every human.
    // "agent" is an alias for MinimalReporter, which hard-codes silent:"passed-only";
    // a config-level `silent: false` does NOT override it, because the reporter
    // passes its own `silent` and the config value is only a `??=` fallback.
    // Net effect: console.log/console.warn from PASSING tests — React key warnings,
    // deprecation notices, anything a library prints — were invisible to agents.
    // "default" also prints one line per test file, so a fully-skipped file can be
    // told apart from a passing one. See src/test/reporterPin.test.ts.
    reporters: ["default"],
  },
});
