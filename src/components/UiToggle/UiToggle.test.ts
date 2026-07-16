import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiToggle from "./UiToggle.vue";

describe("UiToggle", () => {
  it("monta desligado com as dimensoes visuais do switch", () => {
    const wrapper = mount(UiToggle);
    const track = wrapper.get(".ui-toggle__track");

    expect(track.classes()).toContain("h-[22px]");
    expect(track.classes()).toContain("w-10");
    expect(track.classes()).toContain("bg-divider");
    expect(wrapper.get(".ui-toggle__knob").classes()).toContain("bg-white");
  });

  it("renderiza o estado ligado pelas classes do trilho e do botao", () => {
    const wrapper = mount(UiToggle, { props: { checked: true } });

    expect(wrapper.get(".ui-toggle__track").classes()).toContain("bg-accent");
    expect(wrapper.get(".ui-toggle__knob").classes()).toContain(
      "translate-x-[18px]",
    );
  });
});
