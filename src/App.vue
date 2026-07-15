<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";

const foundations = [
  "Tauri 2 com core Rust",
  "Vue 3 e TypeScript",
  "Tailwind CSS empacotado localmente",
  "CSP e capabilities mínimas",
];

const version = ref("");

onMounted(async () => {
  try {
    version.value = await getVersion();
  } catch {
    version.value = "";
  }
});
</script>

<template>
  <main class="min-h-screen bg-slate-950 px-6 py-16 text-slate-100">
    <section class="mx-auto max-w-3xl rounded-3xl border border-slate-800 bg-slate-900 p-8 shadow-2xl shadow-black/30 sm:p-12">
      <p class="text-sm font-semibold uppercase tracking-[0.24em] text-emerald-400">Secrets Storage</p>
      <h1 class="mt-4 text-4xl font-semibold tracking-tight sm:text-5xl">Fundação executável pronta</h1>
      <p class="mt-5 max-w-2xl text-lg leading-8 text-slate-300">
        O aplicativo ainda não manipula segredos reais. Esta etapa estabelece somente a base técnica para os protótipos de segurança do M0.
      </p>

      <ul class="mt-8 grid gap-3 sm:grid-cols-2">
        <li
          v-for="foundation in foundations"
          :key="foundation"
          class="rounded-xl border border-slate-700 bg-slate-950/60 px-4 py-3 text-sm text-slate-200"
        >
          {{ foundation }}
        </li>
      </ul>

      <aside class="mt-8 rounded-xl border border-amber-400/30 bg-amber-400/10 p-4 text-sm leading-6 text-amber-100">
        Não use esta versão para armazenar dados sensíveis. Os controles criptográficos e seus testes ainda serão implementados.
      </aside>

      <footer v-if="version" class="mt-8 text-right text-xs text-slate-500">
        Versão {{ version }}
      </footer>
    </section>
  </main>
</template>
