<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import type { StoredSecret } from "@/stores/vault";
import { useVault } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const session = computed(() => vault.activeSession.value);
const query = ref("");

const secrets = computed<StoredSecret[]>(() => {
  const list = session.value?.secrets ?? [];
  const q = query.value.trim().toLocaleLowerCase();
  if (!q) return list;
  return list.filter((s) => s.name.toLocaleLowerCase().includes(q));
});

const iconFor = {
  password: "password",
  "api-key": "api",
  token: "token",
  "secure-note": "note",
  "ssh-key": "ssh",
} as const;

onMounted(() => {
  // Precisa de uma sessão ativa e desbloqueada para ver segredos.
  const s = session.value;
  if (!s) router.replace("/sessions");
  else if (!vault.isSessionUnlocked(s)) router.replace("/sessions/unlock");
});

function lockSession() {
  if (session.value) vault.lockSession(session.value.id);
  router.push("/sessions");
}
</script>

<template>
  <AppShell>
    <div v-if="session" class="mx-auto max-w-4xl">
      <header class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-2xl font-bold">{{ session.name }}</h1>
            <UiBadge tone="accent">Sessão ativa</UiBadge>
          </div>
          <p class="mt-1 text-sm text-secondary">
            {{ session.secrets.length }}
            {{ session.secrets.length === 1 ? "segredo" : "segredos" }} neste cofre
          </p>
        </div>
        <div class="flex gap-2">
          <button
            class="h-10 rounded-control border border-divider px-4 text-sm font-medium text-secondary"
            type="button"
            @click="lockSession"
          >
            Bloquear sessão
          </button>
          <button
            class="h-10 rounded-control bg-accent px-4 text-sm font-medium text-white"
            type="button"
            @click="router.push('/secrets/new')"
          >
            Adicionar
          </button>
        </div>
      </header>

      <div class="mt-6 max-w-md">
        <UiInput v-model="query" label="Buscar segredos" placeholder="Buscar segredos…" />
      </div>

      <section
        v-if="secrets.length"
        class="mt-6 overflow-hidden rounded-card border border-divider bg-surface"
      >
        <article
          v-for="secret in secrets"
          :key="secret.id"
          class="secret-row flex cursor-pointer items-center gap-4 border-b border-divider px-4 py-4 last:border-b-0"
          role="button"
          tabindex="0"
          @click="router.push('/secrets/detail')"
        >
          <span class="flex size-10 shrink-0 items-center justify-center rounded-control bg-elevated text-accent">
            <UiIcon :name="iconFor[secret.type]" />
          </span>
          <div class="min-w-0 flex-1">
            <h3 class="truncate text-sm font-semibold">{{ secret.name }}</h3>
          </div>
        </article>
      </section>

      <section
        v-else
        data-empty
        class="mt-6 flex min-h-80 flex-col items-center justify-center rounded-card border border-dashed border-divider bg-surface px-6 text-center"
      >
        <span class="flex size-14 items-center justify-center rounded-full bg-elevated text-muted">
          <UiIcon name="lock" :size="26" />
        </span>
        <h2 class="mt-5 text-lg font-semibold">Nenhum segredo ainda</h2>
        <p class="mt-2 max-w-sm text-sm text-secondary">
          Adicione senhas, tokens, chaves e notas cifradas a esta sessão.
        </p>
        <button
          class="mt-5 h-10 rounded-control bg-accent px-4 text-sm font-medium text-white"
          type="button"
          @click="router.push('/secrets/new')"
        >
          Adicionar primeiro segredo
        </button>
      </section>
    </div>
  </AppShell>
</template>
