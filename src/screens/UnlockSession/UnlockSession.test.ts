import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useVault } from "@/stores/vault";
import UnlockSession from "./UnlockSession.vue";

const vault = useVault();

async function setupLockedOwnSession() {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123");
  const own = await vault.createSession({
    name: "Financeiro",
    authMode: "own",
    ownPassword: "senha-propria-1",
    hint: "apelido do banco",
  });
  vault.lockSession(own.id);
  vault.openSession(own.id);
  return own;
}

beforeEach(async () => {
  await setupLockedOwnSession();
});

describe("UnlockSession", () => {
  it("mostra o alvo e revela a dica sob demanda", async () => {
    const wrapper = mount(UnlockSession);
    await flushPromises();

    expect(wrapper.text()).toContain("Desbloquear Financeiro");
    expect(wrapper.find("[data-hint]").exists()).toBe(false);

    await wrapper.get("button.text-accent").trigger("click");
    expect(wrapper.get("[data-hint]").text()).toContain("apelido do banco");
  });

  it("rejeita senha errada e desbloqueia a sessão com a correta", async () => {
    const own = vault.activeSession.value;
    expect(own).not.toBeNull();

    const wrong = mount(UnlockSession);
    await flushPromises();
    await wrong.get("input").setValue("errada");
    await wrong.get("form").trigger("submit");
    await vi.waitFor(() =>
      expect(wrong.find("[data-error]").exists()).toBe(true),
    );
    expect(vault.isSessionUnlocked(own!)).toBe(false);

    const right = mount(UnlockSession);
    await flushPromises();
    await right.get("input").setValue("senha-propria-1");
    await right.get("form").trigger("submit");
    await vi.waitFor(() => expect(vault.isSessionUnlocked(own!)).toBe(true));
  });
});
