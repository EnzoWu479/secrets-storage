import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import PasswordStrength from "./PasswordStrength.vue";

describe("PasswordStrength", () => {
  it.each([
    { level: 1 as const, label: "Fraca", color: "bg-danger" },
    { level: 2 as const, label: "Média", color: "bg-warning" },
    { level: 3 as const, label: "Boa", color: "bg-accent" },
    { level: 4 as const, label: "Forte", color: "bg-success" },
  ])("renderiza o nível $level como $label", ({ level, label, color }) => {
    const wrapper = mount(PasswordStrength, { props: { level } });
    const segments = wrapper.findAll("[data-strength-segment]");

    expect(wrapper.get("[data-strength-label]").text()).toBe(label);
    expect(wrapper.attributes("aria-valuenow")).toBe(String(level));
    expect(segments).toHaveLength(4);

    segments.forEach((segment, index) => {
      expect(segment.classes()).toContain(index < level ? color : "bg-divider");
    });
  });
});
