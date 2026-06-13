import { describe, expect, test } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

describe("vite config", () => {
  test("uses relative asset paths for Tauri static frontendDist loading", () => {
    const configText = readFileSync(
      resolve(__dirname, "../vite.config.ts"),
      "utf8",
    );

    expect(configText).toContain('base: "./"');
  });
});
