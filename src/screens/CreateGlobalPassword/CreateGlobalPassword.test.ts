import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { useVault } from "@/stores/vault";
import CreateGlobalPassword from "./CreateGlobalPassword.vue";

const vault = useVault();

beforeEach(() => {
  vault._resetForTests();
});

describe("CreateGlobalPassword", () => {
  it("mostra o aviso de ausência de recuperação e a dica exposta", () => {
    const wrapper = mount(CreateGlobalPassword);

    expect(wrapper.get("h1").text()).toBe("Crie sua senha mestra");
    expect(wrapper.get("[data-recovery-warning]").text()).toContain(
      "Não há recuperação",
    );
    expect(wrapper.get("[data-hint-warning]").text()).toContain(
      "visível sem a senha",
    );
  });

  it("só habilita o envio com senha longa e confirmação coincidente", async () => {
    const wrapper = mount(CreateGlobalPassword);
    const submit = wrapper.get('button[type="submit"]');

    expect(submit.attributes("disabled")).toBeDefined();

    await wrapper.get("#master-password").setValue("curta");
    await wrapper.get("#confirm-password").setValue("curta");
    expect(submit.attributes("disabled")).toBeDefined();

    await wrapper.get("#master-password").setValue("senha-longa-123");
    await wrapper.get("#confirm-password").setValue("senha-longa-123");
    expect(submit.attributes("disabled")).toBeUndefined();
  });

  it("cria a senha global ao enviar o formulário válido", async () => {
    const wrapper = mount(CreateGlobalPassword);

    await wrapper.get("#master-password").setValue("senha-longa-123");
    await wrapper.get("#confirm-password").setValue("senha-longa-123");
    await wrapper.get("#password-hint").setValue("dica boa");
    await wrapper.get("form").trigger("submit");

    await vi.waitFor(() => expect(vault.hasGlobalPassword.value).toBe(true));
    expect(vault.state.globalHint).toBe("dica boa");
  });
});
