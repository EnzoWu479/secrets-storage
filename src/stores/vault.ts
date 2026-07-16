import { computed, reactive } from "vue";

// ⚠️ PLACEHOLDER PRÉ-CRIPTO — NÃO É SEGURO / NÃO É ZERO-KNOWLEDGE.
// Este store dá comportamento funcional (senha global + ciclo de vida de
// sessões) usando estado reativo + persistência em localStorage. A senha é
// guardada apenas como hash SHA-256 (verificação), NÃO como o envelope
// criptográfico do cofre. Nada aqui implementa o formato definido em
// .specs/features/crypto-format — isso entra quando o backend Rust/cripto
// (Argon2id/AEAD, PT-01/PT-02) for implementado. Não use para dados sensíveis.

export type AuthMode = "global" | "own";

export interface StoredSecret {
  id: string;
  type: "password" | "api-key" | "token" | "secure-note" | "ssh-key";
  name: string;
  fields: Record<string, string>;
  createdAt: string;
  updatedAt: string;
}

export interface StoredSession {
  id: string;
  name: string;
  authMode: AuthMode;
  ownPasswordHash: string | null;
  hint: string | null;
  inactivitySecs: number | null; // null = "nunca"
  lockOnWindowsLock: boolean;
  lockOnSuspend: boolean;
  readOnly: boolean;
  createdAt: string;
  secrets: StoredSecret[];
}

interface Persisted {
  globalPasswordHash: string | null;
  globalHint: string | null;
  sessions: StoredSession[];
}

interface VaultState extends Persisted {
  appUnlocked: boolean;
  unlockedSessionIds: string[];
  activeSessionId: string | null;
}

const STORAGE_KEY = "secrets-storage:vault:v0";

const state = reactive<VaultState>({
  globalPasswordHash: null,
  globalHint: null,
  sessions: [],
  appUnlocked: false,
  unlockedSessionIds: [],
  activeSessionId: null,
});

function loadPersisted(): void {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed = JSON.parse(raw) as Partial<Persisted>;
    state.globalPasswordHash = parsed.globalPasswordHash ?? null;
    state.globalHint = parsed.globalHint ?? null;
    state.sessions = (parsed.sessions ?? []).map((s) => ({
      ...s,
      secrets: s.secrets ?? [],
    }));
  } catch {
    // storage corrompido/indisponível: começa do zero (fail-closed).
  }
}

function persist(): void {
  try {
    const data: Persisted = {
      globalPasswordHash: state.globalPasswordHash,
      globalHint: state.globalHint,
      sessions: state.sessions,
    };
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(data));
  } catch {
    // sem storage: mantém só em memória nesta sessão.
  }
}

// Hash de verificação (NÃO é o KDF do cofre). Só evita guardar a senha em claro.
async function hash(text: string): Promise<string> {
  const bytes = new TextEncoder().encode(text);
  const digest = await globalThis.crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest))
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

function normalize(name: string): string {
  return name.trim().toLocaleLowerCase();
}

function makeId(): string {
  // ID não-criptográfico, suficiente para chavear sessões no store.
  const c = globalThis.crypto;
  if (c && "randomUUID" in c) return c.randomUUID();
  return `s_${Date.now().toString(36)}_${state.sessions.length}`;
}

loadPersisted();

// ---- Getters ----

const hasGlobalPassword = computed(() => state.globalPasswordHash !== null);
const sessions = computed(() => state.sessions);
const activeSession = computed(
  () => state.sessions.find((s) => s.id === state.activeSessionId) ?? null,
);

function isSessionUnlocked(session: StoredSession): boolean {
  if (!state.appUnlocked) return false;
  if (session.authMode === "global") return true;
  return state.unlockedSessionIds.includes(session.id);
}

function nameTaken(name: string, exceptId?: string): boolean {
  const n = normalize(name);
  return state.sessions.some((s) => s.id !== exceptId && normalize(s.name) === n);
}

function sessionById(id: string): StoredSession | undefined {
  return state.sessions.find((s) => s.id === id);
}

// ---- Ações: senha global (GMP) ----

async function createGlobalPassword(password: string, hint?: string): Promise<void> {
  state.globalPasswordHash = await hash(password);
  state.globalHint = hint?.trim() ? hint.trim() : null;
  state.appUnlocked = true;
  persist();
}

async function unlockApp(password: string): Promise<boolean> {
  if (!state.globalPasswordHash) return false;
  const ok = (await hash(password)) === state.globalPasswordHash;
  if (ok) state.appUnlocked = true;
  return ok;
}

function lockApp(): void {
  state.appUnlocked = false;
  state.unlockedSessionIds = [];
  state.activeSessionId = null;
}

// ---- Ações: sessões ----

interface CreateSessionInput {
  name: string;
  authMode: AuthMode;
  ownPassword?: string;
  hint?: string;
  inactivitySecs?: number | null;
  lockOnWindowsLock?: boolean;
  lockOnSuspend?: boolean;
  readOnly?: boolean;
}

async function createSession(input: CreateSessionInput): Promise<StoredSession> {
  const name = input.name.trim();
  if (!name) throw new Error("Nome obrigatório");
  if (nameTaken(name)) throw new Error("Já existe uma sessão com esse nome");
  if (input.authMode === "own" && !input.ownPassword) {
    throw new Error("Senha própria obrigatória");
  }

  const session: StoredSession = {
    id: makeId(),
    name,
    authMode: input.authMode,
    ownPasswordHash:
      input.authMode === "own" && input.ownPassword
        ? await hash(input.ownPassword)
        : null,
    hint: input.hint?.trim() ? input.hint.trim() : null,
    inactivitySecs: input.inactivitySecs ?? 15 * 60,
    lockOnWindowsLock: input.lockOnWindowsLock ?? true,
    lockOnSuspend: input.lockOnSuspend ?? true,
    readOnly: input.readOnly ?? false,
    createdAt: new Date().toISOString(),
    secrets: [],
  };

  state.sessions.push(session);
  // Sessões `own` já ficam desbloqueadas para quem acabou de defini-las.
  if (session.authMode === "own") state.unlockedSessionIds.push(session.id);
  persist();
  return session;
}

async function unlockSession(id: string, password: string): Promise<boolean> {
  const session = sessionById(id);
  if (!session || session.authMode !== "own" || !session.ownPasswordHash) {
    return false;
  }
  const ok = (await hash(password)) === session.ownPasswordHash;
  if (ok && !state.unlockedSessionIds.includes(id)) {
    state.unlockedSessionIds.push(id);
  }
  return ok;
}

function lockSession(id: string): void {
  const session = sessionById(id);
  if (!session) return;
  if (session.authMode === "global") {
    // Globais seguem o app: "bloquear" uma global bloqueia o app inteiro.
    lockApp();
    return;
  }
  state.unlockedSessionIds = state.unlockedSessionIds.filter((x) => x !== id);
  if (state.activeSessionId === id) state.activeSessionId = null;
}

function openSession(id: string): void {
  state.activeSessionId = id;
}

function renameSession(id: string, newName: string): void {
  const session = sessionById(id);
  if (!session) return;
  if (!isSessionUnlocked(session)) {
    throw new Error("A sessão precisa estar desbloqueada para renomear");
  }
  if (nameTaken(newName, id)) throw new Error("Nome já usado por outra sessão");
  session.name = newName.trim();
  persist();
}

async function deleteSession(id: string, password: string): Promise<boolean> {
  const session = sessionById(id);
  if (!session) return false;
  const expected =
    session.authMode === "own" ? session.ownPasswordHash : state.globalPasswordHash;
  if (!expected) return false;
  if ((await hash(password)) !== expected) return false;

  state.sessions = state.sessions.filter((s) => s.id !== id);
  state.unlockedSessionIds = state.unlockedSessionIds.filter((x) => x !== id);
  if (state.activeSessionId === id) state.activeSessionId = null;
  persist();
  return true;
}

function setLockPolicy(
  id: string,
  policy: Partial<
    Pick<
      StoredSession,
      "inactivitySecs" | "lockOnWindowsLock" | "lockOnSuspend" | "readOnly"
    >
  >,
): void {
  const session = sessionById(id);
  if (!session) return;
  Object.assign(session, policy);
  persist();
}

// Apenas para testes: restaura o store a um estado limpo.
function _resetForTests(): void {
  state.globalPasswordHash = null;
  state.globalHint = null;
  state.sessions = [];
  state.appUnlocked = false;
  state.unlockedSessionIds = [];
  state.activeSessionId = null;
  try {
    globalThis.localStorage?.removeItem(STORAGE_KEY);
  } catch {
    /* ignore */
  }
}

export function useVault() {
  return {
    state,
    // getters
    hasGlobalPassword,
    sessions,
    activeSession,
    isSessionUnlocked,
    nameTaken,
    sessionById,
    // senha global
    createGlobalPassword,
    unlockApp,
    lockApp,
    // sessões
    createSession,
    unlockSession,
    lockSession,
    openSession,
    renameSession,
    deleteSession,
    setLockPolicy,
    // testes
    _resetForTests,
  };
}
