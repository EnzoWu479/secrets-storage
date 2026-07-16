import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it } from "vitest";

import { useVault } from "@/stores/vault";
import SecretsList from "./SecretsList.vue";

const vault = useVault();

beforeEach(async () => {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123");
  const session = await vault.createSession({ name: "Trabalho", authMode: "global" });
  vault.openSession(session.id);
});

describe("SecretsList", () => {
  it("mostra o cofre da sessão ativa com estado vazio", async () => {
    const wrapper = mount(SecretsList);
    await flushPromises();

    expect(wrapper.get("h1").text()).toBe("Trabalho");
    expect(wrapper.get("[data-empty]").text()).toContain("Nenhum segredo ainda");
    expect(wrapper.text()).toContain("Adicionar");
    expect(wrapper.text()).toContain("Bloquear sessão");
  });

  it("lista os segredos existentes da sessão", async () => {
    const session = vault.activeSession.value!;
    session.secrets.push({
      id: "sec-1",
      type: "password",
      name: "GitHub",
      fields: {},
      createdAt: "2026-01-01",
      updatedAt: "2026-01-01",
    });

    const wrapper = mount(SecretsList);
    await flushPromises();

    expect(wrapper.find("[data-empty]").exists()).toBe(false);
    expect(wrapper.findAll(".secret-row")).toHaveLength(1);
    expect(wrapper.get(".secret-row").text()).toContain("GitHub");
  });
});
