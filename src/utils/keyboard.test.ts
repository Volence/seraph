import { describe, it, expect } from "vitest";
import { isEditableTarget } from "./keyboard";

describe("isEditableTarget", () => {
  it("is true for text-entry form controls", () => {
    expect(isEditableTarget(document.createElement("input"))).toBe(true);
    expect(isEditableTarget(document.createElement("textarea"))).toBe(true);
    expect(isEditableTarget(document.createElement("select"))).toBe(true);
  });

  it("is true for contentEditable elements", () => {
    const div = document.createElement("div");
    // jsdom does not compute isContentEditable from the attribute; stub the
    // browser behavior on the instance.
    Object.defineProperty(div, "isContentEditable", { value: true });
    expect(isEditableTarget(div)).toBe(true);
  });

  it("is false for ordinary elements, window and null", () => {
    expect(isEditableTarget(document.createElement("div"))).toBe(false);
    expect(isEditableTarget(document.createElement("button"))).toBe(false);
    expect(isEditableTarget(document.body)).toBe(false);
    expect(isEditableTarget(window)).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});
