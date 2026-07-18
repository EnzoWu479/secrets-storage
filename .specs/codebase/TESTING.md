# Infraestrutura de Testes

**Analisado:** 2026-07-17
**Contrato da prova Windows/Tauri:** T01 de `windows-tauri-proof`

## Frameworks de Teste

**Frontend unitário:** Vitest 4 com Vue Test Utils 2 e jsdom 29.
**Rust unitário/integração:** harness nativo do Cargo.
**Prova Windows e Tauri:** testes de integração Cargo exclusivos de Windows, executados serialmente.
**E2E:** Tauri WebDriver, introduzido por T14; ainda não está configurado.
**PowerShell:** scripts `pwsh` com self-tests e fixtures isoladas, introduzidos por T11, T16 e T18.
**Cobertura:** ainda não configurada.

## Baseline

O baseline histórico, registrado em `STATE` em 2026-07-16, é de **53 testes Rust** e **88 testes frontend**. É uma referência histórica, não um resultado revalidado.

Em 2026-07-17, `pnpm --version` não concluiu dentro de 10 segundos. Nenhuma instalação, download de dependência ou alteração de lockfile foi tentada. Portanto, T01 não revalida o baseline; a próxima tarefa que puder executar os comandos existentes deve registrar o resultado real ou o motivo de sua indisponibilidade. Nenhuma tarefa pode reduzir silenciosamente o baseline registrado.

## Organização dos Testes

- **Frontend:** os testes ficam próximos ao código exercitado em `src/**/*.test.ts`. A UI da prova e seus testes unitários ficam especificamente em `src/security-proof/**/*.test.ts`.
- **Core Rust:** os testes unitários ficam no módulo correspondente em `src-tauri/src/**/*.rs`, sob `#[cfg(test)]`.
- **Integração Rust:** os testes de integração ficam em `src-tauri/tests/**/*.rs`.
- **Prova Windows:** `src-tauri/tests/windows_sensitive_memory.rs` e `src-tauri/tests/windows_security_proof.rs` são suítes de integração exclusivas de Windows.
- **IPC Tauri:** `src-tauri/tests/security_proof_commands.rs` é a suíte de comandos de prova.
- **Diagnósticos Tauri:** `src-tauri/tests/security_proof_diagnostics.rs` exercita o caminho controlado de falha em processo filho.
- **WebDriver:** `e2e/security-proof/**/*.e2e.ts`, com `e2e/security-proof/wdio.*`; introduzido por T14.
- **PowerShell:** os scripts ficam em `scripts/security/`, seus self-tests em `scripts/security/tests/*.tests.ps1` e fixtures autocontidas em `scripts/security/fixtures/`.
- **Laboratório manual:** o protocolo fica em `.specs/features/windows-tauri-proof/lab-protocol.md`; a evidência gerada permanece no diretório ignorado `.artifacts/security-proof/<run-id>/` e somente resumos sanitizados podem ser propostos em `.specs/features/windows-tauri-proof/evidence/`.

## Matriz de Cobertura de Testes

| Camada de código | Tipo exigido | Padrão de localização | Comando | Paralelismo e ativação |
| --- | --- | --- | --- | --- |
| Componentes Vue | unitário | `src/**/*.test.ts` | `pnpm test:frontend` | Paralelo seguro; existente |
| UI da prova | unitário frontend | `src/security-proof/**/*.test.ts` | `pnpm test:frontend` | Paralelo seguro; T13 |
| Core Rust puro | unitário | `src-tauri/src/**/*.rs` | `pnpm test:rust` | Paralelo seguro, salvo se um teste possuir estado Windows/Tauri; existente |
| Memória sensível Windows | integração Windows | `src-tauri/tests/windows_sensitive_memory.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --features security-proof --test windows_sensitive_memory -- --test-threads=1` | Serial; T05 |
| Event pump, saída e DPAPI Windows | integração Windows | `src-tauri/tests/windows_security_proof.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --features security-proof --test windows_security_proof -- --test-threads=1` | Serial; T07, T08 e T17 |
| Comandos de prova Tauri | integração IPC | `src-tauri/tests/security_proof_commands.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --features security-proof --test security_proof_commands -- --test-threads=1` | Serial; T12 |
| Diagnósticos e isolamento de panic | integração Rust | `src-tauri/tests/security_proof_diagnostics.rs` | `cargo test --manifest-path src-tauri/Cargo.toml --features security-proof --test security_proof_diagnostics -- --test-threads=1` | Serial; T15 |
| Configuração efetiva da prova | integração PowerShell | `scripts/security/tests/assert-effective-config.tests.ps1` | `pwsh -NoProfile -File scripts/security/tests/assert-effective-config.tests.ps1` | Serial; T11 |
| Authority e XSS da WebView | E2E Tauri WebDriver | `e2e/security-proof/**/*.e2e.ts` e `e2e/security-proof/wdio.*` | `pnpm test:e2e:security-proof` | Serial; introduzido por T14 (comando indisponível até então) |
| Scanner de release | integração PowerShell | `scripts/security/tests/scan-release.tests.ps1`, `scripts/security/fixtures/` | `pwsh -NoProfile -File scripts/security/tests/scan-release.tests.ps1` | Paralelo seguro somente com diretório temporário exclusivo por execução; T16 |
| Orquestrador de evidências | integração PowerShell | `scripts/security/tests/run-windows-proof.tests.ps1`, `scripts/security/fixtures/proof-results/` | `pwsh -NoProfile -File scripts/security/tests/run-windows-proof.tests.ps1` | Serial; T18 |
| Laboratório Windows | UAT/laboratório manual | `.specs/features/windows-tauri-proof/lab-protocol.md` | Sem comando não assistido; seguir o protocolo aprovado | Serial e exige autorização do usuário para ações administrativas; T20/T21 |
| Configuração/build | smoke de build | `src-tauri/Cargo.toml`, `src-tauri/build.rs`, configurações/capabilities Tauri | comandos abaixo | Modos normal e de prova são separados; a partir de T02 |

## Compile Smokes

Execute ambos os comandos a partir da raiz do repositório sempre que uma tarefa alterar features Cargo, `build.rs`, módulos de prova ou a configuração Tauri da prova:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
cargo check --manifest-path src-tauri/Cargo.toml --features security-proof
```

O segundo comando fica disponível quando T02 adicionar `security-proof`. Até então, espera-se que ele falhe porque a feature não existe; não trate essa falha esperada antes da ativação como uma prova aprovada.

## Regras de Paralelismo

1. Todo teste que inicie Tauri, possua um recurso Windows, envie mensagens Windows, invoque DPAPI ou compartilhe um diretório de artefatos da prova deve executar serialmente. Os comandos Cargo dessas suítes devem incluir `-- --test-threads=1`.
2. WebDriver executa serialmente porque inicia o binário de prova verificado e possui suas janelas e diretório de relatório.
3. Testes de fixtures do scanner PowerShell podem executar concorrentemente somente quando cada invocação receber um diretório temporário distinto. As suítes de configuração e do orquestrador de evidências executam serialmente.
4. Testes unitários frontend e testes unitários Rust puros permanecem paralelos seguros somente enquanto não adquirirem estado externo Windows/Tauri.
5. O laboratório é sempre serial; experimentos administrativos exigem aprovação no momento da execução e devem restaurar o estado alterado.

## Execução e Gates de Teste

- Frontend: `pnpm test:frontend`
- Rust: `pnpm test:rust`
- Gate frontend: `pnpm check:frontend`
- Gate Rust: `pnpm check:rust`
- Gate completo: `pnpm check`
- Smoke de build Tauri existente: `pnpm build --no-bundle`
- Compile smokes da prova: os dois comandos Cargo exatos em [Compile Smokes](#compile-smokes)
- Gate agregado futuro: `pnpm test:security-proof` (introduzido por T19; indisponível até que essa tarefa adicione o script)
- Release Windows: tag `vX.Y.Z` em commit da `main`; o workflow cria somente um draft.

Os comandos introduzidos por T11, T14, T16, T18 e T19 são nomes e caminhos contratuais. São documentados intencionalmente antes da existência de seus runners para que tarefas posteriores possam implementá-los sem alterar o contrato de testes.
