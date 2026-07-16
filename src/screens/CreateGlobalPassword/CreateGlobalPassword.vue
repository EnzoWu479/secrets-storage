<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import PasswordStrength from "@/components/PasswordStrength/PasswordStrength.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import { passwordStrength } from "@/utils/password";
import { useVault } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const MIN_LENGTH = 8;

const password = ref("");
const confirm = ref("");
const hint = ref("");
const showPassword = ref(false);
const error = ref("");
const submitting = ref(false);

const strength = computed(() => passwordStrength(password.value));
const tooShort = computed(
  () => password.value.length > 0 && password.value.length < MIN_LENGTH,
);
const mismatch = computed(
  () => confirm.value.length > 0 && confirm.value !== password.value,
);
const canSubmit = computed(
  () =>
    password.value.length >= MIN_LENGTH &&
    confirm.value === password.value &&
    !submitting.value,
);

async function submit() {
  error.value = "";
  if (password.value.length < MIN_LENGTH) {
    error.value = `A senha precisa de pelo menos ${MIN_LENGTH} caracteres.`;
    return;
  }
  if (password.value !== confirm.value) {
    error.value = "As senhas não coincidem.";
    return;
  }
  submitting.value = true;
  try {
    await vault.createGlobalPassword(password.value, hint.value);
    router.push("/welcome");
  } finally {
    submitting.value = false;
  }
}
</script>

<template>
  <main class="flex min-h-screen items-center justify-center bg-app px-6 py-12 text-primary">
    <section class="w-full max-w-lg" aria-labelledby="create-password-title">
      <div class="text-center">
        <h1 id="create-password-title" class="text-3xl font-semibold">
          Crie sua senha mestra
        </h1>
        <p class="mx-auto mt-3 max-w-md text-secondary">
          Ela protege todo o app e desbloqueia suas sessões de uma vez.
        </p>
      </div>

      <form class="mt-8 grid gap-5" @submit.prevent="submit">
        <div class="grid gap-3">
          <label class="grid gap-1.5 text-sm font-medium text-primary" for="master-password">
            Senha mestra
            <span class="relative">
              <input
                id="master-password"
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                autocomplete="new-password"
                placeholder="Pelo menos 8 caracteres"
                class="h-10 w-full rounded-control border border-divider bg-surface px-3 pr-10 text-sm text-primary outline-none focus:border-accent"
              />
              <button
                type="button"
                class="absolute inset-y-0 right-3 flex items-center text-secondary"
                :aria-label="showPassword ? 'Ocultar senha' : 'Mostrar senha'"
                @click="showPassword = !showPassword"
              >
                <UiIcon name="eye" :size="16" />
              </button>
            </span>
          </label>
          <PasswordStrength :level="strength.level" />
          <p v-if="tooShort" class="text-xs text-danger">
            Mínimo de {{ MIN_LENGTH }} caracteres.
          </p>
        </div>

        <label class="grid gap-1.5 text-sm font-medium text-primary" for="confirm-password">
          Confirmar senha
          <input
            id="confirm-password"
            v-model="confirm"
            :type="showPassword ? 'text' : 'password'"
            autocomplete="new-password"
            class="h-10 w-full rounded-control border bg-surface px-3 text-sm text-primary outline-none focus:border-accent"
            :class="mismatch ? 'border-danger' : 'border-divider'"
          />
          <span v-if="mismatch" class="text-xs text-danger">As senhas não coincidem.</span>
        </label>

        <div class="grid gap-2">
          <label class="grid gap-1.5 text-sm font-medium text-primary" for="password-hint">
            Dica (opcional)
            <input
              id="password-hint"
              v-model="hint"
              type="text"
              placeholder="Uma pista que não revele a senha"
              class="h-10 w-full rounded-control border border-divider bg-surface px-3 text-sm font-normal text-primary outline-none focus:border-accent"
            />
          </label>
          <p data-hint-warning class="flex items-center gap-2 text-xs text-warning">
            <UiIcon name="warning" :size="14" />
            <span>a dica é visível sem a senha</span>
          </p>
        </div>

        <UiCard data-recovery-warning class="border-warning bg-warning/10">
          <div class="flex gap-3 text-warning">
            <UiIcon name="warning" class="mt-0.5 shrink-0" :size="18" />
            <p class="text-sm font-medium">
              Não há recuperação. Se você esquecer esta senha, o app fica inacessível.
            </p>
          </div>
        </UiCard>

        <p v-if="error" data-form-error class="text-sm text-danger">{{ error }}</p>

        <UiButton type="submit" class="w-full" :disabled="!canSubmit">
          Criar senha e continuar
        </UiButton>
      </form>
    </section>
  </main>
</template>
