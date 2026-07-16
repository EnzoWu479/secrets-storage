import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiCard from "./UiCard.vue";

describe("UiCard", () => {
  it("monta o conteudo principal com os tokens visuais do card", () => {
    const wrapper = mount(UiCard, {
      slots: { default: "Conteudo do card" },
    });

    expect(wrapper.text()).toBe("Conteudo do card");
    expect(wrapper.classes()).toEqual(
      expect.arrayContaining([
        "ui-card",
        "bg-surface",
        "rounded-card",
        "p-5",
        "border-divider",
      ]),
    );
    expect(wrapper.find("header").exists()).toBe(false);
    expect(wrapper.find("footer").exists()).toBe(false);
  });

  it("renderiza header e footer somente quando seus slots sao fornecidos", () => {
    const wrapper = mount(UiCard, {
      slots: {
        header: "Cabecalho",
        default: "Conteudo",
        footer: "Rodape",
      },
    });

    expect(wrapper.get("header").text()).toBe("Cabecalho");
    expect(wrapper.get("footer").text()).toBe("Rodape");
  });
});
