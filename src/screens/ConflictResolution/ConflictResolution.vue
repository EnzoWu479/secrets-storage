<script setup lang="ts">
import AppShell from "@/components/AppShell/AppShell.vue";
import UiBadge from "@/components/UiBadge/UiBadge.vue";
import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";
</script>

<template>
  <AppShell active-session="Trabalho">
    <div class="relative min-h-[760px]">
      <div class="absolute inset-0 rounded-card bg-app/70" aria-hidden="true" />

      <section
        role="dialog"
        aria-modal="true"
        aria-labelledby="conflict-title"
        class="relative mx-auto max-w-4xl overflow-hidden rounded-modal border border-divider bg-surface shadow-2xl"
      >
        <header class="border-b border-divider px-6 py-5">
          <div class="flex items-center gap-3">
            <span class="flex size-10 items-center justify-center rounded-control bg-warning/10 text-warning">
              <UiIcon name="warning" />
            </span>
            <div>
              <div class="flex items-center gap-2">
                <h1 id="conflict-title" class="text-xl font-bold">Resolver conflito</h1>
                <UiBadge tone="warning">Pendente</UiBadge>
              </div>
              <p class="mt-1 text-sm text-secondary">Compare cada campo antes de aplicar a resolução.</p>
            </div>
          </div>
        </header>

        <div role="alert" class="mx-6 mt-5 flex items-center gap-2 rounded-control border border-warning/40 bg-warning/10 px-4 py-3 text-sm text-warning">
          <UiIcon name="warning" :size="18" />
          <span>Conflito pendente em GitHub (Trabalho). Expira em 6 dias…</span>
        </div>

        <div class="space-y-5 px-6 py-5">
          <article class="conflict-field rounded-card border border-divider">
            <h2 class="border-b border-divider px-4 py-3 text-sm font-semibold">Usuário</h2>
            <div class="grid md:grid-cols-2">
              <section data-side="local" class="border-b border-divider p-4 md:border-b-0 md:border-r">
                <div class="flex items-center justify-between">
                  <strong class="text-xs uppercase tracking-wide text-secondary">Local</strong>
                  <span class="text-xs text-muted">editado há 1h</span>
                </div>
                <code class="mt-3 block rounded-control bg-app p-3 text-sm text-primary">usuario@exemplo.com</code>
              </section>
              <section data-side="remote" class="p-4">
                <div class="flex items-center justify-between">
                  <strong class="text-xs uppercase tracking-wide text-secondary">Remoto</strong>
                  <span class="text-xs text-muted">Notebook · há 3h</span>
                </div>
                <code class="mt-3 block rounded-control bg-app p-3 text-sm text-primary">usuario.trabalho@exemplo.com</code>
              </section>
            </div>
            <footer class="flex flex-wrap justify-end gap-2 border-t border-divider px-4 py-3">
              <button type="button" class="rounded-control border border-divider px-3 py-2 text-xs text-secondary">Manter local</button>
              <button type="button" class="rounded-control border border-divider px-3 py-2 text-xs text-secondary">Manter remoto</button>
              <button type="button" class="rounded-control border border-accent px-3 py-2 text-xs text-accent">Manter ambos</button>
            </footer>
          </article>

          <article class="conflict-field rounded-card border border-divider" data-field="password">
            <h2 class="border-b border-divider px-4 py-3 text-sm font-semibold">Senha</h2>
            <div class="grid md:grid-cols-2">
              <section data-side="local" class="border-b border-divider p-4 md:border-b-0 md:border-r">
                <div class="flex items-center justify-between">
                  <strong class="text-xs uppercase tracking-wide text-secondary">Local</strong>
                  <span class="text-xs text-muted">editado há 1h</span>
                </div>
                <code class="mt-3 block rounded-control bg-app p-3 text-sm tracking-widest text-primary">••••••••••••</code>
              </section>
              <section data-side="remote" class="p-4">
                <div class="flex items-center justify-between">
                  <strong class="text-xs uppercase tracking-wide text-secondary">Remoto</strong>
                  <span class="text-xs text-muted">Notebook · há 3h</span>
                </div>
                <code class="mt-3 block rounded-control bg-app p-3 text-sm tracking-widest text-primary">••••••••••••••</code>
              </section>
            </div>
            <footer class="flex flex-wrap justify-end gap-2 border-t border-divider px-4 py-3">
              <button type="button" class="rounded-control border border-divider px-3 py-2 text-xs text-secondary">Manter local</button>
              <button type="button" class="rounded-control border border-divider px-3 py-2 text-xs text-secondary">Manter remoto</button>
              <button type="button" class="rounded-control border border-accent px-3 py-2 text-xs text-accent">Manter ambos</button>
            </footer>
          </article>
        </div>

        <footer class="flex justify-end gap-2 border-t border-divider px-6 py-4">
          <UiButton variant="secondary">Resolver depois</UiButton>
          <UiButton>Aplicar resolução</UiButton>
        </footer>
      </section>
    </div>
  </AppShell>
</template>
