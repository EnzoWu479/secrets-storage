import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it } from "vitest";

import { useVault } from "@/stores/vault";
import AppShell from "./AppShell.vue";

const vault = useVault();

beforeEach(async () => {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123");
});

describe("AppShell", () => {
  it("monta sidebar, rodapé e o conteúdo do slot", () => {
    const wrapper = mount(AppShell, {
      slots: { default: '<section data-testid="content">Conteudo</section>' },
    });

    expect(wrapper.get(".app-shell").classes()).toEqual(
      expect.arrayContaining(["flex", "min-h-screen"]),
    );
    expect(wrapper.get("aside").classes()).toContain("w-60");
    expect(wrapper.get("[data-testid='content']").text()).toBe("Conteudo");
    expect(wrapper.get(".app-shell__lock").text()).toContain("Bloquear app");
    expect(wrapper.get("footer").text()).toContain("Sync");
    expect(wrapper.get("footer").text()).toContain("Configurações");
    expect(wrapper.get("footer").text()).toContain("Sobre");
  });

  it("lista as sessões do cofre e destaca a ativa", async () => {
    const trabalho = await vault.createSession({ name: "Trabalho", authMode: "global" });
    await vault.createSession({ name: "Pessoal", authMode: "global" });
    vault.openSession(trabalho.id);

    const wrapper = mount(AppShell);
    const sessions = wrapper.findAll(".app-shell__session");

    expect(sessions).toHaveLength(2);
    expect(wrapper.get('[data-session="Trabalho"]').classes()).toContain("bg-elevated");
    expect(wrapper.get('[data-session="Pessoal"]').classes()).not.toContain(
      "bg-elevated",
    );
    expect(wrapper.find("a").exists()).toBe(false);
  });
});
