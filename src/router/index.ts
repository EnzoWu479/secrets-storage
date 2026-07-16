import {
  createRouter,
  createWebHashHistory,
  type Router,
  type RouteRecordRaw,
} from "vue-router";

import { useVault } from "@/stores/vault";

// Fluxo de telas do app. Ainda sem Tauri invoke/criptografia:
// o estado vem do store de frontend (ver ../stores/vault).
export const routes: RouteRecordRaw[] = [
  { path: "/", name: "login", component: () => import("@/screens/LoginGoogle/LoginGoogle.vue") },
  { path: "/connecting", name: "connecting", component: () => import("@/screens/Connecting/Connecting.vue") },
  { path: "/create-password", name: "create-password", component: () => import("@/screens/CreateGlobalPassword/CreateGlobalPassword.vue") },
  { path: "/unlock", name: "unlock", component: () => import("@/screens/UnlockApp/UnlockApp.vue") },
  { path: "/welcome", name: "welcome", component: () => import("@/screens/Welcome/Welcome.vue") },
  { path: "/sessions", name: "sessions", component: () => import("@/screens/SessionsList/SessionsList.vue") },
  { path: "/sessions/new", name: "session-new", component: () => import("@/screens/CreateSession/CreateSession.vue") },
  { path: "/sessions/unlock", name: "session-unlock", component: () => import("@/screens/UnlockSession/UnlockSession.vue") },
  { path: "/secrets", name: "secrets", component: () => import("@/screens/SecretsList/SecretsList.vue") },
  { path: "/secrets/detail", name: "secret-detail", component: () => import("@/screens/SecretDetail/SecretDetail.vue") },
  { path: "/secrets/new", name: "secret-new", component: () => import("@/screens/SecretForm/SecretForm.vue") },
  { path: "/secrets/edit", name: "secret-edit", component: () => import("@/screens/SecretForm/SecretForm.vue") },
  { path: "/session-settings", name: "session-settings", component: () => import("@/screens/SessionSettings/SessionSettings.vue") },
  { path: "/sync", name: "sync", component: () => import("@/screens/Sync/Sync.vue") },
  { path: "/conflicts", name: "conflicts", component: () => import("@/screens/ConflictResolution/ConflictResolution.vue") },
  { path: "/update", name: "update", component: () => import("@/screens/AppUpdate/AppUpdate.vue") },
  { path: "/settings", name: "settings", component: () => import("@/screens/GeneralSettings/GeneralSettings.vue") },
  { path: "/:pathMatch(.*)*", redirect: "/" },
];

// Telas de entrada de sync/login ficam fora do caminho local por ora.
const ENTRY_ROUTES = new Set([
  "login",
  "connecting",
  "create-password",
  "unlock",
]);

// Gate de acesso baseado no estado real do cofre (senha global + app lock).
export function registerGuards(target: Router): void {
  target.beforeEach((to) => {
    const vault = useVault();

    if (!vault.hasGlobalPassword.value) {
      // Primeiro uso: só a criação da senha global.
      return to.name === "create-password" ? true : { name: "create-password" };
    }

    if (!vault.state.appUnlocked) {
      // Cofre existente e app bloqueado: exige a senha global.
      return to.name === "unlock" ? true : { name: "unlock" };
    }

    // App desbloqueado: não deixa voltar para as telas de entrada.
    if (to.name && ENTRY_ROUTES.has(to.name as string)) {
      return { name: vault.sessions.value.length ? "sessions" : "welcome" };
    }

    return true;
  });
}

export const router = createRouter({
  history: createWebHashHistory(),
  routes,
});

registerGuards(router);

export default router;
