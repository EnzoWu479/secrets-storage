<script setup lang="ts">
import { useRouter } from "vue-router";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiInput from "@/components/UiInput/UiInput.vue";
import UiToggle from "@/components/UiToggle/UiToggle.vue";

const router = useRouter();
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="mx-auto max-w-4xl">
      <header class="flex items-center gap-3">
        <span
          class="flex size-10 items-center justify-center rounded-control bg-elevated text-accent"
        >
          <UiIcon name="settings" :size="20" />
        </span>
        <div>
          <h1 class="text-2xl font-semibold">Configurações da sessão</h1>
          <p class="text-sm text-secondary">Trabalho</p>
        </div>
      </header>

      <div class="mt-6 grid gap-5">
        <section
          data-settings-section="general"
          class="rounded-card border border-divider bg-surface p-5"
        >
          <h2 class="text-lg font-semibold">Geral</h2>
          <div class="mt-4 grid gap-4 md:grid-cols-3">
            <UiInput label="Nome da sessão" placeholder="Trabalho" />

            <label data-inactivity class="grid gap-1.5 text-sm font-medium text-primary">
              Bloquear por inatividade
              <select
                class="h-10 rounded-control border border-divider bg-surface px-3 text-sm text-primary"
              >
                <option selected>15 min</option>
              </select>
            </label>

            <label data-clipboard class="grid gap-1.5 text-sm font-medium text-primary">
              Limpar clipboard
              <select
                class="h-10 rounded-control border border-divider bg-surface px-3 text-sm text-primary"
              >
                <option selected>5 min</option>
              </select>
            </label>
          </div>
        </section>

        <section
          data-settings-section="authentication"
          class="rounded-card border border-divider bg-surface p-5"
        >
          <div class="flex flex-wrap items-start justify-between gap-4">
            <div>
              <h2 class="text-lg font-semibold">Autenticação</h2>
              <p class="mt-2 text-sm text-primary">Usa a senha mestra global</p>
              <p class="mt-1 max-w-xl text-xs text-secondary">
                Atenção: alterar exige a senha atual apropriada; também é possível voltar à senha
                global depois.
              </p>
            </div>
            <UiButton variant="secondary">Definir senha própria</UiButton>
          </div>
        </section>

        <section
          data-settings-section="access"
          class="rounded-card border border-divider bg-surface p-5"
        >
          <h2 class="text-lg font-semibold">Acesso</h2>
          <div class="mt-4 flex items-center justify-between gap-4">
            <div>
              <p class="text-sm font-medium">Modo somente leitura</p>
              <p class="mt-1 text-xs text-secondary">
                Impede alterações nos segredos desta sessão.
              </p>
            </div>
            <UiToggle :checked="false" />
          </div>
        </section>

        <section
          data-settings-section="danger"
          class="rounded-card border border-danger bg-danger/5 p-5"
        >
          <div class="flex items-start gap-3 text-danger">
            <UiIcon name="warning" class="mt-0.5 shrink-0" :size="20" />
            <div>
              <h2 class="text-lg font-semibold">Zona de perigo</h2>
              <p class="mt-1 text-sm">
                Excluir sessão remove todo o cofre e exige confirmação e senha.
              </p>
            </div>
          </div>

          <div class="mt-5 grid gap-4 md:grid-cols-2">
            <UiInput label="Digite Trabalho para confirmar" placeholder="Trabalho" />
            <UiInput label="Senha atual" type="password" placeholder="••••••••••••" />
          </div>

          <div class="mt-5 flex justify-end">
            <UiButton variant="danger" @click="router.push('/sessions')">Excluir sessão</UiButton>
          </div>
        </section>
      </div>
    </div>
  </AppShell>
</template>

<style scoped>
:deep(input::placeholder) {
  color: var(--color-primary);
  opacity: 1;
}
</style>
