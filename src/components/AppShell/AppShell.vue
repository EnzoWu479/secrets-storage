<script setup lang="ts">
import { useRouter } from "vue-router";

import { useVault, type StoredSession } from "@/stores/vault";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";

// `activeSession` é aceito por compatibilidade com telas antigas, mas o
// destaque real vem do estado do cofre (activeSessionId).
defineProps<{ activeSession?: string }>();

const router = useRouter();
const vault = useVault();

function initial(name: string): string {
  return name.trim().charAt(0).toLocaleUpperCase() || "?";
}

function open(session: StoredSession) {
  vault.openSession(session.id);
  router.push(vault.isSessionUnlocked(session) ? "/secrets" : "/sessions/unlock");
}

function lockApp() {
  vault.lockApp();
  router.push("/unlock");
}
</script>

<template>
  <div class="app-shell flex min-h-screen bg-app text-primary">
    <aside class="flex w-60 shrink-0 flex-col border-r border-divider bg-surface">
      <header
        class="flex cursor-pointer items-center gap-3 border-b border-divider px-5 py-5"
        role="button"
        tabindex="0"
        @click="router.push('/sessions')"
      >
        <span class="flex size-9 items-center justify-center rounded-control bg-accent text-white">
          <UiIcon name="lock" :size="18" />
        </span>
        <span>
          <strong class="block text-sm font-semibold">Secrets Storage</strong>
          <span class="text-xs text-secondary">Cofre local</span>
        </span>
      </header>

      <section class="flex-1 overflow-y-auto px-3 py-5">
        <p class="px-2 pb-2 text-[11px] font-medium uppercase text-muted">Sessões</p>

        <p
          v-if="!vault.sessions.value.length"
          class="px-2 py-2 text-xs text-secondary"
        >
          Nenhuma sessão ainda.
        </p>

        <div class="space-y-1.5">
          <div
            v-for="session in vault.sessions.value"
            :key="session.id"
            class="app-shell__session cursor-pointer rounded-control border px-3 py-3"
            :class="
              session.id === vault.state.activeSessionId
                ? 'border-accent bg-elevated'
                : 'border-transparent'
            "
            :data-session="session.name"
            role="button"
            tabindex="0"
            @click="open(session)"
          >
            <div class="flex items-start gap-2.5">
              <span
                class="flex size-8 shrink-0 items-center justify-center rounded-full bg-divider text-xs font-semibold"
              >
                {{ initial(session.name) }}
              </span>

              <span class="min-w-0 flex-1">
                <strong class="block truncate text-sm font-medium">{{ session.name }}</strong>
                <span class="mt-1 flex flex-wrap gap-1">
                  <UiBadge :tone="session.authMode === 'global' ? 'accent' : 'neutral'">
                    {{ session.authMode === "global" ? "Global" : "Senha própria" }}
                  </UiBadge>
                  <UiBadge :tone="vault.isSessionUnlocked(session) ? 'success' : 'neutral'">
                    {{ vault.isSessionUnlocked(session) ? "Desbloqueada" : "Bloqueada" }}
                  </UiBadge>
                  <UiBadge v-if="session.readOnly" tone="warning">Somente leitura</UiBadge>
                </span>
                <span class="mt-1.5 block text-xs text-secondary">
                  <template v-if="vault.isSessionUnlocked(session)">
                    {{ session.secrets.length }} segredos
                  </template>
                  <template v-else>— segredos ocultos</template>
                </span>
              </span>
            </div>
          </div>
        </div>

        <button
          type="button"
          class="app-shell__lock mt-5 flex w-full items-center justify-center gap-2 rounded-control border border-divider px-3 py-2 text-sm text-secondary hover:bg-elevated"
          @click="lockApp"
        >
          <UiIcon name="lock" :size="16" />
          Bloquear app
        </button>
      </section>

      <footer class="border-t border-divider px-3 py-3 text-xs text-secondary">
        <div class="flex items-center justify-between">
          <button
            type="button"
            class="flex flex-col items-center gap-1 rounded-control px-2 py-1 hover:text-primary"
            @click="router.push('/sync')"
          >
            <UiIcon name="sync" :size="16" />
            Sync
          </button>
          <button
            type="button"
            class="flex flex-col items-center gap-1 rounded-control px-2 py-1 hover:text-primary"
            @click="router.push('/settings')"
          >
            <UiIcon name="settings" :size="16" />
            Configurações
          </button>
          <button
            type="button"
            class="flex flex-col items-center gap-1 rounded-control px-2 py-1 hover:text-primary"
            @click="router.push('/settings')"
          >
            <UiIcon name="note" :size="16" />
            Sobre
          </button>
        </div>
      </footer>
    </aside>

    <main class="min-w-0 flex-1 p-6 lg:p-8">
      <slot />
    </main>
  </div>
</template>
