import { describe, it, expect, beforeEach } from "vitest";
import {
  RECENT_LOCATIONS_KEY,
  MAX_RECENT_LOCATIONS,
  getRecentLocations,
  mostRecentLocation,
  rememberLocation,
  parentDirectory,
} from "./recentLocations";

describe("recentLocations", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns an empty list and the fallback when nothing is stored", () => {
    expect(getRecentLocations()).toEqual([]);
    expect(mostRecentLocation()).toBe("");
    expect(mostRecentLocation("/home/me/projects")).toBe("/home/me/projects");
  });

  it("remembers a location and surfaces it as most recent", () => {
    rememberLocation("/home/me/songs");
    expect(getRecentLocations()).toEqual(["/home/me/songs"]);
    expect(mostRecentLocation("/fallback")).toBe("/home/me/songs");
  });

  it("orders most-recent first", () => {
    rememberLocation("/a");
    rememberLocation("/b");
    rememberLocation("/c");
    expect(getRecentLocations()).toEqual(["/c", "/b", "/a"]);
  });

  it("dedups re-used locations, promoting them to the front", () => {
    rememberLocation("/a");
    rememberLocation("/b");
    rememberLocation("/a");
    expect(getRecentLocations()).toEqual(["/a", "/b"]);
  });

  it("treats trailing-slash variants as the same location", () => {
    rememberLocation("/a/b");
    rememberLocation("/a/b/");
    expect(getRecentLocations()).toEqual(["/a/b"]);
  });

  it("ignores empty and whitespace-only paths", () => {
    rememberLocation("");
    rememberLocation("   ");
    expect(getRecentLocations()).toEqual([]);
  });

  it("caps the list at MAX_RECENT_LOCATIONS, dropping the oldest", () => {
    for (let i = 0; i < MAX_RECENT_LOCATIONS + 3; i++) {
      rememberLocation(`/dir${i}`);
    }
    const list = getRecentLocations();
    expect(list).toHaveLength(MAX_RECENT_LOCATIONS);
    expect(list[0]).toBe(`/dir${MAX_RECENT_LOCATIONS + 2}`);
    expect(list[list.length - 1]).toBe("/dir3");
  });

  it("survives corrupt stored JSON by starting fresh", () => {
    localStorage.setItem(RECENT_LOCATIONS_KEY, "{not json[");
    expect(getRecentLocations()).toEqual([]);
    rememberLocation("/recovered");
    expect(getRecentLocations()).toEqual(["/recovered"]);
  });

  it("filters non-string entries out of stored data", () => {
    localStorage.setItem(RECENT_LOCATIONS_KEY, JSON.stringify(["/ok", 42, null, "/also-ok"]));
    expect(getRecentLocations()).toEqual(["/ok", "/also-ok"]);
  });

  it("persists via localStorage under the documented key", () => {
    rememberLocation("/somewhere");
    expect(JSON.parse(localStorage.getItem(RECENT_LOCATIONS_KEY)!)).toEqual(["/somewhere"]);
  });

  describe("parentDirectory", () => {
    it("returns the parent of a project directory", () => {
      expect(parentDirectory("/home/me/songs/MyTrack")).toBe("/home/me/songs");
    });

    it("ignores a trailing slash", () => {
      expect(parentDirectory("/home/me/songs/MyTrack/")).toBe("/home/me/songs");
    });

    it("returns empty string when there is no parent", () => {
      expect(parentDirectory("MyTrack")).toBe("");
      expect(parentDirectory("/")).toBe("");
    });
  });
});
