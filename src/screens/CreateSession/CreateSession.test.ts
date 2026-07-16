import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useVault } from "@/stores/vault";
import CreateSession from "./CreateSession.vue";

const vault = useVault();

beforeEach(async () => {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123");
});

describe("CreateSession", () => {
  it("esconde os campos de senha própria até ativar o toggle", async () => {
    const wrapper = mount(CreateSession);

    expect(wrapper.find(".password-fields").exists()).toBe(false);
    expect(wrapper.text()).toContain(
      "Esta sessão usará a senha mestra global e abrirá junto com o app.",
    );

    await wrapper.get(".own-password-toggle").trigger("click");
    expect(wrapper.get(".password-fields").text()).toContain("Confirmar senha");
  });

  it("cria uma sessão global ao enviar o formulário", async () => {
    const wrapper = mount(CreateSession);

    await wrapper.get("input").setValue("Trabalho");
    await wrapper.get("form").trigger("submit");

    await vi.waitFor(() => expect(vault.sessions.value).toHaveLength(1));
    expect(vault.sessions.value[0].name).toBe("Trabalho");
    expect(vault.sessions.value[0].authMode).toBe("global");
  });

  it("bloqueia o envio quando o nome já existe", async () => {
    await vault.createSession({ name: "Trabalho", authMode: "global" });
    const wrapper = mount(CreateSession);

    await wrapper.get("input").setValue("trabalho");
    expect(wrapper.find("[data-form-error]").exists()).toBe(false);
    expect(wrapper.text()).toContain("Já existe uma sessão com esse nome.");
    expect(
      wrapper.get('button[type="submit"]').attributes("disabled"),
    ).toBeDefined();
  });
});
