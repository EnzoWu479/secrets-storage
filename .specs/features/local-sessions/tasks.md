# Tarefas — Sessões de Segurança e Desbloqueio (`local-sessions`)

**Design:** [design.md](./design.md) · **Formato cripto:** [../crypto-format/design.md](../crypto-format/design.md)
**Status:** Em progresso — fatia **backend Rust** (SessionManager + storage + comandos Tauri).

> Esta fatia liga o núcleo `crypto::*` (já implementado e testado — 53 testes) ao app,
> através de uma camada de comandos estreitos (C-10: a WebView é não confiável; todo
> comando revalida estado no Rust). **Continua candidato**: o gate D-05 (modelo de
> ameaças) e PT-01/PT-02 seguem abertos. O frontend permanece no placeholder inseguro
> (`src/stores/vault.ts`, AD-023) até a fatia de rewire.

---

## Fases

### Fase 1 — Fundação de armazenamento e modelo (T1–T2)

- [ ] **T1 `session::error`** — `SessionError` tipado e serializável (IPC), sem eco de
  segredo/senha (C-15). Mapeia `crypto::CryptoError` para variantes genéricas
  (`Auth` para qualquer falha de autenticação; `CorruptOrIncompatible` para
  magic/versão/CBOR/params).
- [ ] **T2 `session::model` + `session::storage`** — tipos persistidos (`Registry`,
  `SessionEntry`, `AuthMode`, `LockPolicy`) com normalização de nome (unicidade
  case-insensitive) e camada de armazenamento com escrita atômica: `registry.json`
  (JSON legível bloqueado — AD-013), `keyring.vault` e `vaults/<uuid>.vault` (CBOR).

### Fase 2 — Orquestração de app-unlock (GMP) (T3)

- [ ] **T3 `SessionManager` (gate global)** — estado gerenciado (`AppLock`, `unlocked`,
  `attempts`); comandos `app_status`, `create_global_password`, `unlock_app`
  (deriva a GMK e abre **todas** as sessões `global` de uma vez — D-03), `lock_app`,
  `change_global_password`. Atraso progressivo em memória (VAULT-04 AC2).

### Fase 3 — Ciclo de vida das sessões (T4)

- [ ] **T4 comandos de sessão** — `list_sessions`, `create_session` (unicidade;
  `global` exige app desbloqueado; `own` exige senha própria + força mínima),
  `unlock_session` (apenas `own`; verifica nome autenticado vs. registry),
  `lock_session`, `lock_all`, `change_master_password` (rewrap own→own),
  `set_session_auth_mode` (rewrap own↔global), `set_lock_policy`, `touch_session`,
  `reveal_hint`, `delete_session`.

### Fase 4 — Integração Tauri e bloqueio por inatividade (T5)

- [ ] **T5 wiring** — `manage(SessionManager)` + `invoke_handler` em `lib.rs`; tarefa
  de varredura de inatividade (`sweep_locks`) por relógio; `lock_app` no encerramento.

### Fase 5 — Gate

- [ ] **T6** — `pnpm check:rust` verde (fmt + clippy `-D warnings` + testes).

---

## Fora desta fatia (rastreado)

- **`rename_session`** — renomear honrando o nome autenticado na AAD do `.vault`
  exige um primitivo de *troca de nome* no `crypto::envelope` (o header autentica
  `session_name`, e `rewrap` preserva o nome). Fica para um follow-up que estende
  o envelope com essa operação. Renomear só no `registry.json` quebraria a
  verificação de integridade do nome no `unlock`.
- **Rewire do frontend** (`sessionsApi.ts` + `vault.ts` chamando `invoke`) — próxima fatia.
- **Eventos de bloqueio/suspensão do Windows** — best-effort; validação profunda em PT-05.
- **Zeroização de senhas que chegam via IPC como `String`** — endurecida em PT-04.
- **CRUD de segredos** — o payload nasce vazio (`secrets: []`); fatia de segredos preenche.
