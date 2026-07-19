import { flushPromises, mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";

import SecurityProofHarness from "./SecurityProofHarness.vue";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

const canary = "discard-after-use";

function mountHarness() {
  return mount(SecurityProofHarness);
}

async function enterValidCanary(wrapper: ReturnType<typeof mountHarness>) {
  await wrapper.get("[data-canary]").setValue(canary);
  await wrapper.get("[data-scenario-nonce]").setValue("scenario-1");
}

beforeEach(() => {
  invoke.mockReset();
  invoke.mockResolvedValue({ state: "unlocked", epoch: 1 });
});

describe("SecurityProofHarness", () => {
  it("removes the sensitive input from DOM and state after completing", async () => {
    const wrapper = mountHarness();
    await enterValidCanary(wrapper);

    await wrapper.get("form").trigger("submit");
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("proof_install_canary", {
      input: {
        bytes: Array.from(new TextEncoder().encode(canary)),
        scenarioNonce: "scenario-1",
      },
    });
    expect(wrapper.find("[data-canary]").exists()).toBe(false);
    expect(wrapper.text()).not.toContain(canary);
  });

  it("removes the sensitive input from DOM and state when cancelled", async () => {
    const wrapper = mountHarness();
    await enterValidCanary(wrapper);

    await wrapper.get("[data-cancel]").trigger("click");

    expect(invoke).not.toHaveBeenCalled();
    expect(wrapper.find("[data-canary]").exists()).toBe(false);
    expect(wrapper.text()).not.toContain(canary);
  });

  it("locks before removing the sensitive input from DOM and state", async () => {
    const wrapper = mountHarness();
    await enterValidCanary(wrapper);

    await wrapper.get("[data-lock]").trigger("click");
    await flushPromises();

    expect(invoke).toHaveBeenCalledWith("proof_lock", {
      input: { reason: "manual" },
    });
    expect(wrapper.find("[data-canary]").exists()).toBe(false);
    expect(wrapper.text()).not.toContain(canary);
  });

  it("does not invoke IPC for an empty canary", async () => {
    const wrapper = mountHarness();
    await wrapper.get("[data-scenario-nonce]").setValue("scenario-1");

    await wrapper.get("form").trigger("submit");

    expect(invoke).not.toHaveBeenCalled();
    expect(wrapper.get("[data-error]").text()).toContain("obrigatório");
  });

  it("does not invoke IPC when the canary exceeds 4096 UTF-8 bytes", async () => {
    const wrapper = mountHarness();
    await wrapper.get("[data-canary]").setValue("a".repeat(4097));
    await wrapper.get("[data-scenario-nonce]").setValue("scenario-1");

    await wrapper.get("form").trigger("submit");

    expect(invoke).not.toHaveBeenCalled();
    expect(wrapper.get("[data-error]").text()).toContain("4096");
  });

  it("does not invoke IPC when the scenario nonce is invalid or oversized", async () => {
    const wrapper = mountHarness();
    await wrapper.get("[data-canary]").setValue(canary);
    await wrapper.get("[data-scenario-nonce]").setValue("a".repeat(65));

    await wrapper.get("form").trigger("submit");

    expect(invoke).not.toHaveBeenCalled();
    expect(wrapper.get("[data-error]").text()).toContain("cenário");
  });
});
