import { describe, it, expect } from "vitest";
import { canvasOps } from "./canvasStub";

/**
 * F46: the jsdom test environment supplies a 2D canvas context.
 *
 * These tests are about THE TEST ENVIRONMENT, not about rendering. They assert
 * that a context exists and that calls made to it are observable. None of them
 * asserts, or could assert, that anything looks correct on screen -- see the
 * header of canvasStub.ts. Do not add a test here whose name implies otherwise.
 *
 * The point of pinning this: without the stub, HTMLCanvasElement#getContext
 * returns null and every draw function in src/ aborts on its `if (!ctx) return`
 * line. That is both the 1237-line log flood and a silent hole in coverage. If
 * someone removes the stub, these fail loudly instead of the hole reopening
 * quietly.
 */
describe("F46: the jsdom test environment supplies a 2D canvas context", () => {
  it("getContext('2d') returns a context instead of null", () => {
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d");
    // Without the stub this is null, and jsdom prints its "Not implemented"
    // line to its own virtual console on the way.
    expect(ctx).not.toBeNull();
    expect(typeof ctx?.fillRect).toBe("function");
  });

  it("returns the same context object for repeated calls on one canvas", () => {
    // Browser semantics. Components re-read the context on every redraw.
    const canvas = document.createElement("canvas");
    const first = canvas.getContext("2d");
    // Assert non-null first: without it this passes vacuously when both calls
    // return null, which is exactly the broken state it is meant to catch.
    expect(first).not.toBeNull();
    expect(canvas.getContext("2d")).toBe(first);
  });

  it("gives different canvases different contexts", () => {
    const a = document.createElement("canvas");
    const b = document.createElement("canvas");
    expect(a.getContext("2d")).not.toBe(b.getContext("2d"));
  });

  it("records drawing calls and property assignments in order", () => {
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d")!;
    ctx.fillStyle = "#abcdef";
    ctx.fillRect(1, 2, 3, 4);

    // This proves the calls were ISSUED with these arguments. It proves
    // nothing about pixels: no rectangle was painted anywhere.
    expect(canvasOps(canvas)).toEqual([
      { op: "set:fillStyle", args: ["#abcdef"] },
      { op: "fillRect", args: [1, 2, 3, 4] },
    ]);
  });

  it("reads back context state that was assigned, as a real context does", () => {
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d")!;
    ctx.lineWidth = 3;
    ctx.font = "11px Inter";
    expect(ctx.lineWidth).toBe(3);
    expect(ctx.font).toBe("11px Inter");
    // setLineDash/getLineDash round-trip: some draw code reads it back.
    ctx.setLineDash([4, 2]);
    expect(ctx.getLineDash()).toEqual([4, 2]);
  });

  it("throws on an unknown context method, exactly as a browser would", () => {
    // The stub is an explicit allowlist, not a permissive Proxy. A Proxy that
    // answered every property with a no-op would swallow a typo like this and
    // let it reach a real screen.
    const canvas = document.createElement("canvas");
    const ctx = canvas.getContext("2d") as unknown as Record<
      string,
      (...args: unknown[]) => unknown
    >;
    expect(ctx.filRect).toBeUndefined();
    expect(() => ctx.filRect(0, 0, 1, 1)).toThrow(TypeError);
  });
});
