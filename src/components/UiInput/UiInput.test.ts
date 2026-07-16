import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiInput from "./UiInput.vue";

describe("UiInput", () => {
  it("renderiza um valor estático no input interno", () => {
    const input = mount(UiInput, {
      props: { label: "Nome", value: "Financeiro" },
    }).get("input");

    expect(input.attributes("value")).toBe("Financeiro");
  });

  it("monta um campo de texto com label e ajuda", () => {
    const wrapper = mount(UiInput, {
      props: {
        label: "Nome",
        help: "Como o item sera identificado",
        placeholder: "Ex.: Conta principal",
      },
    });

    expect(wrapper.get("label").text()).toBe("Nome");
    expect(wrapper.get("input").attributes("type")).toBe("text");
    expect(wrapper.get("input").attributes("placeholder")).toBe(
      "Ex.: Conta principal",
    );
    expect(wrapper.get(".ui-input__message").text()).toBe(
      "Como o item sera identificado",
    );
  });

  it("renderiza as variantes password e erro pelas classes visuais", () => {
    const password = mount(UiInput, {
      props: { label: "Senha", type: "password" },
    });
    const invalid = mount(UiInput, {
      props: { label: "Senha", error: "Senha obrigatoria" },
    });

    expect(password.get("input").attributes("type")).toBe("password");
    expect(password.get(".ui-input__password-icon").attributes("aria-hidden")).toBe(
      "true",
    );
    expect(invalid.get("input").classes()).toContain("border-danger");
    expect(invalid.get("input").attributes("aria-invalid")).toBe("true");
    expect(invalid.get(".ui-input__message").classes()).toContain("text-danger");
    expect(invalid.get(".ui-input__message").text()).toBe("Senha obrigatoria");
  });
});
