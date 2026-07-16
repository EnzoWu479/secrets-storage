<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import { useVault } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const session = computed(() => vault.activeSession.value);

const password = ref("");
const hintRevealed = ref(false);
const error = ref("");
const attempts = ref(0);
const cooldownUntil = ref(0);
const now = ref(Date.now());

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

onMounted(() => {
  // Sem sessão-alvo válida (ou já desbloqueada) não há o que fazer aqui.
  const s = session.value;
  if (!s || s.authMode !== "own") router.replace("/sessions");
  else if (vault.isSessionUnlocked(s)) router.replace("/secrets");
});

async function submit() {
  if (!session.value || locked.value) return;
  error.value = "";
  const ok = await vault.unlockSession(session.value.id, password.value);
  if (ok) {
    router.push("/secrets");
    return;
  }
  attempts.value++;
  error.value = "Senha incorreta.";
  password.value = "";
  cooldownUntil.value = Date.now() + Math.min(8000, attempts.value * 2000);
  now.value = Date.now();
  startTicker();
}
</script>

<template>
  <main class="flex min-h-screen items-center justify-center bg-app px-6 py-12 text-primary">
    <section v-if="session" class="w-full max-w-sm">
      <UiCard>
        <div class="text-center">
          <div class="mx-auto flex size-12 items-center justify-center rounded-card bg-elevated text-accent">
            <UiIcon name="lock" :size="24" />
          </div>
          <h1 class="mt-5 text-xl font-semibold">Desbloquear {{ session.name }}</h1>
          <p class="mt-2 text-sm text-secondary">Sessão com senha própria.</p>
        </div>

        <form class="mt-6 grid gap-3" @submit.prevent="submit">
          <UiInput v-model="password" label="Senha" type="password" placeholder="••••••••••••" />

          <button
            v-if="session.hint"
            type="button"
            class="w-fit text-sm font-medium text-accent"
            @click="hintRevealed = !hintRevealed"
          >
            {{ hintRevealed ? "Ocultar dica" : "Mostrar dica" }}
          </button>

          <p
            v-if="hintRevealed && session.hint"
            data-hint
            class="rounded-control bg-elevated px-3 py-2 text-sm text-secondary"
          >
            Dica: {{ session.hint }}
          </p>

          <div v-if="error || locked" class="grid gap-1 text-sm">
            <p v-if="error" data-error class="text-danger">{{ error }}</p>
            <p v-if="locked" data-delay class="text-warning">Aguarde {{ remaining }}s</p>
          </div>

          <div class="mt-3 flex justify-end gap-3">
            <UiButton type="button" variant="secondary" @click="router.push('/sessions')">
              Cancelar
            </UiButton>
            <UiButton type="submit" :disabled="locked">Desbloquear</UiButton>
          </div>
        </form>
      </UiCard>
    </section>
  </main>
</template>
