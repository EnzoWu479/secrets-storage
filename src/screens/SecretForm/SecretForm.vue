<script setup lang="ts">
import { useRouter } from "vue-router";

import AppShell from "@/components/AppShell/AppShell.vue";
import PasswordStrength from "@/components/PasswordStrength/PasswordStrength.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import UiInput from "@/components/UiInput/UiInput.vue";

const router = useRouter();
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="relative min-h-[720px]">
      <div class="absolute inset-0 rounded-card bg-app/70" aria-hidden="true" />

      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="secret-form-title"
        class="relative mx-auto max-w-2xl rounded-modal border border-divider bg-surface shadow-2xl"
      >
        <header class="flex items-center justify-between border-b border-divider px-6 py-5">
          <div>
            <h1 id="secret-form-title" class="text-xl font-bold">Criar segredo</h1>
            <p class="mt-1 text-sm text-secondary">Adicione um item cifrado à sessão Trabalho.</p>
          </div>
          <span class="flex size-9 items-center justify-center rounded-control text-secondary">
            <UiIcon name="lock" :size="18" />
          </span>
        </header>

        <div class="space-y-6 px-6 py-5">
          <fieldset>
            <legend class="mb-2 text-sm font-medium">Tipo de segredo</legend>
            <div class="grid grid-cols-5 gap-2">
              <button
                data-secret-type
                type="button"
                aria-pressed="true"
                class="flex flex-col items-center gap-2 rounded-control border border-accent bg-[var(--color-accent-soft)] px-2 py-3 text-xs font-medium text-accent"
              >
                <UiIcon name="password" />
                Senha
              </button>
              <button data-secret-type type="button" aria-pressed="false" class="flex flex-col items-center gap-2 rounded-control border border-divider px-2 py-3 text-xs text-secondary">
                <UiIcon name="api" />
                API key
              </button>
              <button data-secret-type type="button" aria-pressed="false" class="flex flex-col items-center gap-2 rounded-control border border-divider px-2 py-3 text-xs text-secondary">
                <UiIcon name="token" />
                Token
              </button>
              <button data-secret-type type="button" aria-pressed="false" class="flex flex-col items-center gap-2 rounded-control border border-divider px-2 py-3 text-xs text-secondary">
                <UiIcon name="note" />
                Nota
              </button>
              <button data-secret-type type="button" aria-pressed="false" class="flex flex-col items-center gap-2 rounded-control border border-divider px-2 py-3 text-xs text-secondary">
                <UiIcon name="ssh" />
                SSH
              </button>
            </div>
          </fieldset>

          <div class="grid gap-4 sm:grid-cols-2">
            <label class="grid gap-1.5 text-sm font-medium">
              Nome
              <input name="name" value="GitHub" class="h-10 rounded-control border border-divider bg-surface px-3 font-normal outline-none" />
            </label>
            <label class="grid gap-1.5 text-sm font-medium">
              Usuário
              <input name="username" value="usuario@exemplo.com" class="h-10 rounded-control border border-divider bg-surface px-3 font-normal outline-none" />
            </label>
          </div>

          <div>
            <UiInput label="Senha" type="password" value="MockPassword2026!">
              <template #password-icon>
                <UiIcon name="eye" :size="16" />
              </template>
            </UiInput>
            <div class="mt-2">
              <PasswordStrength :level="4" />
            </div>
          </div>

          <label class="grid gap-1.5 text-sm font-medium">
            URL
            <input name="url" value="https://github.com" class="h-10 rounded-control border border-divider bg-surface px-3 font-normal outline-none" />
          </label>

          <label class="grid gap-1.5 text-sm font-medium">
            Notas
            <textarea name="notes" rows="3" class="resize-none rounded-control border border-divider bg-surface px-3 py-2 font-normal outline-none">Conta principal de desenvolvimento.</textarea>
          </label>
        </div>

        <footer class="flex justify-end gap-2 border-t border-divider px-6 py-4">
          <UiButton variant="secondary" @click="router.push('/secrets')">Cancelar</UiButton>
          <UiButton @click="router.push('/secrets')">Salvar</UiButton>
        </footer>
      </section>
    </div>
  </AppShell>
</template>
