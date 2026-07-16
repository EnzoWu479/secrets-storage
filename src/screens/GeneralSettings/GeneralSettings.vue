<script setup lang="ts">
import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiToggle from "@/components/UiToggle/UiToggle.vue";
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="mx-auto max-w-4xl">
      <header>
        <h1 class="text-2xl font-bold">Configurações</h1>
        <p class="mt-1 text-sm text-secondary">Preferências gerais e informações deste aplicativo.</p>
      </header>

      <div class="mt-6 space-y-5">
        <section data-settings-section class="rounded-card border border-divider bg-surface">
          <header class="flex items-center gap-3 border-b border-divider px-5 py-4">
            <span class="text-accent"><UiIcon name="lock" /></span>
            <h2 class="font-semibold">Segurança</h2>
          </header>
          <div class="divide-y divide-divider px-5">
            <div class="flex flex-wrap items-center justify-between gap-4 py-4">
              <div>
                <strong class="text-sm font-medium">Trocar senha mestra global</strong>
                <p class="mt-1 text-xs text-secondary">Exige a senha atual para confirmar a alteração.</p>
              </div>
              <UiButton variant="secondary">Trocar senha mestra global</UiButton>
            </div>
            <div class="flex flex-wrap items-center justify-between gap-4 py-4">
              <div>
                <strong class="text-sm font-medium">Bloquear o aplicativo</strong>
                <p class="mt-1 text-xs text-secondary">Encerra agora o acesso a todas as sessões abertas.</p>
              </div>
              <UiButton variant="danger">Bloquear app agora</UiButton>
            </div>
          </div>
          <div role="alert" class="mx-5 mb-5 flex items-center gap-2 rounded-control border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-warning">
            <UiIcon name="warning" :size="18" />
            O v1 não possui recuperação de acesso.
          </div>
        </section>

        <section data-settings-section class="rounded-card border border-divider bg-surface">
          <header class="flex items-center gap-3 border-b border-divider px-5 py-4">
            <span class="text-accent"><UiIcon name="settings" /></span>
            <h2 class="font-semibold">Aparência</h2>
          </header>
          <div class="flex items-center justify-between gap-4 px-5 py-5">
            <div>
              <strong class="text-sm font-medium">Tema</strong>
              <p class="mt-1 text-xs text-secondary">Escolha a aparência usada pelo cofre.</p>
            </div>
            <div class="flex items-center gap-3 text-sm">
              <span class="text-secondary">Claro</span>
              <UiToggle checked />
              <span class="font-medium text-primary">Escuro</span>
            </div>
          </div>
        </section>

        <section data-settings-section class="rounded-card border border-divider bg-surface">
          <header class="flex items-center gap-3 border-b border-divider px-5 py-4">
            <span class="text-accent"><UiIcon name="eye" /></span>
            <h2 class="font-semibold">Privacidade</h2>
          </header>
          <div class="flex items-center justify-between px-5 py-5">
            <div>
              <strong class="text-sm font-medium">Telemetria: Desativada</strong>
              <p class="mt-1 text-xs text-secondary">Nenhum dado de uso é coletado ou enviado.</p>
            </div>
            <span class="rounded-pill bg-success/10 px-2 py-1 text-xs font-medium text-success">Privado</span>
          </div>
        </section>

        <section data-settings-section class="rounded-card border border-divider bg-surface">
          <header class="flex items-center gap-3 border-b border-divider px-5 py-4">
            <span class="text-accent"><UiIcon name="note" /></span>
            <h2 class="font-semibold">Sobre</h2>
          </header>
          <div class="flex flex-wrap items-center justify-between gap-4 px-5 py-5">
            <div>
              <strong class="text-sm font-medium">Secrets Storage v0.1.7 · Open source · Tauri + Vue</strong>
              <p class="mt-1 text-xs text-secondary">Cofre local-first e zero-knowledge.</p>
            </div>
            <nav class="flex gap-4 text-sm font-medium text-accent" aria-label="Links do projeto">
              <span role="link" tabindex="0">Repositório</span>
              <span role="link" tabindex="0">Licença</span>
            </nav>
          </div>
        </section>
      </div>
    </div>
  </AppShell>
</template>
