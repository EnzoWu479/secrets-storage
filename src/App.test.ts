import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import App from "./App.vue";

describe("App", () => {
  it("identifica a fundação sem anunciar um cofre utilizável", () => {
    const wrapper = mount(App);

    expect(wrapper.get("h1").text()).toBe("Fundação executável pronta");
    expect(wrapper.text()).toContain("não manipula segredos reais");
    expect(wrapper.text()).toContain("Não use esta versão para armazenar dados sensíveis");
  });

  it("lista os quatro pilares técnicos aprovados", () => {
    const items = mount(App).findAll("li").map((item) => item.text());

    expect(items).toEqual([
      "Tauri 2 com core Rust",
      "Vue 3 e TypeScript",
      "Tailwind CSS empacotado localmente",
      "CSP e capabilities mínimas",
    ]);
  });
});
