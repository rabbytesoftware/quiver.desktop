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

    it("throws a clear error for a malformed beta tag instead of silently sorting it in", () => {
      const tags = ["beta-26.5-4", "beta-abc", "beta-26.4"];
      expect(() => resolveChannel(tags, "beta")).toThrow(/malformed beta tag "beta-abc"/);
    });

    it("compares series numerically, not lexicographically", () => {
      const tags = ["beta-26.9", "beta-26.10"];
      expect(resolveChannel(tags, "beta")).toBe("beta-26.10");
    });

    it("compares counts numerically, not lexicographically", () => {
      const tags = ["beta-26.5-9", "beta-26.5-10"];
      expect(resolveChannel(tags, "beta")).toBe("beta-26.5-10");
    });

    it("reads a dated series, ranked by its date among calendar versions", () => {
      const tags = ["nightly-latest", "beta-2026-09-27", "stable-26.5.1", "beta-26.5-4", "beta-26.5"];
      expect(resolveChannel(tags, "beta")).toBe("beta-2026-09-27");
    });

    it("ranks a dated rebuild above its series and a later calendar version above a date", () => {
      expect(resolveChannel(["beta-2026-09-27", "beta-2026-09-27-1", "beta-26.5-4"], "beta")).toBe("beta-2026-09-27-1");
      expect(resolveChannel(["beta-2026-09-27-3", "beta-26.11"], "beta")).toBe("beta-26.11");
      expect(resolveChannel(["beta-2026-09-27-3", "beta-2026-10-01"], "beta")).toBe("beta-2026-10-01");
    });

    it("throws for a date that is not a real one", () => {
      expect(() => resolveChannel(["beta-26.5", "beta-2026-13-01"], "beta")).toThrow(/malformed beta tag "beta-2026-13-01"/);
    });
  });

  describe("stable, dated series", () => {
    it("ranks a date as YY.MM.DD.patch, the order quiver.core's own channel ranking uses", () => {
      const tags = ["stable-26.5.1", "stable-2026-09-27", "stable-2026-09-27.1", "stable-2026-10-01"];
      expect(resolveChannel(tags, "stable")).toBe("stable-2026-10-01");
      expect(resolveChannel([...tags, "stable-26.11"], "stable")).toBe("stable-26.11");
      expect(resolveChannel(["stable-2026-09-27", "stable-2026-09-27.1", "stable-26.5.1"], "stable")).toBe(
        "stable-2026-09-27.1"
      );
    });

    it("ignores a stable- tag whose date is not a real one", () => {
      expect(resolveChannel(["stable-26.5", "stable-2026-02-40"], "stable")).toBe("stable-26.5");
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
