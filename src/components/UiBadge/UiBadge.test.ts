import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiBadge from "./UiBadge.vue";

describe("UiBadge", () => {
  it("monta o conteúdo do slot como uma pill compacta", () => {
    const badge = mount(UiBadge, {
      slots: { default: "Bloqueada" },
    }).get("span");

    expect(badge.text()).toBe("Bloqueada");
    expect(badge.classes()).toEqual(
      expect.arrayContaining([
        "rounded-pill",
        "text-[11px]",
        "font-medium",
        "leading-[14px]",
      ]),
    );
  });

  it.each([
    ["neutral", ["bg-divider", "text-secondary"]],
    ["accent", ["bg-[var(--color-accent-soft)]", "text-accent"]],
    ["success", ["bg-success/10", "text-success"]],
    ["warning", ["bg-warning/10", "text-warning"]],
    ["danger", ["bg-danger/10", "text-danger"]],
  ] as const)("aplica o tom %s com tokens semânticos", (tone, classes) => {
    const badge = mount(UiBadge, { props: { tone } }).get("span");

    expect(badge.classes()).toEqual(expect.arrayContaining([...classes]));
  });
});
