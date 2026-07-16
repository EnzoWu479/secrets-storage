import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it } from "vitest";

import { useVault } from "@/stores/vault";
import SessionsList from "./SessionsList.vue";

const vault = useVault();

beforeEach(async () => {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123");
});

describe("SessionsList", () => {
  it("lista as sessões do cofre com cabeçalho e ações", async () => {
    await vault.createSession({ name: "Trabalho", authMode: "global" });
    await vault.createSession({
      name: "Financeiro",
      authMode: "own",
      ownPassword: "senha-propria-1",
    });
    vault.lockSession(vault.sessions.value[1].id);

    const wrapper = mount(SessionsList);
    await flushPromises();

    expect(wrapper.get("h1").text()).toBe("Sessões");
    expect(wrapper.get(".new-session").text()).toBe("Nova sessão");
    expect(wrapper.findAll(".session-card")).toHaveLength(2);

    const financeiro = wrapper.get('.session-card[data-session="Financeiro"]');
    expect(financeiro.text()).toContain("Senha própria");
    expect(financeiro.text()).toContain("Bloqueada");
    expect(financeiro.text()).toContain("— segredos ocultos");
  });

  it("mostra o estado vazio quando não há sessões", () => {
    const wrapper = mount(SessionsList);

    expect(wrapper.get("[data-empty]").text()).toContain("Nenhuma sessão ainda");
    expect(wrapper.findAll(".session-card")).toHaveLength(0);
  });

  it("filtra as sessões pela busca", async () => {
    await vault.createSession({ name: "Trabalho", authMode: "global" });
    await vault.createSession({ name: "Pessoal", authMode: "global" });

    const wrapper = mount(SessionsList);
    await flushPromises();
    expect(wrapper.findAll(".session-card")).toHaveLength(2);

    await wrapper.get('input[type="search"]').setValue("trab");
    expect(wrapper.findAll(".session-card")).toHaveLength(1);
    expect(wrapper.get(".session-card").text()).toContain("Trabalho");
  });
});
