import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import Connecting from "./Connecting.vue";

describe("Connecting", () => {
  it("monta as variacoes de carregamento e erro ao mesmo tempo", () => {
    const wrapper = mount(Connecting);
    const states = wrapper.findAll(".connecting-state");

    expect(wrapper.get("main").classes()).toEqual(
      expect.arrayContaining(["min-h-screen", "items-center", "justify-center"]),
    );
    expect(states).toHaveLength(2);
    expect(wrapper.get(".connecting-spinner").classes()).toContain(
      "border-t-accent",
    );
    expect(wrapper.text()).toContain("Abrindo o Google…");
    expect(wrapper.text()).toContain(
      "Conclua a autorização na janela do navegador…",
    );
    expect(wrapper.text()).toContain("Autorização não concluída");
  });

  it("exibe somente ações visuais para cancelar ou tentar novamente", () => {
    const wrapper = mount(Connecting);
    const buttons = wrapper.findAll("button").map((button) => button.text());

    expect(buttons).toEqual(["Cancelar", "Tentar novamente"]);
    expect(wrapper.find("a").exists()).toBe(false);
  });
});
