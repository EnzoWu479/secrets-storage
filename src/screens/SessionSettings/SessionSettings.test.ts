import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import UiToggle from "@/components/UiToggle/UiToggle.vue";
import SessionSettings from "./SessionSettings.vue";

describe("SessionSettings", () => {
  it("monta as quatro seções de configuração dentro do AppShell", () => {
    const wrapper = mount(SessionSettings);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.get('[data-settings-section="general"]').text()).toContain("Geral");
    expect(wrapper.get('[data-settings-section="authentication"]').text()).toContain(
      "Autenticação",
    );
    expect(wrapper.get('[data-settings-section="access"]').text()).toContain("Acesso");
    expect(wrapper.get('[data-settings-section="danger"]').text()).toContain("Zona de perigo");
  });

  it("exibe os valores e ações estáticos especificados", () => {
    const wrapper = mount(SessionSettings);

    expect(wrapper.text()).toContain("Trabalho");
    expect(wrapper.get('[data-inactivity]').text()).toContain("15 min");
    expect(wrapper.get('[data-clipboard]').text()).toContain("5 min");
    expect(wrapper.text()).toContain("Usa a senha mestra global");
    expect(wrapper.text()).toContain("alterar exige a senha atual apropriada");
    expect(wrapper.findAllComponents(UiInput)).toHaveLength(3);
    expect(wrapper.getComponent(UiToggle).props("checked")).toBe(false);

    const buttons = wrapper.findAllComponents(UiButton).map((button) => button.text());
    expect(buttons).toEqual(expect.arrayContaining(["Definir senha própria", "Excluir sessão"]));

    const danger = wrapper.get('[data-settings-section="danger"]');
    expect(danger.classes()).toContain("border-danger");
    expect(danger.text()).toContain("exige confirmação e senha");
  });
});
