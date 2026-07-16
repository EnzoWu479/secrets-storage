<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import PasswordStrength from "@/components/PasswordStrength/PasswordStrength.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import UiToggle from "@/components/UiToggle/UiToggle.vue";
import { passwordStrength } from "@/utils/password";
import { useVault } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const MIN_LENGTH = 8;

const name = ref("");
const useOwnPassword = ref(false);
const ownPassword = ref("");
const confirmPassword = ref("");
const hint = ref("");
const inactivity = ref<"60" | "900" | "never">("900");
const lockOnWindowsLock = ref(true);
const lockOnSuspend = ref(true);
const error = ref("");
const submitting = ref(false);

const nameError = computed(() =>
  name.value.trim() && vault.nameTaken(name.value)
    ? "Já existe uma sessão com esse nome."
    : "",
);
const strength = computed(() => passwordStrength(ownPassword.value));

async function submit() {
  error.value = "";
  const trimmed = name.value.trim();
  if (!trimmed) {
    error.value = "Dê um nome à sessão.";
    return;
  }
  if (vault.nameTaken(trimmed)) {
    error.value = "Já existe uma sessão com esse nome.";
    return;
  }
  if (useOwnPassword.value) {
    if (ownPassword.value.length < MIN_LENGTH) {
      error.value = `A senha própria precisa de pelo menos ${MIN_LENGTH} caracteres.`;
      return;
    }
    if (ownPassword.value !== confirmPassword.value) {
      error.value = "As senhas não coincidem.";
      return;
    }
  }

  submitting.value = true;
  try {
    const session = await vault.createSession({
      name: trimmed,
      authMode: useOwnPassword.value ? "own" : "global",
      ownPassword: useOwnPassword.value ? ownPassword.value : undefined,
      hint: useOwnPassword.value ? hint.value : undefined,
      inactivitySecs: inactivity.value === "never" ? null : Number(inactivity.value),
      lockOnWindowsLock: lockOnWindowsLock.value,
      lockOnSuspend: lockOnSuspend.value,
    });
    vault.openSession(session.id);
    router.push("/secrets");
  } catch (e) {
    error.value = e instanceof Error ? e.message : "Não foi possível criar a sessão.";
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <main class="min-h-screen bg-app px-6 py-10 text-primary flex flex-col items-center justify-center">
    <form
      class="mx-auto max-w-xl rounded-modal border border-divider bg-surface p-6"
      @submit.prevent="submit"
    >
      <header class="flex items-center gap-3 border-b border-divider pb-5">
        <span class="flex size-10 items-center justify-center rounded-control bg-elevated text-accent">
          <UiIcon name="lock" :size="20" />
        </span>
        <h1 class="text-xl font-semibold">Nova sessão</h1>
      </header>

      <div class="mt-5 space-y-5">
        <UiInput
          v-model="name"
          label="Nome"
          placeholder="Ex.: Trabalho"
          :error="nameError || undefined"
          help="nome único, sem diferenciar maiúsculas/minúsculas"
        />

        <div class="flex items-center justify-between gap-4">
          <span class="text-sm font-medium">Usar senha própria para esta sessão</span>
          <UiToggle v-model="useOwnPassword" class="own-password-toggle" />
        </div>

        <div
          v-if="!useOwnPassword"
          class="rounded-control bg-[var(--color-accent-soft)] p-3 text-sm text-accent"
        >
          Esta sessão usará a senha mestra global e abrirá junto com o app.
        </div>

        <div v-else class="password-fields space-y-4">
          <UiInput v-model="ownPassword" label="Senha" type="password" placeholder="••••••••••••" />
          <PasswordStrength :level="strength.level" />
          <UiInput
            v-model="confirmPassword"
            label="Confirmar senha"
            type="password"
            placeholder="••••••••••••"
          />
          <UiInput v-model="hint" label="Dica (opcional)" placeholder="Uma pista que não revele a senha" />
          <div class="flex gap-2 rounded-control bg-warning/10 p-3 text-sm text-warning">
            <UiIcon name="warning" :size="18" />
            <span>Esta senha não pode ser recuperada se for esquecida.</span>
          </div>
        </div>

        <div>
          <label class="text-sm font-medium" for="idle">Política de inatividade</label>
          <select
            id="idle"
            v-model="inactivity"
            class="mt-1.5 h-10 w-full rounded-control border border-divider bg-surface px-3 text-sm"
          >
            <option value="60">1 min</option>
            <option value="900">15 min</option>
            <option value="never">Nunca</option>
          </select>
        </div>

        <div class="space-y-3 border-t border-divider pt-4">
          <div class="flex items-center justify-between gap-4 text-sm">
            <span>Bloquear ao bloquear o Windows</span>
            <UiToggle v-model="lockOnWindowsLock" />
          </div>
          <div class="flex items-center justify-between gap-4 text-sm">
            <span>Bloquear ao suspender</span>
            <UiToggle v-model="lockOnSuspend" />
          </div>
        </div>
      </div>

      <p v-if="error" data-form-error class="mt-5 text-sm text-danger">{{ error }}</p>

      <footer class="mt-6 flex justify-end gap-3 border-t border-divider pt-5">
        <UiButton type="button" variant="secondary" @click="router.push('/sessions')">
          Cancelar
        </UiButton>
        <UiButton type="submit" :disabled="submitting || !!nameError">
          Criar sessão
        </UiButton>
      </footer>
    </form>
  </main>
</template>
