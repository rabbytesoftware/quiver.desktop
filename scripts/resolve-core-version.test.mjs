import { describe, it, expect } from "vitest";
import { resolveTag } from "./resolve-core-version.mjs";

describe("resolveTag", () => {
  it("picks the highest stable- tag satisfying the constraint", () => {
    const tags = ["stable-25.8.0", "stable-25.9.0", "stable-25.9.2", "stable-26.0.0", "beta-26.1-1"];
    expect(resolveTag(tags, "^25.9")).toBe("stable-25.9.2");
  });

  it("ignores non-stable channels entirely", () => {
    const tags = ["beta-26.5-4", "hotfix-25.9.3-1"];
    expect(() => resolveTag(tags, "^25.9")).toThrow(/no stable release/);
  });

  it("throws when nothing satisfies the constraint", () => {
    const tags = ["stable-24.0.0"];
    expect(() => resolveTag(tags, "^25.9")).toThrow(/no stable release/);
  });
});
