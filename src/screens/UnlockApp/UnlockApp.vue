<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import { useVault } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const password = ref("");
const showPassword = ref(false);
const hintRevealed = ref(false);
const error = ref("");
const attempts = ref(0);
const cooldownUntil = ref(0);
const now = ref(Date.now());

// Atraso progressivo simples após erros (placeholder de UI).
const remaining = computed(() =>
  Math.max(0, Math.ceil((cooldownUntil.value - now.value) / 1000)),
);
const locked = computed(() => remaining.value > 0);

let ticker: ReturnType<typeof setInterval> | undefined;
function startTicker() {
  if (ticker) return;
  ticker = setInterval(() => {
    now.value = Date.now();
    if (remaining.value <= 0 && ticker) {
      clearInterval(ticker);
      ticker = undefined;
    }
  }, 250);
}

async function submit() {
  if (locked.value) return;
  error.value = "";
  const ok = await vault.unlockApp(password.value);
  if (ok) {
    router.push(vault.sessions.value.length ? "/sessions" : "/welcome");
    return;
  }
  attempts.value++;
  error.value = "Senha incorreta.";
  password.value = "";
  const delayMs = Math.min(8000, attempts.value * 2000);
  cooldownUntil.value = Date.now() + delayMs;
  now.value = Date.now();
  startTicker();
}
</script>

<template>
  <main class="flex min-h-screen items-center justify-center bg-app px-6 py-10 text-primary">
    <section class="w-full max-w-sm">
      <header class="mb-6 text-center">
        <div class="mx-auto flex h-16 w-16 items-center justify-center rounded-full bg-[var(--color-accent-soft)] text-accent">
          <UiIcon name="lock" :size="32" />
        </div>
        <h1 class="mt-5 text-2xl font-bold">Desbloquear Secrets Storage</h1>
        <p class="mt-2 text-sm text-secondary">
          Digite sua senha global para acessar o cofre.
        </p>
      </header>

      <UiCard>
        <form class="grid gap-3" @submit.prevent="submit">
          <label class="grid gap-1.5 text-sm font-medium text-primary" for="global-password">
            Senha global
            <span class="relative">
              <input
                id="global-password"
                v-model="password"
                :type="showPassword ? 'text' : 'password'"
                autocomplete="current-password"
                placeholder="Digite sua senha"
                class="h-10 w-full rounded-control border bg-surface px-3 pr-10 text-sm text-primary outline-none focus:border-accent"
                :class="error ? 'border-danger' : 'border-divider'"
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

          <button
            v-if="vault.state.globalHint"
            type="button"
            class="w-fit text-sm font-medium text-accent"
            @click="hintRevealed = !hintRevealed"
          >
            {{ hintRevealed ? "Ocultar dica" : "Mostrar dica" }}
          </button>

          <p
            v-if="hintRevealed && vault.state.globalHint"
            data-hint
            class="rounded-control border border-divider bg-elevated p-3 text-sm text-secondary"
          >
            Dica: {{ vault.state.globalHint }}
          </p>

          <p v-if="error" data-error class="text-xs text-danger">{{ error }}</p>
          <p v-if="locked" data-delay class="text-xs text-warning">
            Aguarde {{ remaining }}s…
          </p>

          <p class="text-xs leading-relaxed text-secondary">
            Desbloqueia todas as sessões que usam a senha global.
          </p>

          <UiButton type="submit" class="w-full" :disabled="locked">
            Desbloquear
          </UiButton>
        </form>
      </UiCard>
    </section>
  </main>
</template>
