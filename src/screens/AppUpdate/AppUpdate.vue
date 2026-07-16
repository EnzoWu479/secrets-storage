<script setup lang="ts">
import AppShell from "@/components/AppShell/AppShell.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
</script>

<template>
  <AppShell>
    <section class="flex min-h-full items-center justify-center text-primary">
      <div class="grid w-full max-w-6xl items-start gap-6 lg:grid-cols-3">
      <article
        data-update-state="available"
        class="rounded-modal border border-divider bg-surface p-6"
      >
        <div class="flex size-11 items-center justify-center rounded-control bg-elevated text-accent">
          <UiIcon name="sync" :size="22" />
        </div>
        <p class="mt-5 text-xs font-medium uppercase tracking-wide text-accent">Disponível</p>
        <h2 class="mt-1 text-xl font-semibold">Nova versão v0.2.0</h2>
        <div class="mt-5 rounded-control bg-elevated p-4">
          <h3 class="text-sm font-medium">Notas da versão</h3>
          <ul class="mt-2 list-inside list-disc space-y-1 text-sm text-secondary">
            <li>Melhorias de segurança e estabilidade.</li>
            <li>Ajustes na sincronização dos cofres.</li>
          </ul>
        </div>
        <div class="mt-6 flex justify-end gap-3">
          <UiButton variant="secondary">Depois</UiButton>
          <UiButton>Instalar e reiniciar</UiButton>
        </div>
      </article>

      <article
        data-update-state="checking"
        class="rounded-modal border border-divider bg-surface p-6 text-center"
      >
        <UiIcon name="sync" class="mx-auto animate-spin text-accent" :size="32" />
        <p class="mt-5 text-xs font-medium uppercase tracking-wide text-secondary">Verificando</p>
        <h2 class="mt-1 text-xl font-semibold">Buscando atualizações…</h2>
        <p class="mt-2 text-sm text-secondary">Validando a versão mais recente com segurança.</p>
      </article>

      <article
        data-update-state="error"
        class="rounded-modal border border-danger bg-danger/5 p-6"
      >
        <div class="flex size-11 items-center justify-center rounded-control bg-danger/10 text-danger">
          <UiIcon name="warning" :size="22" />
        </div>
        <p class="mt-5 text-xs font-medium uppercase tracking-wide text-danger">Erro</p>
        <h2 class="mt-1 text-xl font-semibold">Não foi possível atualizar</h2>
        <p class="mt-3 text-sm text-danger">
          Atualização rejeitada: assinatura inválida. Sua versão atual continua funcionando.
        </p>
        <div class="mt-6 flex justify-end">
          <UiButton variant="secondary">Fechar</UiButton>
        </div>
      </article>
      </div>
    </section>
  </AppShell>
</template>
