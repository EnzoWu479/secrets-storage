import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import ConflictResolution from "./ConflictResolution.vue";

describe("ConflictResolution", () => {
  it("monta o modal no AppShell com o banner de expiração", () => {
    const wrapper = mount(ConflictResolution);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.get('[role="dialog"]').attributes("aria-modal")).toBe("true");
    expect(wrapper.get('[role="alert"]').text()).toContain(
      "Conflito pendente em GitHub (Trabalho). Expira em 6 dias…",
    );
  });

  it("compara exatamente dois campos entre Local e Remoto", () => {
    const fields = mount(ConflictResolution).findAll(".conflict-field");

    expect(fields).toHaveLength(2);
    expect(fields.map((field) => field.get("h2").text())).toEqual([
      "Usuário",
      "Senha",
    ]);
    expect(fields.every((field) => field.findAll("[data-side]").length === 2)).toBe(true);
    expect(fields.every((field) => field.text().includes("Manter local"))).toBe(true);
    expect(fields.every((field) => field.text().includes("Manter remoto"))).toBe(true);
    expect(fields.every((field) => field.text().includes("Manter ambos"))).toBe(true);
  });

  it("mostra os metadados da senha e as ações finais", () => {
    const wrapper = mount(ConflictResolution);
    const password = wrapper.get('[data-field="password"]');
    const footerButtons = wrapper
      .findAllComponents(UiButton)
      .slice(-2)
      .map((button) => button.text());

    expect(password.text()).toContain("editado há 1h");
    expect(password.text()).toContain("Notebook");
    expect(password.text()).toContain("há 3h");
    expect(footerButtons).toEqual(["Resolver depois", "Aplicar resolução"]);
  });
});
