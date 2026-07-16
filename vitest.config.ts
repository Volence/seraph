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
  },
});
