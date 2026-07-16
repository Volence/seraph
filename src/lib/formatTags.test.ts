import { describe, it, expect } from "vitest";
import { formatTags } from "./formatTags";

describe("formatTags", () => {
  it("joins and lowercases tags", () => {
    expect(formatTags(["Lead", "BRIGHT"])).toBe("lead, bright");
  });
  it("returns empty string for no tags", () => {
    expect(formatTags([])).toBe("");
  });
});
