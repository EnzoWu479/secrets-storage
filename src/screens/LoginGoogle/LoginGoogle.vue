<script setup lang="ts">
import { useRouter } from "vue-router";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";

const router = useRouter();
</script>

<template>
  <main class="flex min-h-screen items-center justify-center bg-app p-6 text-primary">
    <section class="w-full max-w-[380px] text-center">
      <div class="mx-auto flex h-14 w-14 items-center justify-center rounded-full bg-[var(--color-accent-soft)] text-accent">
        <UiIcon name="lock" :size="28" />
      </div>

      <h1 class="mt-5 text-[28px] font-bold leading-tight">Secrets Storage</h1>
      <p class="mt-2 text-sm text-secondary">
        Cofre local-first e zero-knowledge…
      </p>

      <UiButton class="mt-8 w-full gap-2" @click="router.push('/connecting')">
        <UiIcon name="google" />
        Continuar com Google
      </UiButton>

      <p class="mt-4 text-xs leading-relaxed text-secondary">
        Usamos sua conta apenas para sincronizar o cofre já cifrado…
      </p>

      <footer class="mt-12 text-[11px] text-muted">
        v0.1.7 · Open source · Telemetria desativada
      </footer>
    </section>
  </main>
</template>
