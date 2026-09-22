import { describe, it, expect } from "vitest";
import { resolveChannel } from "./resolve-core-version.mjs";

describe("resolveChannel", () => {
  describe("stable", () => {
    it("picks the highest stable- release, no range restriction", () => {
      const tags = ["stable-25.8.0", "stable-25.9.0", "stable-25.9.2", "stable-26.0.0", "beta-26.1-1"];
      expect(resolveChannel(tags, "stable")).toBe("stable-26.0.0");
    });

    it("throws when no stable release exists", () => {
      const tags = ["beta-26.5-4", "hotfix-25.9.3-1"];
      expect(() => resolveChannel(tags, "stable")).toThrow(/no stable release found/);
    });

    it("throws when stable- tags exist but none are valid semver", () => {
      const tags = ["stable-not-a-version", "beta-26.5-1"];
      expect(() => resolveChannel(tags, "stable")).toThrow(/no stable release found/);
    });
  });

  describe("beta", () => {
    it("picks the highest series, then the highest increment within it", () => {
      const tags = ["beta-26.5", "beta-26.5-1", "beta-26.5-2", "beta-26.6", "stable-26.5.1"];
      expect(resolveChannel(tags, "beta")).toBe("beta-26.6");
    });

    it("prefers a higher increment over a bare series of the same value", () => {
      const tags = ["beta-26.5", "beta-26.5-3", "beta-26.5-1"];
      expect(resolveChannel(tags, "beta")).toBe("beta-26.5-3");
    });

    it("throws when no beta release exists", () => {
      const tags = ["stable-26.5.1"];
      expect(() => resolveChannel(tags, "beta")).toThrow(/no beta release found/);
    });
  });

  describe("nightly", () => {
    it("returns the literal rolling tag when it exists", () => {
      const tags = ["stable-26.5.1", "nightly-latest", "beta-26.5-4"];
      expect(resolveChannel(tags, "nightly")).toBe("nightly-latest");
    });

    it("throws when the rolling tag does not exist", () => {
      const tags = ["stable-26.5.1"];
      expect(() => resolveChannel(tags, "nightly")).toThrow(/nightly-latest.*not found/);
    });
  });

  it("throws for an unknown channel", () => {
    expect(() => resolveChannel(["stable-26.5.1"], "hotfix")).toThrow(/unknown channel "hotfix"/);
  });
});
