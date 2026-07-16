import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiToggle from "@/components/UiToggle/UiToggle.vue";
import GeneralSettings from "./GeneralSettings.vue";

describe("GeneralSettings", () => {
  it("monta as quatro seções de configurações no AppShell", () => {
    const wrapper = mount(GeneralSettings);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.get("h1").text()).toBe("Configurações");
    expect(wrapper.findAll("[data-settings-section]").map((section) => section.get("h2").text())).toEqual([
      "Segurança",
      "Aparência",
      "Privacidade",
      "Sobre",
    ]);
  });

  it("apresenta ações de segurança e o aviso persistente", () => {
    const wrapper = mount(GeneralSettings);
    const buttons = wrapper.findAllComponents(UiButton).map((button) => button.text());

    expect(wrapper.text()).toContain("Trocar senha mestra global");
    expect(wrapper.text()).toContain("Exige a senha atual");
    expect(buttons).toEqual(["Trocar senha mestra global", "Bloquear app agora"]);
    expect(wrapper.get('[role="alert"]').text()).toContain(
      "O v1 não possui recuperação de acesso.",
    );
  });

  it("mostra tema, privacidade e informações do aplicativo sem lógica", () => {
    const wrapper = mount(GeneralSettings);

    expect(wrapper.findComponent(UiToggle).props("checked")).toBe(true);
    expect(wrapper.text()).toContain("Claro");
    expect(wrapper.text()).toContain("Escuro");
    expect(wrapper.text()).toContain("Telemetria: Desativada");
    expect(wrapper.text()).toContain(
      "Secrets Storage v0.1.7 · Open source · Tauri + Vue",
    );
    expect(wrapper.findAll('[role="link"]').map((link) => link.text())).toEqual([
      "Repositório",
      "Licença",
    ]);
  });
});
