<script setup lang="ts">
import { useRouter } from "vue-router";

import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
import { SECRETS } from "@/fixtures";

const router = useRouter();

const revealedPassword = SECRETS[0].password;

const details = [
  {
    secret: SECRETS[0],
    icon: "password" as const,
    fields: [
      { label: "Usuário", value: SECRETS[0].username, sensitive: false },
      { label: "Senha", value: SECRETS[0].password, sensitive: true },
      { label: "URL", value: SECRETS[0].url, sensitive: false },
      { label: "Notas", value: SECRETS[0].notes, sensitive: false },
    ],
  },
  {
    secret: SECRETS[1],
    icon: "api" as const,
    fields: [
      { label: "Chave", value: SECRETS[1].key, sensitive: true },
      { label: "Ambiente", value: SECRETS[1].environment, sensitive: false },
      { label: "Escopos", value: "charges:read, customers:read", sensitive: false },
    ],
  },
  {
    secret: SECRETS[2],
    icon: "token" as const,
    fields: [
      { label: "Valor", value: SECRETS[2].value, sensitive: true },
      { label: "Expira", value: SECRETS[2].expiresAt, sensitive: false },
    ],
  },
  {
    secret: SECRETS[3],
    icon: "note" as const,
    fields: [
      { label: "Texto", value: SECRETS[3].text, sensitive: false },
    ],
  },
  {
    secret: SECRETS[4],
    icon: "ssh" as const,
    fields: [
      { label: "Chave pública", value: SECRETS[4].publicKey, sensitive: false },
      { label: "Chave privada", value: SECRETS[4].privateKey, sensitive: true },
      { label: "Passphrase", value: SECRETS[4].passphrase, sensitive: true },
    ],
  },
] as const;
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="mx-auto max-w-7xl">
      <header>
        <p class="text-sm text-secondary">Trabalho</p>
        <h1 class="mt-1 text-2xl font-semibold">Detalhes dos segredos</h1>
        <p class="mt-2 text-sm text-secondary">
          Variações estáticas dos cinco tipos suportados.
        </p>
      </header>

      <section class="mt-7 grid items-start gap-5 xl:grid-cols-2">
        <article
          v-for="detail in details"
          :key="detail.secret.type"
          class="secret-detail rounded-card border border-divider bg-surface p-5"
        >
          <header class="flex items-start gap-3 border-b border-divider pb-4">
            <span
              class="flex size-10 shrink-0 items-center justify-center rounded-control bg-elevated text-accent"
            >
              <UiIcon :name="detail.icon" :size="20" />
            </span>
            <span class="min-w-0 flex-1">
              <h2 class="truncate text-lg font-semibold">{{ detail.secret.name }}</h2>
              <UiBadge tone="accent" class="mt-1.5">
                {{ detail.secret.typeLabel }}
              </UiBadge>
            </span>
          </header>

          <dl class="mt-4 space-y-4">
            <div v-for="field in detail.fields" :key="field.label">
              <dt class="text-xs font-medium uppercase tracking-wide text-secondary">
                {{ field.label }}
              </dt>
              <dd class="mt-1 flex items-start gap-2 rounded-control bg-elevated px-3 py-2">
                <code
                  class="min-w-0 flex-1 whitespace-pre-wrap break-all font-mono text-sm text-primary"
                  :class="field.sensitive ? 'sensitive-value' : undefined"
                >{{ field.sensitive ? "••••••••" : field.value }}</code>
                <span v-if="field.sensitive" class="flex shrink-0 gap-2 text-secondary">
                  <UiIcon name="eye" :size="17" />
                  <UiIcon name="copy" :size="17" />
                </span>
                <UiIcon v-else name="copy" :size="17" class="shrink-0 text-secondary" />
              </dd>
            </div>
          </dl>

          <div
            v-if="detail.secret.type === 'password'"
            data-sensitive-state="revealed"
            class="mt-4 rounded-control border border-accent bg-[var(--color-accent-soft)] px-3 py-2"
          >
            <span class="text-xs font-medium uppercase tracking-wide text-accent">
              Senha revelada
            </span>
            <span class="mt-1 flex items-center gap-2">
              <code class="min-w-0 flex-1 break-all font-mono text-sm text-primary">
                {{ revealedPassword }}
              </code>
              <UiIcon name="eye" :size="17" class="shrink-0 text-accent" />
            </span>
          </div>

          <div
            v-if="detail.secret.type === 'password'"
            class="mt-4 flex flex-wrap items-center justify-between gap-3 rounded-control bg-success/10 px-3 py-2 text-sm text-success"
          >
            <span>Copiado. O clipboard será limpo em 05:00.</span>
            <button type="button" class="font-medium underline">Limpar agora</button>
          </div>

          <p class="mt-5 text-xs text-secondary">
            Criado em {{ detail.secret.createdAt }} · {{ detail.secret.updatedAt }}
          </p>

          <footer class="mt-4 flex justify-end gap-3 border-t border-divider pt-4">
            <UiButton
              type="button"
              variant="secondary"
              class="edit-secret"
              @click="router.push('/secrets/edit')"
            >
              Editar
            </UiButton>
            <UiButton
              type="button"
              variant="danger"
              class="delete-secret"
              @click="router.push('/secrets')"
            >
              Excluir
            </UiButton>
          </footer>
        </article>
      </section>
    </div>
  </AppShell>
</template>
