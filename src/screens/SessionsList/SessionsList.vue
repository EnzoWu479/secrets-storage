<script setup lang="ts">
import { computed, ref } from "vue";
import { useRouter } from "vue-router";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import { useVault, type StoredSession } from "@/stores/vault";

const router = useRouter();
const vault = useVault();

const query = ref("");

const filtered = computed(() => {
  const q = query.value.trim().toLocaleLowerCase();
  if (!q) return vault.sessions.value;
  return vault.sessions.value.filter((s) =>
    s.name.toLocaleLowerCase().includes(q),
  );
});

function initial(name: string): string {
  return name.trim().charAt(0).toLocaleUpperCase() || "?";
}

function open(session: StoredSession) {
  vault.openSession(session.id);
  if (vault.isSessionUnlocked(session)) {
    router.push("/secrets");
  } else {
    router.push("/sessions/unlock");
  }
}
</script>

<template>
  <AppShell>
    <div class="mx-auto max-w-6xl">
      <header class="flex flex-wrap items-center justify-between gap-4">
        <div>
          <h1 class="text-2xl font-semibold">Sessões</h1>
          <p class="mt-1 text-sm text-secondary">
            Escolha um cofre para acessar os segredos disponíveis.
          </p>
        </div>

        <button
          type="button"
          class="new-session inline-flex h-10 items-center rounded-control bg-accent px-4 text-sm font-medium text-white"
          @click="router.push('/sessions/new')"
        >
          Nova sessão
        </button>
      </header>

      <label class="mt-7 block">
        <span class="sr-only">Buscar sessões</span>
        <input
          v-model="query"
          type="search"
          placeholder="Buscar sessões…"
          class="h-10 w-full rounded-control border border-divider bg-surface px-3 text-sm text-primary outline-none placeholder:text-muted focus:border-accent"
        />
      </label>

      <p
        v-if="!vault.sessions.value.length"
        data-empty
        class="mt-10 rounded-card border border-dashed border-divider bg-surface p-10 text-center text-sm text-secondary"
      >
        Nenhuma sessão ainda. Crie a primeira para começar.
      </p>

      <section
        v-else
        class="mt-6 grid gap-4 sm:grid-cols-2"
        aria-label="Sessões disponíveis"
      >
        <UiCard
          v-for="session in filtered"
          :key="session.id"
          class="session-card cursor-pointer"
          :data-session="session.name"
          role="button"
          tabindex="0"
          @click="open(session)"
        >
          <template #header>
            <div class="flex items-start gap-3">
              <span
                class="flex size-10 shrink-0 items-center justify-center rounded-full bg-elevated font-semibold"
              >
                {{ initial(session.name) }}
              </span>
              <span class="min-w-0 flex-1">
                <strong class="block text-base font-semibold">{{ session.name }}</strong>
                <span class="mt-2 flex flex-wrap gap-1.5">
                  <UiBadge :tone="session.authMode === 'global' ? 'accent' : 'neutral'">
                    {{ session.authMode === "global" ? "Global" : "Senha própria" }}
                  </UiBadge>
                  <UiBadge :tone="vault.isSessionUnlocked(session) ? 'success' : 'neutral'">
                    {{ vault.isSessionUnlocked(session) ? "Desbloqueada" : "Bloqueada" }}
                  </UiBadge>
                  <UiBadge v-if="session.readOnly" tone="warning">
                    Somente leitura
                  </UiBadge>
                </span>
              </span>
              <span :class="vault.isSessionUnlocked(session) ? 'text-success' : 'text-secondary'">
                <UiIcon
                  :name="vault.isSessionUnlocked(session) ? 'password' : 'lock'"
                  :size="20"
                />
              </span>
            </div>
          </template>

          <p class="mt-5 border-t border-divider pt-4 text-sm text-secondary">
            <template v-if="vault.isSessionUnlocked(session)">
              {{ session.secrets.length }} segredos
            </template>
            <template v-else>— segredos ocultos</template>
          </p>
        </UiCard>
      </section>
    </div>
  </AppShell>
</template>
