<script setup lang="ts">
import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";

const sessions = [
  { name: "Trabalho", status: "Sincronizado", tone: "success" },
  { name: "Pessoal", status: "Enviando…", tone: "accent" },
  { name: "Projeto X", status: "Somente leitura", tone: "warning" },
] as const;
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="mx-auto max-w-4xl">
      <header class="flex flex-wrap items-start justify-between gap-4">
        <div>
          <h1 class="text-2xl font-semibold">Sincronização</h1>
          <p class="mt-1 text-sm text-secondary">Status dos cofres conectados à nuvem.</p>
        </div>
        <div data-network class="flex items-center gap-2 text-sm text-secondary">
          <span class="size-2 rounded-full bg-success" />
          <span>Rede</span>
          <UiBadge tone="success">Online</UiBadge>
        </div>
      </header>

      <section class="mt-6 rounded-card border border-divider bg-surface p-5">
        <div class="flex flex-wrap items-center justify-between gap-4">
          <div class="flex items-center gap-3">
            <span
              class="flex size-11 items-center justify-center rounded-control bg-elevated text-primary"
            >
              <UiIcon name="google-drive" :size="24" />
            </span>
            <div>
              <h2 class="font-semibold">Google Drive</h2>
              <p class="text-sm text-secondary">enzo.wu@exemplo.com</p>
            </div>
          </div>

          <div class="flex flex-wrap items-center gap-3">
            <span class="text-right text-xs text-secondary">
              <UiBadge tone="success">Sincronizado</UiBadge>
              <span class="mt-1 block">há 2 min</span>
            </span>
            <UiButton variant="secondary">Desconectar / revogar</UiButton>
          </div>
        </div>
      </section>

      <section class="mt-5 rounded-card border border-divider bg-surface p-5">
        <h2 class="font-semibold">Sessões</h2>
        <div class="mt-4 divide-y divide-divider">
          <div
            v-for="session in sessions"
            :key="session.name"
            :data-sync-session="session.name"
            class="flex items-center justify-between gap-4 py-4 first:pt-0 last:pb-0"
          >
            <span class="font-medium">{{ session.name }}</span>
            <span class="flex items-center gap-2">
              <UiIcon
                v-if="session.name === 'Pessoal'"
                name="sync"
                class="animate-spin text-accent"
                :size="16"
              />
              <UiBadge :tone="session.tone">{{ session.status }}</UiBadge>
            </span>
          </div>
        </div>
      </section>

      <aside
        data-offline
        class="mt-5 flex items-start gap-3 rounded-card border border-warning bg-warning/10 p-4 text-warning"
      >
        <UiIcon name="warning" class="mt-0.5 shrink-0" :size="18" />
        <div>
          <UiBadge tone="warning">Offline</UiBadge>
          <p class="mt-2 text-sm font-medium">
            Alterações locais serão enviadas quando a conexão voltar
          </p>
        </div>
      </aside>
    </div>
  </AppShell>
</template>
