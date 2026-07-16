<script setup lang="ts">
import { useRouter } from "vue-router";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";

const router = useRouter();
</script>

<template>
  <main
    class="flex min-h-screen items-center justify-center bg-app px-6 py-10 text-primary"
  >
    <div class="grid w-full max-w-4xl gap-6 md:grid-cols-2">
      <section
        class="connecting-state flex min-h-80 cursor-pointer flex-col items-center justify-center rounded-card border border-divider bg-surface p-8 text-center"
        role="button"
        tabindex="0"
        title="Simular autorização concluída"
        @click="router.push('/unlock')"
      >
        <span
          class="connecting-spinner size-12 animate-spin rounded-full border-4 border-divider border-t-accent"
          aria-hidden="true"
        />

        <h1 class="mt-6 text-xl font-semibold">Abrindo o Google…</h1>
        <p class="mt-2 max-w-sm text-sm leading-6 text-secondary">
          Conclua a autorização na janela do navegador…
        </p>

        <UiButton
          type="button"
          variant="secondary"
          class="mt-8"
          @click.stop="router.push('/')"
        >
          Cancelar
        </UiButton>
      </section>

      <section
        class="connecting-state flex min-h-80 flex-col items-center justify-center rounded-card border border-divider bg-surface p-8 text-center"
      >
        <span
          class="flex size-12 items-center justify-center rounded-full bg-warning/10 text-warning"
        >
          <UiIcon name="warning" :size="24" />
        </span>

        <h2 class="mt-6 text-xl font-semibold">Autorização não concluída</h2>
        <p class="mt-2 max-w-sm text-sm leading-6 text-secondary">
          A janela foi fechada ou o tempo expirou. Inicie a autorização novamente
          para continuar.
        </p>

        <UiButton type="button" class="mt-8" @click="router.push('/unlock')">
          Tentar novamente
        </UiButton>
      </section>
    </div>
  </main>
</template>
