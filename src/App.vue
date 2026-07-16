<script setup lang="ts">
// Casca do app: apenas hospeda o fluxo de telas roteadas.
// A navegação tela-a-tela vive em ./router e nos CTAs de cada tela.
</script>

<template>
  <div class="min-h-screen bg-app text-primary">
    <RouterView />
  </div>
</template>
