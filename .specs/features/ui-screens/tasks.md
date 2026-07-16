# Mockup Visual das Telas — Tasks

**Design:** [design.md](./design.md)
**Spec:** [spec.md](./spec.md)
**Status:** Done

> Cada componente Vue é de apresentação pura (só `<template>` + `const` de mock). "Nada funcional." O teste de cada tarefa é um **smoke test** de montagem (monta sem erro), conforme a matriz de cobertura (componentes Vue → `unit`, parallel-safe, gate `quick`). Todos os arquivos vivem em `src/mockup/` e **não** são montados por `main.ts`.

## Execution Results

- **Concluído em:** 2026-07-15
- **Tarefas:** T1–T28 concluídas; nenhuma parcial ou bloqueada
- **Frontend:** 27 arquivos de teste, 85 testes aprovados, `vue-tsc --noEmit` e build Vite aprovados
- **Rust/Tauri:** `cargo fmt`, Clippy, testes Rust e `tauri build --no-bundle` aprovados
- **Auditoria:** três revisões paralelas de conformidade concluídas; desvios encontrados foram corrigidos por TDD
- **UAT visual automatizada:** não executada porque o navegador integrado falhou ao inicializar; seleção de tela e tema do viewer permanecem cobertos por testes unitários

---

## Execution Plan

### Phase 1 — Foundation (Sequential)

```
T1 (tokens)
```

### Phase 2 — Átomos + fixtures (Parallel)

```
        ┌→ T2 UiButton [P]
        ├→ T3 UiInput [P]
        ├→ T4 UiCard [P]
T1 ─────┼→ T5 UiBadge [P]
        ├→ T6 UiToggle [P]
        ├→ T7 PasswordStrength [P]
        └→ T8 UiIcon [P]
(sem dep) → T9 fixtures.ts [P]
```

### Phase 3 — Compostos (Parallel)

```
T5, T8 ───────────────→ T10 AppShell [P]
T2..T8 ───────────────→ T11 StyleGuide [P]
```

### Phase 4 — Telas T01–T16 (Parallel)

```
Átomos (T2..T8) ┐
AppShell (T10) ─┼→ T12..T27 [P]  (cada tela depende só dos componentes que usa)
fixtures (T9) ──┘
```

### Phase 5 — Viewer (Sequential)

```
T11 + T12..T27 → T28 MockupShell
```

---

## Task Breakdown

### T1: Definir design tokens no `@theme`
**What:** adicionar tokens de cor (escuro + `.theme-light`), fontes e raios em `src/style.css`, expondo utilitários (`bg-app`, `text-secondary`, `border-divider`, `text-accent`, `font-mono`, etc.).
**Where:** `src/style.css` (modify)
**Depends on:** None
**Reuses:** `@theme` já existente em `src/style.css`
**Requirement:** spec §Identidade Visual
**Tools:** Edit · Skill: NONE
**Done when:**
- [ ] Tokens escuro em `:root` e claro em `.theme-light` conforme design.md
- [ ] `--font-sans`/`--font-mono` com fallback Windows (Segoe UI / Cascadia Code)
- [ ] Gate check passes: `pnpm check:frontend`
**Tests:** none (camada configuração/build — matriz)
**Gate:** quick

---

### T2: UiButton.vue [P]
**What:** botão com 4 variantes visuais (primary/secondary/danger/ghost) + estados hover/focus/disabled.
**Where:** `src/mockup/components/UiButton.vue` (+ `UiButton.test.ts`)
**Depends on:** T1
**Reuses:** tokens (T1), convenção `<script setup>` de `src/App.vue`
**Requirement:** design §Componentes base
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] 4 variantes renderizam com as cores dos tokens
- [ ] Anel de foco accent visível; disabled usa `muted`
- [ ] Smoke test monta o componente sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T3: UiInput.vue [P]
**What:** campo com label, ajuda, variante password (ícone-olho visual) e variante erro.
**Where:** `src/mockup/components/UiInput.vue` (+ test)
**Depends on:** T1
**Reuses:** tokens (T1)
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Label 14/500, input h-10 radius-control, ajuda 12 secondary
- [ ] Variantes texto/password/erro por classe (sem lógica)
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T4: UiCard.vue [P]
**What:** container de superfície (radius-card, padding 20, borda escuro / sombra claro) com slots.
**Where:** `src/mockup/components/UiCard.vue` (+ test)
**Depends on:** T1
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Slot default + slots header/footer
- [ ] Borda no escuro, sombra no claro
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T5: UiBadge.vue [P]
**What:** pill de estado (11px micro) nos tons neutro/accent-soft/success/warning/danger.
**Where:** `src/mockup/components/UiBadge.vue` (+ test)
**Depends on:** T1
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] 5 tons de badge conforme design
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T6: UiToggle.vue [P]
**What:** switch visual (trilho 40x22) nos estados ligado/desligado.
**Where:** `src/mockup/components/UiToggle.vue` (+ test)
**Depends on:** T1
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Ligado = bg accent; desligado = divider
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T7: PasswordStrength.vue [P]
**What:** indicador de 4 segmentos + rótulo (Fraca/Média/Boa/Forte).
**Where:** `src/mockup/components/PasswordStrength.vue` (+ test)
**Depends on:** T1
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] 4 níveis com as cores danger/warning/accent/success
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T8: UiIcon.vue [P]
**What:** conjunto de ícones inline SVG (`currentColor`, stroke ~1.75px): cadeado, olho, copiar, Google, Google Drive, senha/api/token/nota/ssh, engrenagem, sync, aviso.
**Where:** `src/mockup/components/UiIcon.vue` (+ test)
**Depends on:** T1
**Reuses:** nada externo (CSP/offline — sem libs de ícone)
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Todos os nomes de ícone da spec renderizam via SVG inline
- [ ] Cor herda de `currentColor`
- [ ] Smoke test monta ao menos 3 ícones sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T9: fixtures.ts [P]
**What:** constantes de mock tipadas (sessões, segredos de exemplo por tipo) para reduzir divergência entre telas.
**Where:** `src/mockup/fixtures.ts`
**Depends on:** None
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] `SESSIONS` e amostras dos 5 tipos de segredo exportadas como `const` (sem lógica)
- [ ] `vue-tsc --noEmit` sem erros de tipo
- [ ] Gate check passes: `pnpm check:frontend`
**Tests:** none (módulo de dados estáticos; sem linha na matriz)
**Gate:** quick

---

### T10: AppShell.vue [P]
**What:** layout de produto — sidebar 240px (sessões fictícias + rodapé de ícones) + área principal com slot.
**Where:** `src/mockup/components/AppShell.vue` (+ test)
**Depends on:** T5, T8
**Reuses:** UiBadge (T5), UiIcon (T8), fixtures (T9)
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Sidebar 240px com item ativo destacado + rodapé (Sync/Config/Sobre)
- [ ] Slot da área principal com padding 24–32px
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### T11: StyleGuide.vue [P]
**What:** vitrine da paleta (escuro/claro), tipografia e todos os componentes base em seus estados.
**Where:** `src/mockup/StyleGuide.vue` (+ test)
**Depends on:** T2, T3, T4, T5, T6, T7, T8
**Reuses:** todos os átomos
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Seções de paleta, tipografia e componentes visíveis
- [ ] Smoke test monta sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick

---

### Telas (Phase 4) — cada tela: `src/mockup/screens/<arquivo>.vue` (+ test), Tests: unit, Gate: quick

Padrão do **Done when** de cada tela: renderiza o conteúdo e **todas as variações de estado** da spec (lado a lado, sem JS); smoke test monta sem erro; `pnpm check:frontend` passa; +1 test.

### T12: T01_LoginGoogle.vue [P]
**Depends on:** T2, T8 · **Requirement:** spec T01
Layout centralizado; marca + "Continuar com Google" + aviso zero-knowledge. **Não** desenhar a página do Google.

### T13: T02_Connecting.vue [P]
**Depends on:** T2, T8 · **Requirement:** T02
Estado "Abrindo o Google…" + variação erro (cancelado).

### T14: T03_CreateGlobalPassword.vue [P]
**Depends on:** T2, T3, T4, T7, T8 · **Requirement:** T03
Criar senha mestra **global** (1º uso): senha+força, confirmação, dica (aviso) + card âmbar "sem recuperação".

### T15: T04_UnlockApp.vue [P]
**Depends on:** T2, T3, T4, T8 · **Requirement:** T04
Desbloquear app com a senha global. 3 variações: padrão + dica revelada + atraso pós-erro.

### T16: T05_Welcome.vue [P]
**Depends on:** T2, T4, T8 · **Requirement:** T05
Cofre vazio (senha global já existe) + convite a criar a primeira sessão.

### T17: T06_SessionsList.vue [P]
**Depends on:** T10, T4, T5, T8 · **Requirement:** T06
Cards com badge Global/Senha própria + estado; globais desbloqueadas, próprias bloqueadas. Botão "Bloquear app".

### T18: T07_CreateSession.vue [P]
**Depends on:** T2, T3, T6, T7, T8 · **Requirement:** T07
Nome + **toggle "Usar senha própria"**; 2 variações: desligado (usa a global) e ligado (senha+força+dica). Política de inatividade + toggles Windows.

### T19: T08_UnlockSession.vue [P]
**Depends on:** T2, T3, T4, T8 · **Requirement:** T08
Desbloquear sessão **com senha própria** (as globais já abrem com o app). 3 variações: padrão + dica + atraso.

### T20: T09_SecretsList.vue [P]
**Depends on:** T10, T3, T5, T8 · **Requirement:** T09
Busca + lista por tipo + variação cofre vazio.

### T21: T10_SecretDetail.vue [P]
**Depends on:** T10, T2, T5, T8 · **Requirement:** T10
Um exemplo por tipo (valores em mono, ocultos/revelados) + toast de cópia "limpar em 05:00".

### T22: T11_SecretForm.vue [P]
**Depends on:** T10, T2, T3, T8 · **Requirement:** T11
Seletor de tipo + campos do tipo Senha (opcional: Chave SSH).

### T23: T12_SessionSettings.vue [P]
**Depends on:** T10, T2, T3, T6, T8 · **Requirement:** T12
Renomear, política, clipboard, **seção Autenticação** (definir senha própria / voltar à global), somente leitura, zona de perigo.

### T24: T13_Sync.vue [P]
**Depends on:** T10, T2, T5, T8 · **Requirement:** T13
Provedor conectado + estados sincronizado/enviando/offline por sessão.

### T25: T14_ConflictResolution.vue [P]
**Depends on:** T10, T2, T5, T8 · **Requirement:** T14
Banner de expiração + 2 campos Local vs. Remoto com 3 ações cada.

### T26: T15_AppUpdate.vue [P]
**Depends on:** T10, T2, T8 · **Requirement:** T15
3 variações: disponível / verificando / erro (assinatura inválida).

### T27: T16_GeneralSettings.vue [P]
**Depends on:** T10, T2, T6, T8 · **Requirement:** T16
**Trocar senha mestra global** + "Bloquear app agora" + aviso "sem recuperação"; aparência (toggle tema), telemetria desativada, Sobre.

---

### T28: MockupShell.vue
**What:** galeria navegável (andaime) — lista T01–T16 + StyleGuide e renderiza a seleção; toggle claro/escuro aplicando `.theme-light` no root.
**Where:** `src/mockup/MockupShell.vue` (+ test)
**Depends on:** T11, T12, T13, T14, T15, T16, T17, T18, T19, T20, T21, T22, T23, T24, T25, T26, T27
**Reuses:** todas as telas + StyleGuide
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Lista de 17 entradas (16 telas + StyleGuide); seleção troca a tela exibida
- [ ] Toggle de tema alterna `.theme-light` no elemento raiz
- [ ] Smoke test monta e alterna ao menos 1 tela sem erro
- [ ] Gate check passes: `pnpm check:frontend`
- [ ] Test count: +1 test passa
**Tests:** unit · **Gate:** quick
**Commit:** `feat(ui): adiciona mockup visual navegável das telas T01–T16`

---

## Pre-Approval Validation

### Check 1 — Task Granularity

| Task | Escopo | Status |
| --- | --- | --- |
| T1 | 1 arquivo (tokens) | ✅ |
| T2–T8 | 1 componente base cada | ✅ |
| T9 | 1 arquivo de dados | ✅ |
| T10, T11 | 1 componente cada | ✅ |
| T12–T27 | 1 tela cada | ✅ |
| T28 | 1 componente (viewer) | ✅ |

### Check 2 — Diagram ↔ Definition Cross-Check

| Task | Depends on (corpo) | Diagrama mostra | Status |
| --- | --- | --- | --- |
| T2–T8 | T1 | T1 → átomos | ✅ |
| T9 | None | (sem dep) | ✅ |
| T10 | T5, T8 | T5,T8 → T10 | ✅ |
| T11 | T2..T8 | T2..T8 → T11 | ✅ |
| T12–T27 | subconjunto de {T2..T8, T10} | átomos/AppShell → telas | ✅ |
| T28 | T11, T12..T27 | T11 + telas → T28 | ✅ |

Nenhum par `[P]` depende de outro na mesma fase. ✅

### Check 3 — Test Co-location Validation

| Task | Camada criada | Matriz exige | Task diz | Status |
| --- | --- | --- | --- | --- |
| T1 | Configuração/build (`style.css`) | none | none | ✅ |
| T2–T8, T10, T11 | Componentes Vue | unit | unit | ✅ |
| T9 | Módulo de dados TS (sem linha na matriz) | — | none | ✅ |
| T12–T27 | Componentes Vue | unit | unit | ✅ |
| T28 | Componente Vue | unit | unit | ✅ |

Sem violações. Cada componente escreve seu smoke test na própria tarefa (sem deferimento).

---

## Parallel Execution Map

```
Phase 1:  T1
Phase 2:  T2 T3 T4 T5 T6 T7 T8 T9        (todos [P])
Phase 3:  T10 T11                        ([P] entre si)
Phase 4:  T12 T13 T14 T15 T16 T17 T18 T19
          T20 T21 T22 T23 T24 T25 T26 T27  (todos [P])
Phase 5:  T28
```

**Execução:** delegar cada `[P]` a um sub-agente (um por tarefa) por fase; o orquestrador aguarda a fase fechar antes de avançar. Frontend unitário é parallel-safe (jsdom novo por montagem — TESTING.md).

---

## Tools / MCPs / Skills

- **Todas as tarefas:** ferramentas de arquivo (Write/Edit). MCP: NONE. Skill: NONE.
- Opcional: gerar o visual primeiro no **Claude Design** e depois transcrever para os `.vue` desta quebra — não altera as dependências nem os gates.
