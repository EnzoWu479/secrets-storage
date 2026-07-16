import { config } from "@vue/test-utils";
import { createMemoryHistory, createRouter } from "vue-router";

import { registerGuards, routes } from "./router";

// Torna o router disponível a todos os mounts de teste (as telas usam
// useRouter() nos CTAs). Memory history evita depender do DOM de histórico.
export const router = createRouter({
  history: createMemoryHistory(),
  routes,
});

registerGuards(router);

config.global.plugins = [router];
