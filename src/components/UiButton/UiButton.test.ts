import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiButton from "./UiButton.vue";

describe("UiButton", () => {
  it("monta com o slot e encaminha atributos nativos", () => {
    const wrapper = mount(UiButton, {
      attrs: { type: "submit", "aria-label": "Salvar segredo" },
      slots: { default: "Salvar" },
    });

    expect(wrapper.get("button").text()).toBe("Salvar");
    expect(wrapper.get("button").attributes()).toMatchObject({
      type: "submit",
      "aria-label": "Salvar segredo",
    });
  });

  it.each([
    ["primary", ["bg-accent", "text-white", "h-10"]],
    ["secondary", ["bg-transparent", "border-divider", "h-10"]],
    ["danger", ["bg-danger", "text-white", "h-10"]],
    ["ghost", ["bg-transparent", "h-9", "w-9"]],
  ] as const)("aplica a variante %s com tokens visuais", (variant, classes) => {
    const button = mount(UiButton, { props: { variant } }).get("button");

    expect(button.classes()).toEqual(expect.arrayContaining([...classes]));
    expect(button.classes()).toContain("rounded-control");
  });

  it("oferece foco accent visível e estado disabled muted", () => {
    const button = mount(UiButton, {
      attrs: { disabled: true },
    }).get("button");

    expect(button.attributes()).toHaveProperty("disabled");
    expect(button.classes()).toEqual(
      expect.arrayContaining([
        "focus-visible:ring-accent",
        "focus-visible:ring-2",
        "disabled:bg-muted",
        "disabled:cursor-not-allowed",
      ]),
    );
  });
});
