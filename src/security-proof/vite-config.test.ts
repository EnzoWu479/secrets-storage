import { describe, expect, it } from "vitest";
import configSource from "../../e2e/security-proof/vite.config.ts?raw";

describe("security-proof Vite configuration", () => {
  it("uses a dedicated proof entrypoint instead of the normal application source", () => {
    expect(configSource).toContain("../../src/security-proof");
    expect(configSource).toContain("../../src/security-proof/index.html");
    expect(configSource).not.toContain("../../src/main.ts");
  });
});
