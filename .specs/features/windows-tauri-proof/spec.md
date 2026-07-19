# Prova de Integração Windows e Tauri — Especificação

**Milestone:** M0 — Fundação de segurança
**Status:** Pausada em 2026-07-19 após T14; T15–T22 permanecem pendentes

## Problem Statement

O núcleo criptográfico candidato existe, mas ainda não há evidência reproduzível de que a integração real entre Windows, Tauri, Rust e WebView preserve suas fronteiras de segurança. Antes de conectar o cofre funcional ao backend, o projeto precisa demonstrar como material sensível é contido no core, descartado em bloqueios e eventos do sistema, excluído de logs e artefatos e protegido contra autoridade excessiva da WebView.

Esta feature produz protótipos, testes e procedimentos de laboratório para os controles C-10 a C-13, C-15 e C-19 do modelo de ameaças, cobrindo principalmente PT-04, PT-05, PT-07, PT-09 e a avaliação exploratória de PT-06/PT-13.

## Goals

- [ ] Demonstrar que a WebView possui autoridade mínima e não recebe chaves, tokens ou plaintext devolvidos pelo core Rust.
- [ ] Demonstrar um ciclo de vida fail-closed para material sensível diante de bloqueio manual, lock/suspend/resume do Windows e encerramento do aplicativo.
- [ ] Produzir evidência reproduzível de zeroização best-effort, tratamento de falha ao impedir paginação e limites honestos contra inspeção de memória.
- [ ] Verificar que logs, panic, dumps controlados e bundles de release não incluem canários sensíveis, credenciais ou chaves privadas.
- [ ] Determinar o papel aceitável de DPAPI/TPM como defesa opcional, sem torná-los a única chave nem impedir portabilidade do cofre.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Parâmetros finais de KDF e AEAD (PT-01/PT-02) | Permanecem sujeitos à revisão criptográfica independente. |
| Orquestração completa de sessões e comandos de produção (PT-03) | Pertence à feature `local-sessions`; esta prova usa apenas harnesses mínimos. |
| Clipboard (PT-08) | Será validado junto ao gerenciamento de segredos. |
| OAuth, sincronização e gravação atômica (PT-10 a PT-12) | Dependem de features posteriores. |
| Validação completa do updater hostil (PT-14) | Pertence ao hardening de distribuição; aqui só se inspeciona o bundle por material proibido. |
| Fuzzing geral de formatos e IPC (PT-15) | Será uma feature própria de hardening. |
| Proteção contra administrador, driver, firmware ou malware ativo | Fora do modelo; os controles apenas reduzem exposição acidental e residual. |

---

## User Stories

### P1: Provar a fronteira WebView → core Rust ⭐ MVP

**User Story:** Como mantenedor de segurança, quero uma fronteira IPC mínima e testável para que uma WebView comprometida não adquira autoridade genérica nem receba material criptográfico.

**Why P1:** Qualquer integração funcional futura depende de a WebView ser tratada como não confiável.

**Acceptance Criteria:**

1. WHEN o aplicativo é compilado para release THEN o sistema SHALL carregar somente conteúdo local, aplicar CSP sem CDN nem `unsafe-eval`, desabilitar DevTools e conceder apenas capabilities/plugins explicitamente necessários.
2. WHEN a WebView invoca um comando permitido THEN o core SHALL validar a operação, o estado de autorização, os identificadores e os limites antes de executá-la.
3. WHEN a WebView invoca comando desconhecido, não autorizado ou com entrada fora dos limites THEN o sistema SHALL negar a operação sem panic, efeito privilegiado ou detalhe sensível.
4. WHEN uma senha entra por um comando estreito de teste THEN o core SHALL consumi-la sem devolver senha, chave derivada, GMK, root key ou plaintext em resposta, evento ou erro.
5. WHEN um cenário de XSS controlado executa no harness THEN ele SHALL ser incapaz de ampliar capabilities, acessar APIs não permitidas ou obter material sensível do core.

**Independent Test:** Executar o harness IPC com chamadas permitidas, negadas, malformadas e um payload XSS controlado; conferir respostas allowlisted, ausência de efeitos proibidos e auditoria da configuração final.

### P1: Provar o ciclo de vida da memória sensível ⭐ MVP

**User Story:** Como usuário, quero que chaves mantidas para uso sejam descartadas ao bloquear ou fechar o aplicativo para reduzir sua exposição residual em memória.

**Why P1:** O cofre não pode ser considerado bloqueado se o material de abertura continuar acessível no processo.

**Acceptance Criteria:**

1. WHEN material sensível é criado no core THEN ele SHALL permanecer em tipos de vida curta, sem `Debug`/`Display` revelador e com zeroização no descarte.
2. WHEN o coordenador de bloqueio bloqueia o aplicativo THEN ele SHALL descartar GMK e todas as chaves de sessões antes de permitir nova operação privilegiada.
3. WHEN a tentativa de impedir paginação de uma região sensível falha THEN o sistema SHALL tratar e registrar somente o estado não sensível da falha, seguindo uma política documentada sem assumir proteção inexistente.
4. WHEN o harness encerra o ciclo de vida de um canário sensível THEN a evidência de teste SHALL comprovar a zeroização observável no buffer controlado, sem alegar erradicação de cópias feitas pelo compilador, sistema operacional ou WebView.
5. WHEN um segredo é digitado na WebView THEN a UI SHALL limitar sua retenção ao menor ciclo prático e removê-lo do estado/DOM ao concluir, cancelar ou bloquear a operação.

**Independent Test:** Executar testes Rust instrumentados de criação, bloqueio e descarte; inspecionar o harness WebView antes e depois de concluir/cancelar/bloquear.

### P1: Reagir a eventos críticos do Windows ⭐ MVP

**User Story:** Como usuário Windows, quero que o aplicativo se bloqueie em eventos críticos para que sessões não permaneçam abertas após lock, suspensão, hibernação ou encerramento.

**Why P1:** O modelo de ameaças classifica a perda ou o atraso desses eventos como risco alto.

**Acceptance Criteria:**

1. WHEN o Windows sinaliza lock, sleep, hibernação, shutdown ou suspensão crítica e há tempo de processamento THEN o aplicativo SHALL acionar o mesmo coordenador fail-closed usado pelo bloqueio manual.
2. WHEN o aplicativo recebe resume ou detecta que atravessou uma transição do sistema THEN ele SHALL permanecer bloqueado até nova autenticação, mesmo se o evento anterior tiver sido perdido.
3. WHEN o processo inicia ou encerra THEN o estado em memória SHALL começar bloqueado e SHALL descartar material sensível antes do encerramento normal.
4. WHEN uma sessão futura tiver uma política explícita que permita permanecer aberta em lock/suspend THEN a exceção SHALL ser individual, desativada por padrão e não SHALL impedir o bloqueio global no encerramento.
5. WHEN um evento crítico não concede tempo suficiente para limpeza THEN a documentação SHALL registrar o limite e recomendar hibernação/desligamento com criptografia de disco para ameaça física avançada.

**Independent Test:** Rodar uma matriz Windows com lock/unlock, sleep/resume, hibernação/resume e encerramento, registrando somente transições de estado não sensíveis e confirmando a negação de operações após cada retorno.

### P1: Excluir segredos de diagnóstico e distribuição ⭐ MVP

**User Story:** Como mantenedor, quero testar canários contra logs, falhas e artefatos para evitar que o próprio diagnóstico ou pacote distribua material sensível.

**Why P1:** Logs, WER, panic e bundles são canais persistentes fora da proteção do cofre.

**Acceptance Criteria:**

1. WHEN comandos e harnesses processam canários de senha, chave e token THEN stdout, stderr, logs estruturados e mensagens de erro SHALL conter somente campos allowlisted e nunca os canários.
2. WHEN o harness força panic e falhas controladas THEN mensagens, backtraces e artefatos de diagnóstico coletados SHALL ser verificados automaticamente ou por procedimento reproduzível contra os canários.
3. WHEN um bundle Windows de release é produzido THEN uma varredura SHALL falhar se encontrar chaves privadas, secrets de CI, tokens-canário, credenciais, arquivos `.env`, source maps não autorizados ou configuração de DevTools.
4. WHEN o updater é configurado no pipeline THEN somente a chave pública e endpoints esperados SHALL aparecer no bundle; a chave privada SHALL existir apenas no ambiente protegido de release.
5. WHEN a evidência não puder inspecionar uma classe de dump ou armazenamento do sistema THEN o relatório SHALL marcá-la como não verificada, sem promover o controle a mitigado.

**Independent Test:** Produzir um bundle de release de teste com canários injetados apenas no harness, executar o scanner e os cenários de falha e revisar o relatório de ausência/presença.

### P2: Avaliar pagefile, hibernação e DPAPI/TPM

**User Story:** Como mantenedor de segurança, quero medir as garantias reais do Windows e documentar seus limites para não transformar defesa em profundidade em promessa de recuperação ou proteção absoluta.

**Why P2:** A prova melhora o hardening, mas não deve bloquear a arquitetura portável nem substituir a senha mestra.

**Acceptance Criteria:**

1. WHEN o laboratório executa com e sem BitLocker nos ambientes disponíveis THEN o procedimento SHALL procurar canários em pagefile, imagem de hibernação e dumps após uso, bloqueio e crash, registrando método, permissões e limitações.
2. WHEN DPAPI ou TPM protege material local opcional THEN esse material SHALL ser específico do dispositivo e não SHALL ser a única chave necessária para abrir um backup válido em outro dispositivo.
3. WHEN DPAPI/TPM está indisponível, muda o perfil do Windows ou falha THEN o aplicativo SHALL falhar de forma explícita sem corromper o cofre nem criar uma recuperação implícita.
4. WHEN malware executa como o mesmo usuário THEN a documentação SHALL declarar que DPAPI/TPM pode permitir operações autorizadas e não elimina essa ameaça.

**Independent Test:** Executar a matriz suportada de disponibilidade, perfil e migração; validar que o blob portátil continua dependente da senha mestra e que falhas locais não o alteram.

---

## Edge Cases

- WHEN o aplicativo recebe eventos Windows duplicados ou fora de ordem THEN o coordenador SHALL manter o estado bloqueado e idempotente.
- WHEN lock e comando privilegiado concorrem THEN o lock SHALL vencer antes de qualquer resposta com dados protegidos.
- WHEN um input IPC excede tamanho, profundidade ou formato permitido THEN ele SHALL ser rejeitado antes de alocação descontrolada.
- WHEN a zeroização ou o page lock não pode ser observado de forma confiável pelo harness THEN o resultado SHALL ser “inconclusivo”, nunca “aprovado por ausência”.
- WHEN o scanner do bundle não reconhece um formato compactado THEN ele SHALL falhar ou sinalizar cobertura incompleta.
- WHEN o teste depende de edição/versão específica do Windows THEN a evidência SHALL registrar versão, build, arquitetura e estado do BitLocker.

---

## Requirement Traceability

| Requirement ID | Story | Threat model | Status |
| --- | --- | --- | --- |
| WINT-01 | Fronteira IPC | C-10, C-11, T-IPC-01/02, PT-09 | Designed |
| WINT-02 | Autorização backend | C-10, T-AUTH-04/05 | Designed |
| WINT-03 | Inputs IPC defensivos | C-17, T-IPC-01 | Designed |
| WINT-04 | Ausência de material na resposta | C-10, C-15, T-MEM-05 | Designed |
| WINT-05 | Ciclo de vida/zeroização | C-12, T-MEM-01/02, PT-04 | Designed |
| WINT-06 | Falha de page lock | C-12, C-19, PT-04 | Designed |
| WINT-07 | Eventos Windows | C-13, T-MEM-06, PT-05 | Designed |
| WINT-08 | Lock fail-closed no resume/exit | C-13, T-PHYS-01, PT-05 | Designed |
| WINT-09 | Logs e erros com allowlist | C-15, T-LOG-01, PT-07 | Designed |
| WINT-10 | Panic, WER e dumps | C-15, T-MEM-03, PT-07 | Designed |
| WINT-11 | Scanner do bundle | C-11, C-15, C-16, PT-09 | Designed |
| WINT-12 | Pagefile/hibernação | C-12, C-19, T-MEM-02, PT-06 | Designed |
| WINT-13 | DPAPI/TPM opcional e portável | C-19, PT-13 | Designed |

**Coverage:** 13 requisitos totais, 13 mapeados ao design, 0 mapeados a tarefas.

## Success Criteria

- [ ] Os cenários P1 produzem comandos, scripts ou procedimentos reproduzíveis com resultado pass/fail/inconclusivo.
- [ ] Cada resultado identifica a versão do Windows, modo debug/release, arquitetura e limitações da observação.
- [ ] Nenhum canário sensível aparece nas saídas allowlisted ou no bundle aprovado.
- [ ] Após cada transição crítica testada, operações privilegiadas permanecem negadas até nova autenticação.
- [ ] Capabilities, CSP, plugins e DevTools do bundle final possuem inventário auditável e mínimo.
- [ ] PT-04, PT-05, PT-07 e PT-09 recebem evidência suficiente para atualizar o modelo de ameaças sem alegações além do que foi observado.
- [ ] PT-06 e PT-13 terminam com veredito documentado: aprovado, bloqueado ou inconclusivo com próximo experimento.
