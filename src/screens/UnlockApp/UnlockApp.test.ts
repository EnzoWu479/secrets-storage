import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useVault } from "@/stores/vault";
import UnlockApp from "./UnlockApp.vue";

const vault = useVault();

beforeEach(async () => {
  vault._resetForTests();
  await vault.createGlobalPassword("senha-global-123", "minha dica");
  vault.lockApp();
});

describe("UnlockApp", () => {
  it("monta a tela de desbloqueio do app", () => {
    const wrapper = mount(UnlockApp);

    expect(wrapper.get("h1").text()).toBe("Desbloquear Secrets Storage");
    expect(wrapper.text()).toContain(
      "Desbloqueia todas as sessões que usam a senha global.",
    );
  });

  it("revela a dica somente após a ação explícita", async () => {
    const wrapper = mount(UnlockApp);

    expect(wrapper.find("[data-hint]").exists()).toBe(false);
    await wrapper.get("button.text-accent").trigger("click");
    expect(wrapper.get("[data-hint]").text()).toContain("minha dica");
  });

  it("rejeita senha incorreta e mantém o app bloqueado", async () => {
    const wrapper = mount(UnlockApp);

    await wrapper.get("#global-password").setValue("errada");
    await wrapper.get("form").trigger("submit");

    await vi.waitFor(() =>
      expect(wrapper.find("[data-error]").exists()).toBe(true),
    );
    expect(wrapper.get("[data-error]").text()).toContain("Senha incorreta");
    expect(vault.state.appUnlocked).toBe(false);
  });

  it("desbloqueia com a senha correta", async () => {
    const wrapper = mount(UnlockApp);

    await wrapper.get("#global-password").setValue("senha-global-123");
    await wrapper.get("form").trigger("submit");

    await vi.waitFor(() => expect(vault.state.appUnlocked).toBe(true));
  });
});
