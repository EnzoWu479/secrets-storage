import { createApp } from "vue";
import { invoke } from "@tauri-apps/api/core";

import SecurityProofHarness from "./SecurityProofHarness.vue";

declare global {
  interface Window {
    __SECURITY_PROOF_START_INVOKE__: (
      command: string,
      input?: Record<string, unknown>,
    ) => number;
    __SECURITY_PROOF_TAKE_RESULT__: (
      operationId: number,
    ) =>
      | { ok: true; value: Record<string, unknown> }
      | { ok: false; error: string }
      | undefined;
  }
}

type ProofInvokeResult =
  | { ok: true; value: Record<string, unknown> }
  | { ok: false; error: string };

let nextOperationId = 1;
const operationResults = new Map<number, ProofInvokeResult>();

window.__SECURITY_PROOF_START_INVOKE__ = (command, input) => {
  const operationId = nextOperationId++;
  void invoke<Record<string, unknown>>(
        command,
        input === undefined ? undefined : { input },
      ).then(
    (value) => operationResults.set(operationId, { ok: true, value }),
    (error) =>
      operationResults.set(operationId, {
        ok: false,
        error: typeof error === "string" ? error : JSON.stringify(error),
      }),
  );
  return operationId;
};

window.__SECURITY_PROOF_TAKE_RESULT__ = (operationId) => {
  const result = operationResults.get(operationId);
  if (result) operationResults.delete(operationId);
  return result;
};

createApp(SecurityProofHarness).mount("#app");
