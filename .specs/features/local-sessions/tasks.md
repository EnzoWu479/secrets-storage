# Sessões de Segurança e Desbloqueio — Tasks

**Spec:** [spec.md](./spec.md)
**Design:** [design.md](./design.md)
**Status:** Approved — 2026-07-21
**Escopo:** `SESSION-01…25`

## Objetivo e destravamento do G1

Esta fatia entrega o `SessionManager` de produção. Sua conclusão fornece a **evidência de liberação do gate externo G1** de [secret-management](../secret-management/tasks.md), que hoje bloqueia T14/T15/T23 daquela feature. O trait `SessionAccess` já existe e pertence ao consumidor (`secrets`); aqui implementamos o adaptador.

## Política de execução

- Todo código de comportamento usa `test-driven-development`: RED observado, GREEN mínimo, refactor só com a suíte verde.
- Decisões de segurança, cripto, Rust core, Win32, authority e integração ficam com o **agente principal**.
- Tarefas frontend estreitamente delimitadas podem usar `delegate-small-tasks`; o agente principal inspeciona e roda o gate.
- Cada tarefa concluída recebe **um** Conventional Commit em português.
- `[P]` = pode usar subagente em paralelo após as dependências, quando os testes não compartilham estado externo.
- Nenhuma senha, chave ou material derivado cruza o IPC. Erros são enums sanitizados (C-15).

## Contrato de testes que L01 deve registrar

| Camada | Tipo | Local | Comando | Paralelismo |
| --- | --- | --- | --- | --- |
| Domínio Rust puro de sessões | unitário | `src-tauri/src/sessions/**/*.rs` | `pnpm test:rust` | Seguro |
| Serviço/manager com vault e registry temporários | integração Rust | `src-tauri/tests/local_sessions.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --test local_sessions -- --test-threads=1` | Serial |
| Eventos Windows de bloqueio/suspensão | integração Windows | `src-tauri/tests/windows_sessions.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --test windows_sessions -- --test-threads=1` | Serial |
| Comandos e authority de sessões | integração IPC | `src-tauri/tests/local_sessions_commands.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --test local_sessions_commands -- --test-threads=1` | Serial |
| API/componentes de sessões Vue | unitário frontend | `src/sessions/**/*.test.ts` | `pnpm test:frontend` | Seguro |
| Jornada de sessões | E2E Tauri WebDriver | `e2e/local-sessions/**/*.e2e.ts` | `pnpm test:e2e:local-sessions` | Serial |
| Integração com secret-management (G1) | integração Rust serial | `src-tauri/tests/secret_management.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --test secret_management -- --test-threads=1` | Serial |

---

## Execution Plan

### Fase 1 — Fundação (sequencial)
```
L01 → L02
```

### Fase 2 — Domínio (paralelo após L02)
```
L02 ─┬→ L03 [P]
     ├→ L04 [P]
     └→ L05 [P]
L03 → L06
```

### Fase 3 — Manager / serviço (sequencial)
```
L03,L05,L06 → L07 → L08 → L09
L07,L04 → L10
L07,L08,L09 → L11   (impl SessionAccess — núcleo do G1)
```

### Fase 4 — Plataforma e IPC
```
L07 → L12 (Windows events)
L09,L10,L11,L12 → L13 (comandos) → L14 (authority/exit)
```

### Fase 5 — Destravamento do G1
```
L11 → L15 (troca o fake pelo SessionManager real em secret_management)
```

### Fase 6 — Frontend
```
L14 → L16 → (L17,L18,L19,L20,L21) [P] → L22
```

### Fase 7 — Aceitação
```
L14,L22,L15 → L23 → L24
```

---

## Task Breakdown

### L01 — Registrar o contrato de testes da feature
**What:** adicionar à matriz de testes as sete camadas/comandos desta feature.
**Where:** `.specs/codebase/TESTING.md`
**Depends on:** nenhum
**Reuses:** organização e regras de paralelismo existentes
**Requirements:** SESSION-01…25
**Owner/Tools:** agente principal; sem MCP

**Done when:**
- [ ] Matriz inclui domínio Rust, integração de manager, eventos Windows, IPC, frontend, E2E e a suíte de integração do G1.
- [ ] Suítes Windows/IPC/E2E marcadas como seriais; comandos futuros marcados como indisponíveis até suas tarefas introdutoras.
- [ ] `git diff --check` passa.

**Verify:** `rg -n "local_sessions|windows_sessions|test:e2e:local-sessions" .specs/codebase/TESTING.md`
**Commit:** `docs(testes): definir matriz das sessões locais`

### L02 — Declarar o módulo `sessions` e o wiring
**What:** criar módulos vazios (`sessions/{mod,model,registry,app_lock,attempts,manager,commands}.rs`) e declarar dependências (runtime async do Tauri) sem comportamento.
**Where:** `src-tauri/src/lib.rs`, `src-tauri/src/sessions/*.rs`, `src-tauri/Cargo.toml`
**Depends on:** L01
**Reuses:** layout de `secrets`, `storage`, `crypto`
**Requirements:** SESSION-23
**Owner/Tools:** agente principal; só wiring

**Done when:**
- [ ] Módulos compilam; nenhuma API de sessão exposta à WebView ainda.
- [ ] Smoke passa; contagem de testes Rust preservada.

**Verify:** `pnpm check:rust`
**Commit:** `chore(sessoes): declarar módulos da feature`

### L03 — Modelo de sessões e normalização de nome [P]
**What:** implementar `AuthMode`, `LockPolicy`, `Registry`, `SessionEntry`, normalização de nome e checagem de unicidade case-insensitive, e `SessionError`.
**Where:** `src-tauri/src/sessions/model.rs`
**Depends on:** L02
**Reuses:** `Uuid`, serde, timestamps do design
**Requirements:** SESSION-05, SESSION-06, SESSION-13, SESSION-14
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre `auth_mode` default global, normalização, unicidade, limites de `LockPolicy` (60s…∞) e "nunca".
- [ ] GREEN valida antes de qualquer persistência; `SessionError` não ecoa senha/segredo.
- [ ] `pnpm test:rust` verde.

**Verify:** `pnpm check:rust`
**Commit:** `feat(sessoes): modelar registro e políticas`

### L04 — Atraso progressivo (AttemptState) [P]
**What:** implementar backoff em memória com clock injetável, aplicável a chave global e por sessão, sem apagar dados.
**Where:** `src-tauri/src/sessions/attempts.rs`
**Depends on:** L02
**Reuses:** clock injetável (padrão do clipboard de secret-management)
**Requirements:** SESSION-18, SESSION-03
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre incremento, `next_allowed` crescente com teto, reset em sucesso e independência global vs. por sessão.
- [ ] Nunca apaga sessão/cofre.
- [ ] `pnpm test:rust` verde.

**Verify:** `pnpm check:rust`
**Commit:** `feat(sessoes): aplicar atraso progressivo`

### L05 — Orquestração do gate global (AppLock/GMP) [P]
**What:** implementar criar/desbloquear/trocar a GMP sobre `crypto::keyring`, mantendo a GMK em `Zeroizing`.
**Where:** `src-tauri/src/sessions/app_lock.rs`
**Depends on:** L02
**Reuses:** `crypto::keyring::{create_keyring, unwrap_gmk, change_gmp}`, `crypto::secret::Key32`
**Requirements:** SESSION-01, SESSION-02, SESSION-04, SESSION-17
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre 1º uso (keyring ausente), desbloqueio correto/incorreto, troca da GMP (mesma GMK), keyring corrompido/versão futura (fail-closed), força mínima.
- [ ] GMP/gKEK/GMK nunca persistem em claro; erros genéricos.
- [ ] `pnpm test:rust` verde.

**Verify:** `pnpm check:rust`
**Commit:** `feat(sessoes): orquestrar gate global da gmp`

### L06 — Persistência atômica do registry
**What:** carregar e gravar `registry.json` de forma atômica, preservando arquivo em corrupção/versão futura.
**Where:** `src-tauri/src/sessions/registry.rs`, `src-tauri/tests/local_sessions.rs`
**Depends on:** L03
**Reuses:** padrão de gravação atômica de `storage::atomic_vault`
**Requirements:** SESSION-07, SESSION-06
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED serial cobre primeira gravação, replace, leitura com app bloqueado, registro corrompido e versão futura.
- [ ] Gravação usa temp exclusivo + flush + replace no mesmo diretório.
- [ ] Gate serial verde.

**Verify:** `cargo test --manifest-path src-tauri/Cargo.toml --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): persistir registry de forma atômica`

### L07 — SessionManager: estado + ciclo da GMP
**What:** criar o `SessionManager` (registry, app_lock, unlocked, attempts) e implementar `create_global_password`, `unlock_app` (abre globais), `lock_app`, `change_global_password`.
**Where:** `src-tauri/src/sessions/manager.rs`, `src-tauri/tests/local_sessions.rs`
**Depends on:** L03, L05, L06
**Reuses:** `AppLock` (L05), registry (L06), `crypto::keys::derive_session_wrap_key`, `crypto::envelope::unlock`
**Requirements:** SESSION-01, SESSION-02, SESSION-04, SESSION-11
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED serial cobre 1º uso, desbloqueio que abre todas as globais e mantém `own` bloqueadas, `lock_app` zeroiza GMK + todas, troca de GMP.
- [ ] Estado desbloqueado guarda apenas `Key32` + metadados.
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): gerenciar ciclo da senha global`

### L08 — Criar, listar, renomear e excluir sessões
**What:** implementar `create_session` (global/own), `list_sessions` (funciona bloqueado), `rename_session` (só desbloqueada), `delete_session` (senha da sessão).
**Where:** `src-tauri/src/sessions/manager.rs`, `src-tauri/tests/local_sessions.rs`
**Depends on:** L07
**Reuses:** `crypto::envelope::create_vault`, normalização/unicidade (L03)
**Requirements:** SESSION-05, SESSION-06, SESSION-07, SESSION-08, SESSION-09
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre criação global (envolve com GMK, sem senha nova) e own (senha própria + força), payload `secrets: []` cifrado, unicidade normalizada, listar bloqueado sem segredos, renomear bloqueada rejeitada, excluir exige senha.
- [ ] Nome autenticado na AAD confere no unlock (detecta adulteração de registry).
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): criar e administrar sessões`

### L09 — Desbloqueio own, conversão de auth_mode e senha/dica
**What:** implementar `unlock_session` (apenas own, sem transitivo), `lock_session`, `lock_all`, `set_session_auth_mode` (global↔own), `change_master_password`, `reveal_hint`.
**Where:** `src-tauri/src/sessions/manager.rs`, `src-tauri/tests/local_sessions.rs`
**Depends on:** L08
**Reuses:** `crypto::envelope::{unlock, rewrap}`, `crypto::kdf::derive_kek`, AttemptState (L04)
**Requirements:** SESSION-10, SESSION-12, SESSION-13, SESSION-19, SESSION-21, SESSION-22
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre unlock own exigindo senha própria mesmo com GMP aberta, conversão nos dois sentidos exigindo senha atual apropriada, anti-rebaixamento por AAD, troca de senha com senha atual, reveal de dica.
- [ ] Conteúdo não muda na conversão; atraso progressivo aplicado.
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): desbloquear e converter sessões`

### L10 — Política de bloqueio e timer de inatividade
**What:** implementar `set_lock_policy`, `touch_session` e a tarefa periódica que bloqueia por inatividade.
**Where:** `src-tauri/src/sessions/manager.rs` (+ `lock_timer.rs` se necessário), `src-tauri/tests/local_sessions.rs`
**Depends on:** L07, L04
**Reuses:** clock injetável para teste determinístico do timer
**Requirements:** SESSION-14, SESSION-15
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre default 15 min, ajuste 60s…∞, "nunca" (sem auto-lock), reset só por `touch_session`, contagem com app minimizado, cronômetros independentes.
- [ ] Timer testável via clock injetado (sem depender de tempo real).
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): aplicar política de inatividade`

### L11 — Implementar `SessionAccess` no SessionManager (núcleo do G1)
**What:** `impl SessionAccess for SessionManager` com `read_authorized`, `read_all_authorized`, `write_authorized`, `write_two_authorized`, persistindo o envelope cifrado no commit dentro da mesma linearização de epoch/revisão.
**Where:** `src-tauri/src/sessions/manager.rs` (ou `session_access_impl.rs`), `src-tauri/tests/local_sessions.rs`
**Depends on:** L07, L08, L09
**Reuses:** trait `secrets::session_access::SessionAccess`, `crypto::envelope::rewrap`, `storage::atomic_vault::AtomicVaultWriter`
**Requirements:** SESSION-23, SESSION-24
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED cobre read/write/two autorizados, deny por `Locked`/`StaleAuthorization`, ordem de lock por UUID crescente e falha antes do commit preservando a última versão.
- [ ] Sucesso só após o envelope cifrado avançar no disco junto de epoch/revisão.
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test local_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): implementar acesso autorizado a conteúdo`

### L12 — Reação a eventos Windows de bloqueio/suspensão (best-effort)
**What:** ligar `on_windows_lock`/`on_windows_suspend` ao `SessionManager` para bloquear as sessões configuradas.
**Where:** `src-tauri/src/platform/windows/session_events.rs`, `src-tauri/tests/windows_sessions.rs`, `src-tauri/Cargo.toml`
**Depends on:** L07
**Reuses:** event pump Windows existente (`platform/windows`)
**Requirements:** SESSION-16
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED serial cobre bloqueio ao receber evento e respeito à desativação individual por sessão.
- [ ] Comportamento é best-effort documentado; fechar o app sempre bloqueia.
- [ ] Gate serial Windows verde.

**Verify:** `cargo test ... --test windows_sessions -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(windows): bloquear sessões em eventos do sistema`

### L13 — Comandos e DTOs Tauri
**What:** expor os handlers de sessão (`app_status`, `create_global_password`, `unlock_app`, `lock_app`, `change_global_password`, `list_sessions`, `create_session`, `unlock_session`, `lock_session`, `lock_all`, `rename_session`, `delete_session`, `change_master_password`, `set_session_auth_mode`, `set_lock_policy`, `touch_session`, `reveal_hint`) com DTOs fechados e `SessionError` sanitizado.
**Where:** `src-tauri/src/sessions/commands.rs`, `src-tauri/tests/local_sessions_commands.rs`
**Depends on:** L09, L10, L11, L12
**Reuses:** padrão de comandos de `secrets`/`proof`
**Requirements:** SESSION-01…22, SESSION-24, SESSION-25
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED serial cobre DTO desconhecido/oversize, revalidação de lock/epoch/modo e erros sanitizados.
- [ ] Nenhuma senha/chave sai pela WebView; `app_status`/`list_sessions` funcionam bloqueados.
- [ ] Nenhum erro serializado contém senha, dica, path ou erro cripto/Win32 bruto.
- [ ] Gate serial IPC verde (mínimo 20 testes).

**Verify:** `cargo test ... --test local_sessions_commands -- --test-threads=1` + `pnpm check:rust`
**Commit:** `feat(sessoes): expor comandos tauri de sessão`

### L14 — Authority, capabilities, estado e hook de saída
**What:** registrar `manage(SessionManager)`, `invoke_handler`, gerar permissões individuais concedidas só à janela `main` e ligar o hook de exit a `lock_all`.
**Where:** `src-tauri/src/lib.rs`, `src-tauri/build.rs`, `src-tauri/capabilities/default.json`
**Depends on:** L13
**Reuses:** `AppManifest`, capability atual, wiring do app
**Requirements:** SESSION-02, SESSION-11, SESSION-24
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED IPC prova deny para label/origin/capability não permitidos.
- [ ] WebView não recebe filesystem direto; exit bloqueia todas as sessões.
- [ ] Builds normal e security-proof compilam separados.
- [ ] `pnpm check` verde.

**Verify:** `pnpm check` + `pnpm build --no-bundle`
**Commit:** `feat(tauri): integrar authority das sessões`

### L15 — Destravar o G1: trocar o fake pelo SessionManager real
**What:** substituir `FakeSessionAccess` pelo `SessionManager` real na suíte de integração de `secret-management` e comprovar a suíte verde.
**Where:** `src-tauri/tests/secret_management.rs` (e helpers de teste compartilhados)
**Depends on:** L11
**Reuses:** suíte de integração existente de secret-management
**Requirements:** SESSION-23, SESSION-25
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] A suíte `secret_management` passa usando o `SessionManager` real (não o fake) para os cenários de CRUD/move/recovery aplicáveis.
- [ ] Deny de commit após lock/epoch preservado; última versão confirmada intacta.
- [ ] `rg` não encontra canário em logs/temporários gerados pela suíte.
- [ ] Gate serial verde.

**Verify:** `cargo test ... --test secret_management -- --test-threads=1` + `pnpm check:rust`
**Commit:** `test(sessoes): validar integração real com segredos`

### L16 — API frontend tipada de sessões
**What:** encapsular todos os `invoke` de sessão e códigos de erro em uma API tipada, sem persistir estado.
**Where:** `src/sessions/sessionsApi.ts`, `src/sessions/sessionsApi.test.ts`
**Depends on:** L14
**Reuses:** `@tauri-apps/api/core`, contrato de L13
**Requirements:** SESSION-01…22
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED verifica nomes/args dos comandos e mapeamento de erro.
- [ ] API não loga args nem persiste senha/dados.
- [ ] `pnpm test:frontend` verde (mínimo 12 testes).

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): adicionar api tipada de sessões`

### L17 — Heurística de força de senha [P]
**What:** implementar `passwordStrength.ts` (comprimento + variedade).
**Where:** `src/sessions/passwordStrength.ts`, `src/sessions/passwordStrength.test.ts`
**Depends on:** L16
**Reuses:** nenhum; heurística local
**Requirements:** SESSION-17
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED cobre faixas de força e mínimo.
- [ ] `pnpm test:frontend` verde.

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): avaliar força de senha`

### L18 — Modais de GMP: criar e desbloquear [P]
**What:** `CreateGlobalPasswordModal.vue` (1º uso) e `UnlockAppModal.vue` (gate de entrada com feedback de atraso).
**Where:** `src/sessions/CreateGlobalPasswordModal.{vue,test.ts}`, `src/sessions/UnlockAppModal.{vue,test.ts}`
**Depends on:** L16, L17
**Reuses:** `passwordStrength.ts`, controles UI existentes
**Requirements:** SESSION-01, SESSION-02, SESSION-03, SESSION-18
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED cobre 1º uso vs. desbloqueio, senha incorreta com atraso, força mínima.
- [ ] Campos de senha limpos em sucesso/unmount.
- [ ] `pnpm test:frontend` verde.

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): telas de senha global`

### L19 — Lista de sessões [P]
**What:** `SessionList.vue` mostrando sessões (bloqueada/desbloqueada), contagem e ações.
**Where:** `src/sessions/SessionList.{vue,test.ts}`
**Depends on:** L16
**Reuses:** `AppShell`/controles UI existentes
**Requirements:** SESSION-07, SESSION-11
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED cobre estados bloqueado/desbloqueado, contagem e ações de lock.
- [ ] DOM não expõe segredos.
- [ ] `pnpm test:frontend` verde.

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): listar sessões`

### L20 — Modal de criação de sessão [P]
**What:** `CreateSessionModal.vue` com nome, controle de `auth_mode` (global padrão / própria opt-out), senha própria + força + dica + aviso (só own) e política.
**Where:** `src/sessions/CreateSessionModal.{vue,test.ts}`
**Depends on:** L16, L17
**Reuses:** `passwordStrength.ts`, controles UI
**Requirements:** SESSION-05, SESSION-14, SESSION-19, SESSION-20, SESSION-22
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED cobre global (sem senha nova) vs. own (senha + força + dica + aviso), unicidade e política.
- [ ] Avisos de exposição da dica e de não-recuperação presentes.
- [ ] `pnpm test:frontend` verde.

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): criar sessão`

### L21 — Modais de desbloqueio e troca de senha [P]
**What:** `UnlockModal.vue` (own, com "Mostrar dica" e feedback de atraso) e `ChangePasswordModal.vue` (senha atual + nova + força).
**Where:** `src/sessions/UnlockModal.{vue,test.ts}`, `src/sessions/ChangePasswordModal.{vue,test.ts}`
**Depends on:** L16, L17
**Reuses:** `passwordStrength.ts`, `reveal_hint`
**Requirements:** SESSION-10, SESSION-19, SESSION-21
**Owner/Tools:** Terra; `delegate-small-tasks`, `test-driven-development`

**Done when:**
- [ ] RED cobre unlock own, "Mostrar dica", atraso e troca com senha atual.
- [ ] Senhas limpas em sucesso/unmount.
- [ ] `pnpm test:frontend` verde.

**Verify:** `pnpm check:frontend`
**Commit:** `feat(frontend): desbloquear e trocar senha`

### L22 — Integrar rotas e remover o placeholder inseguro
**What:** ligar as telas ao `App.vue`/rotas e **remover o store placeholder** (SHA-256 em `localStorage`) documentado como desvio inseguro.
**Where:** `src/App.vue`, `src/router/index.ts`, `src/stores/vault.ts` (placeholder), testes co-localizados
**Depends on:** L18, L19, L20, L21
**Reuses:** shell/estilo atuais
**Requirements:** SESSION-02, SESSION-07, SESSION-25
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED prova gate de entrada real e ausência de `localStorage`/SHA-256 como fonte de autenticação.
- [ ] `rg` não encontra o hash placeholder nos fluxos funcionais.
- [ ] `pnpm check` verde.

**Verify:** `rg -n "localStorage|sha256|SHA-256" src/sessions src/stores src/App.vue` (apenas usos não funcionais) + `pnpm check`
**Commit:** `feat(frontend): integrar sessões reais e remover placeholder`

### L23 — Jornada E2E Windows
**What:** criar a jornada serial criar GMP → reiniciar → desbloquear → criar global e own → converter → bloquear → deny pós-lock.
**Where:** `e2e/local-sessions/local-sessions.e2e.ts`, configuração dedicada, `package.json`
**Depends on:** L14, L22, L15
**Reuses:** harness WebDriver existente, sem compartilhar artefatos
**Requirements:** SESSION-01…24
**Owner/Tools:** agente principal; `test-driven-development`

**Done when:**
- [ ] RED falha antes do wiring e GREEN passa no binário normal.
- [ ] Nenhuma senha/dica em URL, console, localStorage, screenshots.
- [ ] Operações pós-lock negadas; own não abre por desbloqueio transitivo.
- [ ] Gate serial verde.

**Verify:** `pnpm test:e2e:local-sessions` + `pnpm check` + `pnpm build --no-bundle`
**Commit:** `test(sessoes): validar jornada e2e no windows`

### L24 — Fechar rastreabilidade e liberar o gate G1
**What:** registrar evidências e contagens, marcar 25/25 requisitos, atualizar STATE/ROADMAP e **remover o blocker G1** em [secret-management/tasks.md](../secret-management/tasks.md), liberando T14+.
**Where:** `tasks.md`, `spec.md`, `design.md`, `../secret-management/tasks.md`, `../../project/{ROADMAP,STATE}.md`
**Depends on:** L23
**Reuses:** IDs `SESSION-*`/`VAULT-*`, commits e gates
**Requirements:** SESSION-01…25
**Owner/Tools:** agente principal; `delegate-small-tasks` opcional para docs

**Done when:**
- [ ] Todas as tarefas Done com commit e evidência.
- [ ] Cada requisito tem teste e implementação rastreáveis.
- [ ] G1 marcado como liberado em secret-management com link para a evidência (L15).
- [ ] `git diff --check` e `pnpm check` passam; contagem final registrada.

**Verify:** `rg -n "SESSION-(0[1-9]|1[0-9]|2[0-5])" .specs/features/local-sessions/{spec,design,tasks}.md`
**Commit:** `docs(sessoes): concluir rastreabilidade e liberar g1`

---

## Task Granularity Check

| Task | Entrega dominante | Status |
| --- | --- | --- |
| L01 | 1 contrato de testes | ✅ |
| L02 | 1 boundary de módulos | ✅ |
| L03 | 1 modelo de domínio | ✅ |
| L04 | 1 mecanismo de atraso | ✅ |
| L05 | 1 orquestrador de GMP | ✅ |
| L06 | 1 persistência de registry | ✅ |
| L07 | 1 ciclo de vida da GMP no manager | ✅ Coeso |
| L08 | 1 CRUD de sessões | ✅ Coeso |
| L09 | 1 conjunto unlock/conversão | ✅ Coeso |
| L10 | 1 política de bloqueio | ✅ |
| L11 | 1 impl de porta (SessionAccess) | ✅ Coeso |
| L12 | 1 adaptador de eventos | ✅ |
| L13 | 1 superfície IPC | ✅ Coeso |
| L14 | 1 wiring de authority | ✅ Coeso |
| L15 | 1 integração de destravamento | ✅ |
| L16 | 1 API frontend | ✅ |
| L17 | 1 utilitário | ✅ |
| L18 | 2 modais coesos (GMP) | ⚠️ OK (coeso) |
| L19 | 1 componente | ✅ |
| L20 | 1 componente | ✅ |
| L21 | 2 modais coesos | ⚠️ OK (coeso) |
| L22 | 1 integração de navegação | ✅ Coeso |
| L23 | 1 jornada E2E | ✅ |
| L24 | 1 fechamento documental | ✅ Coeso |

## Requirement Traceability

| Requirement | Tasks |
| --- | --- |
| SESSION-01 | L05, L07, L13, L18 |
| SESSION-02 | L05, L07, L13, L14, L18, L22 |
| SESSION-03 | L04, L05, L13, L18 |
| SESSION-04 | L05, L07, L13 |
| SESSION-05 | L03, L08, L13, L20 |
| SESSION-06 | L03, L06, L08, L13 |
| SESSION-07 | L06, L08, L13, L19, L22 |
| SESSION-08 | L08, L13 |
| SESSION-09 | L08, L13 |
| SESSION-10 | L09, L13, L21 |
| SESSION-11 | L07, L08, L13, L14, L19 |
| SESSION-12 | L09, L13 |
| SESSION-13 | L03, L08, L09, L13 |
| SESSION-14 | L03, L10, L13, L20 |
| SESSION-15 | L10, L13 |
| SESSION-16 | L12, L13 |
| SESSION-17 | L05, L13, L17, L18, L20 |
| SESSION-18 | L04, L13, L18, L21 |
| SESSION-19 | L09, L13, L20, L21 |
| SESSION-20 | L13, L20 |
| SESSION-21 | L09, L13, L21 |
| SESSION-22 | L13, L20 |
| SESSION-23 | L02, L11, L15 |
| SESSION-24 | L11, L13, L14 |
| SESSION-25 | L13, L15, L22 |

**Coverage:** 25/25 requisitos mapeados; 24 tarefas; libera o gate G1 de secret-management na L15/L24.
