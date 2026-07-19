<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

const MAX_CANARY_BYTES = 4096;
const MAX_IDENTIFIER_BYTES = 64;
const validIdentifier = /^[A-Za-z0-9_-]+$/;

const active = ref(true);
const canary = ref("");
const scenarioNonce = ref("");
const error = ref("");
const outcome = ref("");

function clearSensitiveInput() {
  canary.value = "";
  scenarioNonce.value = "";
  active.value = false;
}

function validate() {
  const bytes = new TextEncoder().encode(canary.value);
  if (bytes.length === 0) return "O canário é obrigatório.";
  if (bytes.length > MAX_CANARY_BYTES) {
    return `O canário deve ter no máximo ${MAX_CANARY_BYTES} bytes.`;
  }
  if (
    scenarioNonce.value.length === 0 ||
    scenarioNonce.value.length > MAX_IDENTIFIER_BYTES ||
    !validIdentifier.test(scenarioNonce.value)
  ) {
    return "O identificador do cenário é inválido.";
  }
  return undefined;
}

async function complete() {
  error.value = "";
  const validationError = validate();
  if (validationError) {
    error.value = validationError;
    return;
  }

  const input = {
    bytes: Array.from(new TextEncoder().encode(canary.value)),
    scenarioNonce: scenarioNonce.value,
  };

  try {
    await invoke("proof_install_canary", { input });
    outcome.value = "Canário instalado para a prova.";
  } catch {
    error.value = "Não foi possível concluir a prova.";
  } finally {
    clearSensitiveInput();
  }
}

function cancel() {
  outcome.value = "Entrada descartada.";
  clearSensitiveInput();
}

async function lock() {
  error.value = "";
  try {
    await invoke("proof_lock", { input: { reason: "manual" } });
    outcome.value = "Harness bloqueado.";
  } catch {
    error.value = "Não foi possível bloquear o harness.";
  } finally {
    clearSensitiveInput();
  }
}
</script>

<template>
  <main class="security-proof-harness">
    <h1>Prova de segurança local</h1>
    <p>Use somente um canário descartável. A entrada sai da tela ao concluir, cancelar ou bloquear.</p>

    <form v-if="active" @submit.prevent="complete">
      <label>
        Canário descartável
        <input data-canary v-model="canary" type="password" autocomplete="off" />
      </label>
      <label>
        Identificador do cenário
        <input
          data-scenario-nonce
          v-model="scenarioNonce"
          type="text"
          maxlength="64"
          autocomplete="off"
        />
      </label>
      <p v-if="error" data-error role="alert">{{ error }}</p>
      <button type="submit">Concluir</button>
      <button data-cancel type="button" @click="cancel">Cancelar</button>
      <button data-lock type="button" @click="lock">Bloquear</button>
    </form>

    <p v-if="outcome" data-outcome>{{ outcome }}</p>
    <p v-else-if="!active" data-outcome>Entrada removida.</p>
  </main>
</template>
