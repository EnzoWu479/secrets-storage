type InvokeResult =
  | { ok: true; value: Record<string, unknown> }
  | { ok: false; error: string };

async function startProof(
  command: string,
  input?: Record<string, unknown>,
): Promise<void> {
  await browser.execute(
    (
      invokedCommand: string,
      invokedInput: Record<string, unknown> | undefined,
    ) => {
      (
        window as typeof window & {
          __SECURITY_PROOF_START_INVOKE__: (
            command: string,
            input?: Record<string, unknown>,
          ) => number;
        }
      ).__SECURITY_PROOF_START_INVOKE__(invokedCommand, invokedInput);
    },
    command,
    input,
  );
}

async function invokeProof(
  command: string,
  input?: Record<string, unknown>,
): Promise<InvokeResult> {
  const operationId = await browser.execute(
    (
      invokedCommand: string,
      invokedInput: Record<string, unknown> | undefined,
    ) => {
      return (
        window as typeof window & {
          __SECURITY_PROOF_START_INVOKE__: (
            command: string,
            input?: Record<string, unknown>,
          ) => number;
        }
      ).__SECURITY_PROOF_START_INVOKE__(invokedCommand, invokedInput);
    },
    command,
    input,
  );

  let result: InvokeResult | null | undefined;
  await browser.waitUntil(
    async () => {
      result = await browser.execute((pendingOperationId: number) => {
        return (
          window as typeof window & {
            __SECURITY_PROOF_TAKE_RESULT__: (
              operationId: number,
            ) => InvokeResult | undefined;
          }
        ).__SECURITY_PROOF_TAKE_RESULT__(pendingOperationId);
      }, operationId);
      return result != null;
    },
    { timeout: 5_000, interval: 25 },
  );
  return result!;
}

async function installCanary(bytes = [65, 66, 67]) {
  return invokeProof("proof_install_canary", {
    bytes,
    scenarioNonce: "webdriver",
  });
}

describe("security-proof runtime authority", () => {
  it("runs only in the dedicated proof window label", async () => {
    const label = await browser.execute(() => {
      const metadata = (
        window as typeof window & {
          __TAURI_INTERNALS__: { metadata: { currentWindow: { label: string } } };
        }
      ).__TAURI_INTERNALS__.metadata;
      return metadata.currentWindow.label;
    });

    expect(label).toBe("security-proof");
  });

  it("allows the proof window to install a bounded canary", async () => {
    const result = await installCanary();

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toMatchObject({ state: "unlocked" });
      expect(JSON.stringify(result.value)).not.toContain("ABC");
    }
  });

  it("allows an authorized probe without echoing its identifier", async () => {
    expect((await installCanary()).ok).toBe(true);

    const result = await invokeProof("proof_authorized_probe", {
      identifier: "webdriver-probe",
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.value).toMatchObject({ authorized: true });
      expect(JSON.stringify(result.value)).not.toContain("webdriver-probe");
    }
  });

  it("denies an authorized probe after lock", async () => {
    expect((await installCanary()).ok).toBe(true);
    expect(
      (
        await invokeProof("proof_lock", {
          reason: "manual",
        })
      ).ok,
    ).toBe(true);

    const before = await invokeProof("proof_status");
    await startProof("proof_authorized_probe", {
      identifier: "after-lock",
    });
    await browser.pause(100);
    const after = await invokeProof("proof_status");

    expect(before).toMatchObject({ ok: true, value: { state: "locked" } });
    expect(after).toEqual(before);
  });

  it("denies an unknown command without changing proof state", async () => {
    const before = await invokeProof("proof_status");
    await startProof("proof_not_registered");
    await browser.pause(100);
    const after = await invokeProof("proof_status");

    expect(before.ok).toBe(true);
    expect(after).toEqual(before);
  });

  it("denies malformed install input before allocation", async () => {
    const before = await invokeProof("proof_status");
    await startProof("proof_install_canary", {
      bytes: "not-an-array",
      scenarioNonce: "webdriver",
    });
    await browser.pause(100);
    const after = await invokeProof("proof_status");

    expect(before).toMatchObject({ ok: true, value: { state: "locked" } });
    expect(after).toEqual(before);
  });

  it("denies an oversized canary before allocation", async () => {
    const before = await invokeProof("proof_status");
    await startProof("proof_install_canary", {
      bytes: Array.from({ length: 4097 }, () => 65),
      scenarioNonce: "webdriver",
    });
    await browser.pause(100);
    const after = await invokeProof("proof_status");

    expect(before).toMatchObject({ ok: true, value: { state: "locked" } });
    expect(after).toEqual(before);
  });

  it("keeps controlled XSS inside the proof UI without expanding authority", async () => {
    const xssCanary =
      '<img src=x onerror="window.__securityProofXssExpanded=true">SECURITY_PROOF_XSS_CANARY';
    const canary = await browser.$("[data-canary]");
    const nonce = await browser.$("[data-scenario-nonce]");

    await canary.setValue(xssCanary);
    await nonce.setValue("xss");
    await browser.$('button[type="submit"]').click();
    await browser.waitUntil(async () => !(await browser.$("[data-canary]").isExisting()));

    const observation = await browser.execute(() => ({
      expanded:
        (window as typeof window & { __securityProofXssExpanded?: boolean })
          .__securityProofXssExpanded === true,
      markerInDom: document.documentElement.textContent?.includes("SECURITY_PROOF_XSS_CANARY"),
    }));
    const before = await invokeProof("proof_status");
    await startProof("proof_not_registered");
    await browser.pause(100);
    const after = await invokeProof("proof_status");

    expect(observation).toEqual({ expanded: false, markerInDom: false });
    expect(after).toEqual(before);
  });
});
