import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import LoginGoogle from "./LoginGoogle.vue";

describe("LoginGoogle", () => {
  it("monta a tela estática com todo o conteúdo de autenticação", () => {
    const wrapper = mount(LoginGoogle);

    expect(wrapper.get("h1").text()).toBe("Secrets Storage");
    expect(wrapper.text()).toContain("Cofre local-first e zero-knowledge");
    expect(wrapper.text()).toContain("Continuar com Google");
    expect(wrapper.text()).toContain(
      "Usamos sua conta apenas para sincronizar o cofre já cifrado",
    );
    expect(wrapper.get("footer").text()).toBe(
      "v0.1.7 · Open source · Telemetria desativada",
    );
  });

  it("reutiliza os ícones de marca e Google dentro do botão largo", () => {
    const wrapper = mount(LoginGoogle);
    const icons = wrapper.findAllComponents(UiIcon);
    const button = wrapper.getComponent(UiButton);

    expect(icons.map((icon) => icon.props("name"))).toEqual(["lock", "google"]);
    expect(button.classes()).toContain("w-full");
    expect(button.text()).toBe("Continuar com Google");
  });

  it("centraliza uma única coluna compacta sobre o fundo do app", () => {
    const wrapper = mount(LoginGoogle);

    expect(wrapper.get("main").classes()).toEqual(
      expect.arrayContaining(["bg-app", "items-center", "justify-center"]),
    );
    expect(wrapper.get("section").classes()).toContain("max-w-[380px]");
    expect(wrapper.find("aside").exists()).toBe(false);
  });
});
