import { mount, type VueWrapper } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import AppUpdate from "./AppUpdate.vue";

describe("AppUpdate", () => {
  it("monta as três variações estáticas dentro do AppShell", () => {
    const wrapper = mount(AppUpdate);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.get(".app-shell__lock").text()).toContain("Bloquear app");
    expect(wrapper.findAll("[data-update-state]")).toHaveLength(3);
    expect(wrapper.get('[data-update-state="available"]').text()).toContain("Disponível");
    expect(wrapper.get('[data-update-state="checking"]').text()).toContain("Verificando");
    expect(wrapper.get('[data-update-state="error"]').text()).toContain("Erro");
  });

  it("exibe versão, ações, progresso e erro de assinatura", () => {
    const wrapper = mount(AppUpdate);
    const available = wrapper.get('[data-update-state="available"]');
    const checking = wrapper.get('[data-update-state="checking"]');
    const error = wrapper.get('[data-update-state="error"]');

    expect(available.text()).toContain("v0.2.0");
    expect(available.text()).toContain("Notas da versão");
    expect(
      available.findAllComponents(UiButton).map((button: VueWrapper) => button.text()),
    ).toEqual(["Depois", "Instalar e reiniciar"]);
    expect(checking.getComponent(UiIcon).props("name")).toBe("sync");
    expect(checking.getComponent(UiIcon).classes()).toContain("animate-spin");
    expect(error.classes()).toContain("border-danger");
    expect(error.text()).toContain(
      "Atualização rejeitada: assinatura inválida. Sua versão atual continua funcionando.",
    );
    expect(error.getComponent(UiButton).text()).toBe("Fechar");
  });
});
