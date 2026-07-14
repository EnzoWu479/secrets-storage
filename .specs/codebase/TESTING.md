# Testing Infrastructure

**Analyzed:** 2026-07-14

## Test Frameworks

**Frontend unitário:** Vitest 4 com Vue Test Utils 2 e jsdom 29.
**Rust unitário/integração:** harness nativo do Cargo.
**E2E:** ainda não configurado; será definido quando houver um fluxo de usuário implementado.
**Coverage:** ainda não configurada.

## Test Organization

**Frontend:** testes `src/**/*.test.ts` próximos ao código exercitado.
**Rust:** testes unitários em módulos `#[cfg(test)]` e integrações futuras em `src-tauri/tests/`.

## Test Execution

- Frontend: `pnpm test:frontend`
- Rust: `pnpm test:rust`
- Gate completo: `pnpm check`
- Compile smoke Tauri: `pnpm tauri build --no-bundle`

## Test Coverage Matrix

| Code Layer | Required Test Type | Location Pattern | Run Command |
| --- | --- | --- | --- |
| Componentes Vue | unit | `src/**/*.test.ts` | `pnpm test:frontend` |
| Core Rust puro | unit | `src-tauri/src/**/*.rs` | `pnpm test:rust` |
| Comandos Tauri/IPC | integration | `src-tauri/tests/**/*.rs` | A definir antes do primeiro comando |
| Fluxos críticos de UI | e2e | A definir | A definir antes do primeiro fluxo |
| Configuração/build | none | arquivos de configuração | `pnpm build` e compile smoke Tauri |

## Parallelism Assessment

| Test Type | Parallel-Safe? | Isolation Model | Evidence |
| --- | --- | --- | --- |
| Frontend unitário | Sim | DOM jsdom novo por montagem, sem estado externo | `src/App.test.ts` |
| Rust unitário | Sim | Nenhum estado persistente implementado | `src-tauri/src/lib.rs` |
| Integração/E2E | Não definido | Ainda não existe infraestrutura | Ausência de testes desse tipo |

## Gate Check Commands

| Gate Level | When to Use | Command |
| --- | --- | --- |
| Quick | Alterações somente no frontend | `pnpm check:frontend` |
| Full | Alterações frontend e Rust | `pnpm check` |
| Build | Fechamento de fase | `pnpm check` seguido de `pnpm tauri build --no-bundle` |
