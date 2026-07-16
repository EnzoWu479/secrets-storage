import { beforeEach, describe, expect, it } from "vitest";

import { useVault } from "./vault";

const vault = useVault();

beforeEach(() => {
  vault._resetForTests();
});

describe("vault store", () => {
  it("cria a senha global e deixa o app desbloqueado", async () => {
    expect(vault.hasGlobalPassword.value).toBe(false);
    await vault.createGlobalPassword("senha-global-123", "minha dica");

    expect(vault.hasGlobalPassword.value).toBe(true);
    expect(vault.state.appUnlocked).toBe(true);
    expect(vault.state.globalHint).toBe("minha dica");
  });

  it("verifica a senha global no desbloqueio", async () => {
    await vault.createGlobalPassword("senha-global-123");
    vault.lockApp();
    expect(vault.state.appUnlocked).toBe(false);

    expect(await vault.unlockApp("errada")).toBe(false);
    expect(vault.state.appUnlocked).toBe(false);

    expect(await vault.unlockApp("senha-global-123")).toBe(true);
    expect(vault.state.appUnlocked).toBe(true);
  });

  it("rejeita nomes de sessão duplicados sem diferenciar maiúsculas", async () => {
    await vault.createGlobalPassword("senha-global-123");
    await vault.createSession({ name: "Trabalho", authMode: "global" });

    expect(vault.nameTaken("trabalho")).toBe(true);
    await expect(
      vault.createSession({ name: "TRABALHO", authMode: "global" }),
    ).rejects.toThrow();
  });

  it("sessão global segue o app; sessão própria exige a própria senha", async () => {
    await vault.createGlobalPassword("senha-global-123");
    const global = await vault.createSession({ name: "Trabalho", authMode: "global" });
    const own = await vault.createSession({
      name: "Financeiro",
      authMode: "own",
      ownPassword: "senha-propria-1",
    });

    expect(vault.isSessionUnlocked(global)).toBe(true);
    expect(vault.isSessionUnlocked(own)).toBe(true); // recém-criada fica aberta

    vault.lockSession(own.id);
    expect(vault.isSessionUnlocked(own)).toBe(false);
    expect(await vault.unlockSession(own.id, "errada")).toBe(false);
    expect(await vault.unlockSession(own.id, "senha-propria-1")).toBe(true);
    expect(vault.isSessionUnlocked(own)).toBe(true);
  });

  it("bloquear o app fecha todas as sessões", async () => {
    await vault.createGlobalPassword("senha-global-123");
    const global = await vault.createSession({ name: "Trabalho", authMode: "global" });

    vault.lockApp();
    expect(vault.state.appUnlocked).toBe(false);
    expect(vault.isSessionUnlocked(global)).toBe(false);
  });
});
