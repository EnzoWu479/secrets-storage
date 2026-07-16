<script setup lang="ts">
import { useRouter } from "vue-router";

import UiButton from "@/components/UiButton/UiButton.vue";
import UiCard from "@/components/UiCard/UiCard.vue";
import UiIcon from "@/components/UiIcon/UiIcon.vue";

const router = useRouter();
</script>

<template>
  <main class="flex min-h-screen items-center justify-center bg-app px-6 py-12 text-primary">
    <UiCard class="w-full max-w-xl text-center">
      <div
        class="mx-auto flex size-14 items-center justify-center rounded-card bg-elevated text-accent"
      >
        <UiIcon name="vault" :size="28" />
      </div>

      <h1 class="mt-6 text-3xl font-semibold">
        Crie sua primeira sessão de segurança
      </h1>
      <p class="mx-auto mt-3 max-w-md text-secondary">
        Cada sessão é um cofre. Por padrão usa sua senha mestra global; você pode dar a ela uma
        senha própria.
      </p>

      <UiButton class="mt-7" @click="router.push('/sessions/new')">Criar sessão</UiButton>
    </UiCard>
  </main>
</template>
