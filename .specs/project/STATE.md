# State

**Last Updated:** 2026-07-19
**Current Work:** M1 `secret-management` com **Spec, Design e 24 Tasks aprovados** em 2026-07-19; T01–T03 concluídas e próxima tarefa disponível T05 (`SessionAccess`). O core possui agora os módulos da feature e o modelo tipado/validado, com 93 testes Rust verdes. G1 explicita que IPC de produção e E2E dependem de `local-sessions`; modelo, codec e serviços podem avançar antes com um fake determinístico. O store frontend-only não pode virar fonte de verdade. O modelo de ameaças segue em revisão por AD-022; `windows-tauri-proof` está pausada após T14.

---

## Recent Decisions (Last 60 days)

### AD-029: Pausa da prova Windows/Tauri após o gate de authority (2026-07-19)

**Decision:** Encerrar a execução corrente de `windows-tauri-proof` após T14 e seguir para outra spec.
**Reason:** Decisão explícita do usuário de interromper esta feature neste ponto.
**Trade-off:** Authority/ACL/XSS possuem cobertura automatizada, mas panic/saídas, scanner de release, DPAPI, orquestração, CI, laboratório e consolidação final (T15–T22) continuam pendentes.
**Impact:** A feature não deve ser tratada como completa; uma retomada começa em T15.

### AD-028: Remoção do Graphify do fluxo do projeto (2026-07-17)

**Decision:** Abandonar o Graphify neste repositório, removendo a regra obrigatória do `AGENTS.md`, a skill local versionada, os artefatos do grafo e os ignores específicos.
**Reason:** Decisão explícita do usuário de não continuar usando o Graphify no projeto.
**Trade-off:** O projeto deixa de manter um grafo de conhecimento navegável e passa a depender diretamente do código, das specs e das ferramentas normais de busca para exploração.
**Impact:** Tarefas futuras não devem executar nem exigir atualização do Graphify. Instalações globais permanecem fora do escopo e não foram alteradas.

### AD-027: Fases 4–5 do backend cripto — vetores e plano de revisão; fatia completa (2026-07-16)

**Decision:** Concluídas as duas últimas fases do `crypto-format`, fechando a fatia. **T7 `crypto::vectors`** (`#[cfg(test)]`): vetores golden determinísticos (entradas fixas → saídas hex esperadas para gKEK/KEK/K_sessao/content_key/AEAD e os envelopes serializados completos), recuperação e adulteração (1 bit ⇒ rejeição autenticada). **T8 `review-plan.md`**: escopo/entregáveis/critérios da revisão criptográfica independente. Gate `pnpm check:rust` verde (53 testes).
**Reason:** Usuário pediu para "continuar fazendo todas as fases até o fim".
**Trade-off / desvios anotados:** (1) Os vetores golden foram capturados executando o próprio código com entradas fixas e fixados como constantes de regressão — dependem de PT-01/PT-02 e do layout CBOR e mudarão se estes forem revisados (comportamento esperado). (2) Adulteração no nível dos vetores inverte 1 bit do último byte (tag) do envelope serializado; a mutação campo-a-campo do header já é coberta nos testes de `keyring`/`envelope`.
**Impact / gate aberto:** A fatia Sessões + desbloqueio está **implementada e testada como candidato**. **Continua NÃO fechando o gate D-05**: `review-plan.md` define o caminho para a revisão independente que precede a promoção a base estável e a fixação de PT-01/PT-02.

### AD-026: Fase 3 do backend cripto — envelopes `keyring`/`envelope` (2026-07-16)

**Decision:** Implementadas as duas tarefas da Fase 3 do `crypto-format`, com gate `pnpm check:rust` verde (39 testes). **T5 `crypto::keyring`:** keyring global CBOR (`create_keyring`/`unwrap_gmk`/`change_gmp`) — gKEK←Argon2id(GMP,salt_global) envolve a GMK aleatória; troca de GMP reenvolve a mesma GMK. **T6 `crypto::envelope`:** cofre de sessão CBOR versionado (`create_vault`/`unlock`/`rewrap`) com wrap condicional por `auth_mode` (own=Argon2id, global=HKDF(GMK,uuid)), fail-closed em versão superior e preservação de campos desconhecidos. Infra `crypto::codec` (helpers `to_cbor`/`from_cbor` + DTOs `KdfDescriptor`/`WrapField`).
**Reason:** Dar prosseguimento ao plano aprovado em `tasks.md` (Fase 3 após a Fase 2).
**Trade-off / desvios anotados:** (1) Header carregado como **bytes CBOR opacos** dentro do envelope, usado diretamente como AAD — garante AAD estável e forward-compat de campos desconhecidos (FMT-03) sem re-serialização. (2) Em `auth_mode = global`, os campos `salt`/`kdf` do header são placeholder (zeros + candidato), autenticados mas não usados. (3) `rewrap` reautentica (re-sela) o payload sob o novo header, pois a AAD é o header completo. (4) `secrets` fica como `Vec<ciborium::value::Value>` para não fixar o modelo de segredos.
**Impact / gate aberto:** Continua implementando o design **candidato**; **não** fecha o gate D-05. Falta a Fase 4 (vetores T7) e a Fase 5 (plano de revisão T8). Parâmetros PT-01/PT-02 seguem provisórios.

### AD-025: Fase 2 do backend cripto — primitivos `kdf`/`keys`/`aead` (2026-07-16)

**Decision:** Implementadas as três tarefas paralelas da Fase 2 do `crypto-format`, com gate `pnpm check:rust` verde (fmt + clippy `-D warnings` + 21 testes). **T2 `crypto::kdf`:** `derive_kek` via Argon2id + `KdfParams` com `validate()` que rejeita fora dos limites defensivos (`MIN/MAX_*`) antes de alocar; candidato 64 MiB/3/1 (`⚠️ PT-01`). **T3 `crypto::keys`:** `generate_root_key`, `derive_content_key(epoch)`, `derive_session_wrap_key(uuid)` via HKDF-SHA256 com rótulos `ssv:content:v1:` / `ssv:session-wrap:v1:`. **T4 `crypto::aead`:** `seal`/`open` (XChaCha20-Poly1305, nonce 24 bytes, AAD) + `wrap_key`/`unwrap_key` sobre `Key32`.
**Reason:** Dar prosseguimento ao plano aprovado em `tasks.md` (Fase 2 é o passo seguinte a T1).
**Trade-off / desvios anotados:** (1) `seal`/`wrap_key` mantêm assinatura infalível (`-> Vec<u8>`) do design, com `expect` justificado (encrypt só falha além do limite de tamanho da cifra). (2) `unwrap_key` zeroiza o buffer intermediário do material desenvolvido. (3) Ambiente de build Linux exigiu libs GTK do sistema (`libgtk-3-dev` etc.) para o `cargo test` do crate Tauri — não afeta o código.
**Impact / gate aberto:** Continua implementando o design **candidato**; **não** fecha o gate D-05 (modelo de ameaças reaberto por AD-022). Parâmetros PT-01/PT-02 seguem provisórios. Próximo: Fase 3 (`keyring`/`envelope`).

### AD-024: Início do backend criptográfico (`crypto-format`) (2026-07-15)

**Decision:** Iniciar a implementação Rust do formato criptográfico versionado (fatia Sessões + desbloqueio), a partir do `crypto-format/design.md`. Criado `crypto-format/tasks.md` (8 tarefas, 5 fases): T1 fundação; T2/T3/T4 `kdf`/`keys`/`aead`; T5/T6 `keyring`/`envelope`; T7 vetores; T8 plano de revisão. **T1 concluída** (módulo `crypto`, `CryptoError`, `Key32` zeroizável; 10 crates candidatas no `Cargo.toml`; compila, 1 teste verde).
**Reason:** Usuário escolheu "começar backend cripto (M0)" para dar prosseguimento ao roadmap.
**Trade-off / desvios anotados:** (1) `Key32` foi para a fundação (não em `keys` como no design) para desacoplar os primitivos e permitir paralelismo — desvio mínimo. (2) Randomness injetável por parâmetro no núcleo (produção usa `getrandom`), para vetores determinísticos (T7). (3) Params Argon2id/nonce entram como **candidatos** `⚠️ PT-01/PT-02`, não finais.
**Impact / gate aberto:** Implementa o design candidato; **não** fecha o gate D-05 (modelo de ameaças reaberto por AD-022 ainda exige re-aprovação humana antes de virar base estável). A orquestração de app-unlock e os comandos Tauri ficam para `local-sessions`. Pendência imediata: rodar `pnpm check:rust` completo (aviso `linker_messages` a avaliar sob `-D warnings`).

### AD-023: App funcional (frontend-only) — vertical slice senha global + sessões (2026-07-15)

**Decision:** As telas deixaram de ser mockup estático e viraram um **app navegável e funcional**. Os componentes saíram de `src/mockup/` para `src/components`, `src/screens`, `src/stores`, `src/utils`; a galeria `MockupShell`/`StyleGuide` foi removida. Navegação com **vue-router** (hash history) + guard de acesso. Implementada a **1ª vertical**: criar/desbloquear senha global (GMP) e ciclo de vida de sessões (criar/listar/abrir/bloquear/desbloquear), com estado reativo em `src/stores/vault.ts` e persistência em `localStorage`.
**Reason:** Pedido do usuário: "faça ele ser utilizável, não só mockup". Escopo fatiado: sessões primeiro, segredos (CRUD) depois.
**Trade-off / ⚠️ SPEC_DEVIATION:** (1) Deriva do design de `ui-screens` ("100% estático, não montado por main.ts"). (2) **NÃO é seguro / não é zero-knowledge:** a senha é guardada como hash SHA-256 em `localStorage` (apenas verificação), **não** o formato de `crypto-format` (Argon2id/AEAD, presos a PT-01/PT-02). É placeholder pré-backend — não usar para dados sensíveis.
**Impact:** Implementa parcialmente (só frontend) o design de [local-sessions](../features/local-sessions/design.md); o backend Rust/cripto e os comandos Tauri continuam pendentes. Próxima fatia: CRUD de segredos (T09–T11) sobre o mesmo store. `check:frontend` verde (88 testes) e fluxo validado no navegador (criar senha → desbloquear → criar sessão → cofre da sessão → bloquear).

### AD-022: Senha mestra global + autenticação por sessão (global/própria) (2026-07-15)

**Decision:** Introduzir uma **senha mestra global (GMP)** que trava o app inteiro; desbloqueá-la abre em conjunto todas as sessões `global` (padrão). Cada sessão pode optar por **senha própria** (`auth_mode = own`) e manter isolamento total. Modelo criptográfico canônico e fluxos em [ui-screens/context.md](../features/ui-screens/context.md) (D-04).
**Reason:** Conveniência de uma senha única no dia a dia, preservando a opção de isolar sessões sensíveis.
**Trade-off:** Quebra a garantia "sem desbloqueio transitivo": comprometer a GMP expõe todas as sessões `global` de uma vez (raio de exposição maior). Sessões `own` mantêm isolamento.
**Impact:** Propagado para `secure-vault/spec.md` (VAULT-05), `crypto-format/{spec,design}.md` (keyring global, GMK, GKEY-01/02), `secure-vault/threat-model.md` (A-11, T-AUTH-06/07, C-21) e `local-sessions/design.md` (gate de app-unlock, comandos novos). **Rebaixa o modelo de ameaças para "EM REVISÃO" — exige re-aprovação humana antes de virar base de design (D-05).**

### AD-001: Público e modelo de produto (2026-07-13)

**Decision:** Produto open source para cofres individuais, acessível a usuários técnicos e ao público geral.
**Reason:** Atender uso pessoal sem restringir a solução a um nicho exclusivamente técnico.
**Trade-off:** Recursos de equipes e compartilhamento ficam fora do v1.
**Impact:** A interface deve ser simples, mas as garantias e configurações de segurança precisam ser transparentes e auditáveis.

### AD-002: Plataforma inicial (2026-07-13)

**Decision:** Entregar o v1 somente para Windows e manter macOS e Linux no roadmap.
**Reason:** Reduzir a superfície inicial e permitir hardening específico da plataforma.
**Trade-off:** Adoção inicial limitada a Windows.
**Impact:** O formato do cofre deve ser portável, enquanto proteções locais podem ser específicas do Windows.

### AD-003: Recuperação local (2026-07-13) — SUPERADA POR AD-010

**Status:** Superada; preservada somente como histórico da decisão anterior.

**Decision:** Recuperar cofres por kit local criado pelo usuário, sem recuperação por terceiros.
**Reason:** Manter o modelo zero-knowledge sem tornar o esquecimento da senha necessariamente fatal.
**Trade-off:** Perder simultaneamente senha e kit implica perda definitiva do acesso.
**Impact:** O fluxo de criação precisa confirmar backup do kit e o design deve definir revogação e rotação após uso.

### AD-004: Sincronização no v1 (2026-07-13)

**Decision:** O v1 sincronizará entre múltiplos dispositivos via OneDrive ou Google Drive.
**Reason:** Permitir continuidade entre computadores sem operar infraestrutura própria de conteúdo.
**Trade-off:** OAuth, conflitos, rollback e disponibilidade da nuvem ampliam o modelo de ameaças.
**Impact:** O armazenamento remoto conterá apenas blobs cifrados; conflitos nunca poderão causar perda silenciosa.

### AD-005: Tipos de segredo do v1 (2026-07-13)

**Decision:** Suportar senhas, chaves de API, tokens genéricos, notas secretas e chaves SSH.
**Reason:** Cobrir uso geral e técnico sem introduzir anexos binários no primeiro formato.
**Trade-off:** TOTP, arquivos, cartões e identidades ficam adiados.
**Impact:** O modelo de dados precisa aceitar campos sensíveis tipados e extensibilidade futura.

### AD-006: Aprovação da especificação inicial (2026-07-13)

**Decision:** A visão, o roadmap e a especificação inicial do Cofre Seguro v1 foram aprovados pelo usuário.
**Reason:** Os objetivos, limites e requisitos refletem o produto pretendido.
**Trade-off:** Alterações posteriores de escopo precisarão ser avaliadas e rastreadas explicitamente.
**Impact:** O trabalho pode avançar para discussão das áreas ambíguas antes do design.

### AD-007: Sessões de segurança independentes (2026-07-13)

**Decision:** O aplicativo permitirá múltiplas sessões de segurança persistentes e nomeadas pelo usuário para representar contextos como “Trabalho”, “Pessoal” ou projetos específicos. Os nomes serão únicos sem diferenciar maiúsculas de minúsculas e poderão ser alterados somente enquanto a sessão estiver desbloqueada. Cada sessão terá sua própria senha mestra, estado de bloqueio e período configurável de bloqueio automático, inclusive a opção explícita de nunca bloquear automaticamente.
**Reason:** Segredos de contextos e níveis de confidencialidade diferentes precisam de separação identificável e políticas proporcionais sem obrigar o usuário a aplicar o mesmo nível de atrito a tudo.
**Trade-off:** Múltiplas senhas e estados aumentam a complexidade da interface, do gerenciamento de chaves e dos testes; escolher “nunca” reduz a proteção daquela sessão.
**Impact:** A sessão é um contêiner persistente, não uma execução temporária. A criação e renomeação precisam validar unicidade normalizada; desbloquear uma sessão não desbloqueia nenhuma outra e entrar em uma sessão bloqueada sempre exige sua própria senha mestra.

### AD-008: Limpeza configurável do clipboard (2026-07-13)

**Decision:** A limpeza automática do clipboard será configurável, com padrão de 5 minutos, e haverá uma ação “Limpar agora”.
**Reason:** Equilibrar conveniência com a redução do tempo de exposição de um segredo copiado.
**Trade-off:** Cinco minutos ampliam a janela de exposição em relação a um intervalo curto; a limpeza continua sujeita às limitações do clipboard e do sistema operacional.
**Impact:** A interface deve informar o temporizador, permitir configuração e não afirmar sucesso quando a limpeza não puder ser confirmada.

### AD-009: Sincronização com semântica inspirada no Git (2026-07-13)

**Decision:** A sincronização seguirá o modelo conceitual de enviar e obter mudanças (push/pull), tentará mesclar mudanças automaticamente e encaminhará conflitos não resolvidos para decisão explícita do usuário, preservando todas as versões relevantes.
**Reason:** Evitar perda silenciosa e dar controle ao usuário quando dois dispositivos alterarem o mesmo conteúdo.
**Trade-off:** Histórico, detecção de ancestralidade e resolução de conflitos tornam o formato e a experiência mais complexos.
**Impact:** “Inspirada no Git” define o comportamento, não obriga a usar Git internamente nem permite que conteúdo legível seja enviado ao provedor.

### AD-010: Kit de recuperação adiado (2026-07-13)

**Decision:** O v1 não terá kit de recuperação nem mecanismo substituto por enquanto.
**Reason:** O mecanismo precisa de mais reflexão antes de introduzir material adicional capaz de recuperar acesso.
**Trade-off:** Perder a senha mestra de uma sessão implica perda definitiva de acesso aos dados daquela sessão no v1.
**Impact:** O kit sai dos objetivos e do roadmap do v1 e permanece como ideia futura; o onboarding deve explicar claramente a ausência de recuperação.

### AD-011: Acesso físico avançado no modelo de ameaças (2026-07-13)

**Decision:** O modelo de ameaças avaliará explicitamente adversários tecnicamente capacitados com acesso ao equipamento, separando ataques offline de ataques contra um sistema já comprometido durante o uso.
**Reason:** Reduzir o risco de extração de senhas, chaves ou dados cifrados por quem conhece hardware e mecanismos de baixo nível.
**Trade-off:** Algumas ameaças podem apenas ser mitigadas ou depender de recursos como hardware compatível, configuração segura do Windows e proteção de disco; não haverá promessa de segurança absoluta.
**Impact:** A pesquisa de M0 deve avaliar derivação de chave resistente a ataques offline, proteção de memória, apagamento, armazenamento apoiado por hardware e garantias/limites do Windows antes do design criptográfico.

### AD-012: Bloqueio orientado por inatividade e eventos do sistema (2026-07-13)

**Decision:** O bloqueio automático usa 15 minutos por padrão, conta inatividade independentemente por sessão e reinicia somente após interação intencional dentro dela; continua contando enquanto o aplicativo está minimizado e pode ser configurado de 1 minuto até “nunca”. Novas sessões bloqueiam por padrão ao bloquear ou suspender o Windows, mas cada reação pode ser desativada individualmente; fechar o aplicativo bloqueia todas.
**Reason:** Aplicar proteção proporcional por sessão sem confundir tempo de uso ativo com tempo de exposição abandonada.
**Trade-off:** Políticas diferentes podem deixar algumas sessões abertas após eventos do Windows; escolher “nunca” exige que o usuário aceite explicitamente esse risco.
**Impact:** O design deve implementar cronômetros independentes, reconhecer interações intencionais dentro da sessão, ativar por padrão os dois eventos do Windows e exigir confirmação da opção “nunca”.

### AD-013: Visibilidade e operações entre sessões (2026-07-13)

**Decision:** Nomes e quantidade de sessões permanecem visíveis quando bloqueadas; a pesquisa consulta todas as sessões desbloqueadas; mover um segredo exige origem e destino desbloqueados; excluir uma sessão exige confirmação e sua senha mestra.
**Reason:** Manter navegação e organização convenientes sem atravessar as fronteiras criptográficas de sessões bloqueadas.
**Trade-off:** Nomes e quantidade de sessões tornam-se metadados visíveis antes do desbloqueio.
**Impact:** A interface e o índice de pesquisa devem respeitar dinamicamente o conjunto de sessões desbloqueadas.

### AD-014: Política da senha mestra (2026-07-13)

**Decision:** Senhas mestras terão comprimento mínimo, indicador de força, dica opcional e atraso progressivo após erros. A dica será sincronizada como metadado não secreto para a aplicação, aparecerá na tela bloqueada somente após “Mostrar dica” e terá aviso de que é visível sem senha e não deve conter a senha nem partes óbvias dela. A troca exige a senha atual e o aplicativo avisará periodicamente que não há recuperação no v1, com cadência a definir futuramente.
**Reason:** Orientar escolhas melhores e reduzir tentativas repetidas sem criar uma falsa promessa de recuperação.
**Trade-off:** A dica é metadado exposto a quem acessa a tela bloqueada e pode revelar informação; o atraso progressivo também pode afetar o usuário legítimo.
**Impact:** O design deve sincronizar e autenticar a dica sem tratá-la como segredo ou prova de acesso. Um futuro fluxo “Esqueci minha senha” e a cadência exata dos lembretes continuam adiados.

### AD-015: Sincronização automática e modo somente leitura (2026-07-13)

**Decision:** A sincronização ocorrerá automaticamente inclusive para sessões bloqueadas, transportando apenas blobs cifrados sem descriptografá-los. O modo somente leitura será configurado por sessão em cada dispositivo, continuará recebendo atualizações, não produzirá alterações nos segredos naquele dispositivo e exigirá a senha mestra para habilitar edição.
**Reason:** Manter dispositivos atualizados com menor esforço e permitir contextos de consulta sem edição acidental.
**Trade-off:** A sincronização bloqueada exige separar rigorosamente transporte cifrado de descriptografia; o modo somente leitura adiciona estado independente por sessão e dispositivo.
**Impact:** A arquitetura de sincronização deve operar sobre envelopes cifrados e autenticados, e a transição de somente leitura para edição deve passar pela autenticação da sessão.

### AD-016: Resolução e retenção de conflitos (2026-07-13)

**Decision:** Conflitos não resolvidos serão tratados por um mecanismo dedicado, comparados campo a campo e oferecerão manter o valor local, remoto ou ambos. Durante os 7 dias finais dos 30 dias de pendência haverá aviso persistente e notificação diária. Ao expirar, as versões tornam-se entradas permanentes “local” e “remota”, numeradas apenas quando houver múltiplas versões da mesma origem. “Manter ambos” cria entradas separadas para campos de valor único, e uma resolução manual pode ser desfeita por 7 dias.
**Reason:** Dar ao usuário controle granular e tempo previsível para impedir perda acidental em edições concorrentes.
**Trade-off:** A materialização evita perda, mas pode criar entradas duplicadas e aumentar o armazenamento quando conflitos forem ignorados.
**Impact:** Expiração significa encerrar a pendência, nunca apagar versões; o modelo de dados precisa registrar origem, numeração condicional e janela reversível de 7 dias.

### AD-017: Stack de frontend e direção visual inicial (2026-07-13)

**Decision:** O aplicativo usará Tauri 2 com core em Rust e frontend construído com Vite e Tailwind CSS. A primeira implementação seguirá um visual funcional e genérico; uma identidade mais única e característica será explorada depois da validação dos fluxos e da arquitetura.
**Reason:** Entregar rapidamente uma base consistente e ajustável sem transformar a definição de identidade visual em bloqueio para as decisões de segurança e usabilidade.
**Trade-off:** A primeira versão pode parecer menos diferenciada e o refinamento posterior exigirá uma etapa explícita de design visual.
**Impact:** Componentes e tokens devem ser organizados desde o início para permitir evolução estética sem reescrever os fluxos. O bundle do frontend permanece local, sob CSP e capabilities mínimas do Tauri.

### AD-018: Padrão de commits, versionamento e releases (2026-07-13)

**Decision:** O projeto usará Conventional Commits com descrição em português, branches curtas, PRs e squash merge sobre uma `main` protegida. Versões seguem SemVer, com série `0.x` antes da estabilidade, Release PR revisada, tag anotada e assinada `vX.Y.Z`, build no GitHub Actions e GitHub Release publicada de forma imutável. NSIS será o instalador/updater primário do v1.
**Reason:** Tornar histórico, incremento de versão, artefatos e origem do build previsíveis e auditáveis sem permitir que uma mensagem de commit publique automaticamente uma versão sensível.
**Trade-off:** O fluxo adiciona uma PR e confirmação manual por release; assinatura, smoke tests e imutabilidade tornam correções pós-publicação uma nova versão obrigatória.
**Impact:** `tauri.conf.json` será a fonte canônica da versão; workflows validarão manifests e tags, usarão permissões mínimas e separarão assinatura Tauri, Authenticode e attestations. A configuração remota está documentada em `.specs/project/RELEASES.md`.

### AD-019: Aprovação do modelo de ameaças do v1 (2026-07-14)

**Decision:** O modelo de ameaças foi aprovado como base para o design e a implementação do v1, incluindo riscos residuais, limites de garantia e protótipos bloqueadores documentados.
**Reason:** As fronteiras de confiança e os controles obrigatórios estão suficientemente definidos para orientar o scaffold e os experimentos de M0.
**Trade-off:** A aprovação não certifica controles ainda não implementados e mantém KDF, AEAD, formato, memória protegida e checkpoints bloqueados pelos protótipos correspondentes.
**Impact:** O projeto pode criar a fundação executável e avançar nos protótipos, mantendo os gates de release do modelo de ameaças.

### AD-020: Vue 3 e TypeScript no frontend (2026-07-14)

**Decision:** O frontend usará Vue 3 com TypeScript, Vite e Tailwind CSS.
**Reason:** A combinação foi definida pelo usuário e oferece uma base tipada, componentizada e empacotada localmente para a interface Tauri.
**Trade-off:** Vue adiciona runtime e dependências em relação a HTML/TypeScript puro, exigindo auditoria e atualização controlada.
**Impact:** Componentes, testes e configuração do frontend devem seguir Vue 3, sem conteúdo remoto em runtime e sob CSP estrita.

### AD-021: Updater controlado pelo core e configuração de release efêmera (2026-07-14)

**Decision:** O Tauri Updater será registrado e orquestrado pelo core Rust, sem capability direta para a WebView. Builds de release usarão uma sobreposição de configuração gerada no GitHub Actions com a chave pública e o endpoint estável; a chave privada existirá somente no environment protegido `release`.
**Reason:** Reduzir a superfície exposta ao frontend e impedir que material ou configuração operacional de assinatura seja exigido em builds locais.
**Trade-off:** A interface de atualização precisará de comandos Rust estreitos e testados; releases não funcionarão até que environment, variável e secrets sejam configurados no GitHub.
**Impact:** O bundle inicial é somente NSIS, releases nascem em draft e artefatos do updater são assinados no mesmo build da tag.

---

## Active Blockers

- A primeira distribuição pública depende da configuração do environment `release`, do par de chaves do updater e da estratégia/certificado Authenticode.

## Lessons Learned

Nenhuma registrada.

## Quick Tasks Completed

- [x] Padronizar o frontend: nomes reais das telas (sem prefixo `Txx_`), **um componente por pasta com seu teste** (`components/Nome/Nome.vue` + `Nome.test.ts`, idem `screens/`), e **alias de import `@/`** configurado em Vite, Vitest e tsconfig; imports migrados; `check:frontend` verde (88 testes) — 2026-07-15.
- [x] Tornar o app funcional (frontend-only): mover telas para fora de `src/mockup/`, adicionar vue-router + guard, `src/stores/vault.ts` e a vertical senha global + sessões; 88 testes frontend, tsc e build verdes; fluxo validado no navegador — 2026-07-15. Ver AD-023.
- [x] Implementar o mockup visual estático T01–T16, StyleGuide e viewer navegável, com 85 testes frontend e build Tauri validado — 2026-07-15.
- [x] Criar script PowerShell para escolher `fix`, `feature` ou `release`, calcular o próximo SemVer, abrir `chore/release-vX.Y.Z` a partir de uma `main` limpa e sincronizar a versão nos manifests — 2026-07-14.
- [x] Corrigir os atalhos do seletor de release para `X` (Fix), `F` (Feature) e `R` (Release), sem colisões — 2026-07-14.
- [x] Declarar `serde_json` diretamente no crate para o `generate_context!` compilar com o overlay do Tauri Updater usado em releases — 2026-07-14.

## Deferred Ideas

- [ ] Suporte a macOS — Capturado durante: inicialização do projeto
- [ ] Suporte a Linux — Capturado durante: inicialização do projeto
- [ ] Extensão de navegador e preenchimento automático — Capturado durante: escopo do v1
- [ ] Aplicativos móveis — Capturado durante: roadmap
- [ ] Compartilhamento de cofres — Capturado durante: escopo do v1
- [ ] TOTP e anexos — Capturado durante: tipos de segredo
- [ ] Passkeys, biometria e chaves físicas — Capturado durante: desbloqueio
- [ ] Kit local de recuperação ou outro mecanismo de recuperação de acesso — Capturado durante: revisão do escopo do v1
- [ ] Definir a cadência exata dos lembretes de ausência de recuperação — Capturado durante: política da senha mestra

## Todos

- [x] Revisar e aprovar o modelo de ameaças do v1, incluindo riscos residuais e limites explícitos de garantia.
- [ ] Executar os protótipos críticos definidos no modelo de ameaças antes de escolher algoritmos e parâmetros finais.
- [ ] Definir arquitetura e formato criptográfico após aprovação do modelo de ameaças.
- [ ] Criar/configurar o repositório remoto e aplicar rulesets de `main` e tags `v*`, squash merge, environment `release` e immutable releases.
- [x] Implementar os workflows iniciais de CI e release para Windows/NSIS e Tauri Updater.
- [ ] Definir e contratar a estratégia de certificado Authenticode antes da primeira distribuição pública.

## Preferences

**Model Guidance Shown:** never
