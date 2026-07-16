import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import { SECRETS } from "@/fixtures";
import SecretDetail from "./SecretDetail.vue";

describe("SecretDetail", () => {
  it("monta uma variacao para cada tipo com seus campos", () => {
    const wrapper = mount(SecretDetail);
    const details = wrapper.findAll(".secret-detail");

    expect(details).toHaveLength(5);
    expect(details.map((detail) => detail.get("h2").text())).toEqual([
      "GitHub",
      "Stripe (produção)",
      "Token CI/CD",
      "Recuperação da conta bancária",
      "Servidor de deploy",
    ]);
    expect(details[0].text()).toContain("Usuário");
    expect(details[0].text()).toContain("URL");
    expect(details[1].text()).toContain("Ambiente");
    expect(details[1].text()).toContain("Escopos");
    expect(details[2].text()).toContain("Expira");
    expect(details[3].text()).toContain("Código de recuperação");
    expect(details[4].text()).toContain("Chave pública");
    expect(details[4].text()).toContain("Chave privada");
    expect(details[4].text()).toContain("Passphrase");
  });

  it("mostra protecao, aviso de copia, metadados e acoes estaticas", () => {
    const wrapper = mount(SecretDetail);

    expect(wrapper.findAll(".sensitive-value").length).toBeGreaterThanOrEqual(5);
    expect(wrapper.get('[data-sensitive-state="revealed"]').text()).toContain(
      SECRETS[0].password,
    );
    expect(wrapper.findAll(".sensitive-value")[0].text()).toBe("••••••••");
    expect(wrapper.text()).toContain(
      "Copiado. O clipboard será limpo em 05:00.",
    );
    expect(wrapper.text()).toContain("Limpar agora");
    expect(wrapper.text()).toContain("Criado em 02/03/2026");
    expect(wrapper.text()).toContain("editado há 3 dias");
    expect(wrapper.findAll(".edit-secret")).toHaveLength(5);
    expect(wrapper.findAll(".delete-secret")).toHaveLength(5);
    expect(wrapper.findAll("button").some((button) => button.text() === "Editar")).toBe(
      true,
    );
    expect(wrapper.findAll("button").some((button) => button.text() === "Excluir")).toBe(
      true,
    );
  });
});
