import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import PasswordStrength from "@/components/PasswordStrength/PasswordStrength.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import SecretForm from "./SecretForm.vue";

describe("SecretForm", () => {
  it("monta o painel de criação dentro do AppShell", () => {
    const wrapper = mount(SecretForm);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.get('[role="dialog"]').attributes("aria-modal")).toBe("true");
    expect(wrapper.get("h1").text()).toBe("Criar segredo");
  });

  it("exibe os cinco tipos com Senha selecionado", () => {
    const types = mount(SecretForm).findAll("[data-secret-type]");

    expect(types.map((type) => type.text())).toEqual([
      "Senha",
      "API key",
      "Token",
      "Nota",
      "SSH",
    ]);
    expect(types[0].attributes("aria-pressed")).toBe("true");
    expect(types.slice(1).every((type) => type.attributes("aria-pressed") === "false")).toBe(true);
  });

  it("apresenta os campos, força da senha e ações estáticas", () => {
    const wrapper = mount(SecretForm);
    const buttons = wrapper.findAllComponents(UiButton);

    expect(wrapper.get('input[name="name"]').attributes("value")).toBe("GitHub");
    expect(wrapper.get('input[name="username"]').attributes("value")).toBe(
      "usuario@exemplo.com",
    );
    expect(wrapper.findComponent(UiInput).props()).toMatchObject({
      label: "Senha",
      type: "password",
    });
    expect(wrapper.get('input[type="password"]').attributes("value")).toBe(
      "MockPassword2026!",
    );
    expect(wrapper.findComponent(PasswordStrength).props("level")).toBe(4);
    expect(wrapper.find('input[name="url"]').exists()).toBe(true);
    expect(wrapper.find('textarea[name="notes"]').exists()).toBe(true);
    expect(buttons.map((button) => button.text())).toEqual(["Cancelar", "Salvar"]);
  });
});
