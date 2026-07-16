import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import Welcome from "./Welcome.vue";

describe("Welcome", () => {
  it("monta o estado vazio de boas-vindas com o conteúdo completo", () => {
    const wrapper = mount(Welcome);

    expect(wrapper.get("h1").text()).toBe("Crie sua primeira sessão de segurança");
    expect(wrapper.text()).toContain(
      "Cada sessão é um cofre. Por padrão usa sua senha mestra global; você pode dar a ela uma senha própria.",
    );
    expect(wrapper.getComponent(UiButton).text()).toBe("Criar sessão");
  });

  it("apresenta o cofre vazio centralizado com ícone de cadeado", () => {
    const wrapper = mount(Welcome);

    expect(wrapper.get("main").classes()).toEqual(
      expect.arrayContaining(["items-center", "justify-center"]),
    );
    expect(wrapper.findComponent(UiCard).exists()).toBe(true);
    expect(wrapper.find('[data-icon="vault"]').exists()).toBe(true);
    expect(wrapper.getComponent(UiIcon).props("name")).toBe("vault");
  });
});
