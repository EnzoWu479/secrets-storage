import { mount, type VueWrapper } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import Sync from "./Sync.vue";

describe("Sync", () => {
  it("monta a conta Google Drive e o estado da rede no AppShell", () => {
    const wrapper = mount(Sync);

    expect(wrapper.findComponent(AppShell).exists()).toBe(true);
    expect(wrapper.text()).toContain("Google Drive");
    expect(wrapper.text()).toContain("enzo.wu@exemplo.com");
    expect(wrapper.text()).toContain("Sincronizado");
    expect(wrapper.text()).toContain("há 2 min");
    expect(wrapper.findAllComponents(UiButton).map((button: VueWrapper) => button.text())).toEqual([
      "Desconectar / revogar",
    ]);
    expect(wrapper.get('[data-network]').text()).toContain("Online");
  });

  it("exibe os três estados por sessão e o callout offline", () => {
    const wrapper = mount(Sync);
    const sessions = wrapper.findAll("[data-sync-session]");

    expect(sessions).toHaveLength(3);
    expect(wrapper.get('[data-sync-session="Trabalho"]').text()).toContain("Sincronizado");

    const uploading = wrapper.get('[data-sync-session="Pessoal"]');
    expect(uploading.text()).toContain("Enviando…");
    expect(uploading.getComponent(UiIcon).classes()).toContain("animate-spin");

    expect(wrapper.get('[data-sync-session="Projeto X"]').text()).toContain("Somente leitura");
    expect(wrapper.findAllComponents(UiBadge).length).toBeGreaterThanOrEqual(5);

    const offline = wrapper.get('[data-offline]');
    expect(offline.classes()).toContain("border-warning");
    expect(offline.text()).toContain("Offline");
    expect(offline.text()).toContain(
      "Alterações locais serão enviadas quando a conexão voltar",
    );
  });
});
