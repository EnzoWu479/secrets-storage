import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it } from "vitest";

import App from "./App.vue";
import { useVault } from "./stores/vault";
import { router } from "./test-setup";

const vault = useVault();

beforeEach(() => {
  vault._resetForTests();
});

describe("App", () => {
  it("no primeiro uso, o gate leva à criação da senha global", async () => {
    await router.push("/");
    await router.isReady();
    await flushPromises();

    const wrapper = mount(App);
    await flushPromises();

    expect(router.currentRoute.value.name).toBe("create-password");
    expect(wrapper.get("h1").text()).toBe("Crie sua senha mestra");
  });

  it("com senha global e app desbloqueado, permite chegar às sessões", async () => {
    await vault.createGlobalPassword("senha-global-123");

    await router.push("/sessions");
    await flushPromises();

    const wrapper = mount(App);
    await flushPromises();

    expect(router.currentRoute.value.name).toBe("sessions");
    expect(wrapper.text()).toContain("Sessões");
  });
});
