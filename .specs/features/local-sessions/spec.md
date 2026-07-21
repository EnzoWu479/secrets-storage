# Sessões de Segurança e Desbloqueio — Especificação

**Feature:** `local-sessions` (primeira fatia vertical de M1)
**Design:** [design.md](./design.md)
**Requisitos-fonte:** [secure-vault/spec.md](../secure-vault/spec.md) — VAULT-01, VAULT-02, VAULT-03, VAULT-04, VAULT-05
**Modelo de ameaças:** [secure-vault/threat-model.md](../secure-vault/threat-model.md) — re-aprovado em 2026-07-21 (AD-022/GMP, gate D-05 fechado)
**Formato criptográfico:** [crypto-format/design.md](../crypto-format/design.md)
**Modelo canônico de fluxos:** [ui-screens/context.md](../ui-screens/context.md) (D-04/D-05)
**Status:** Approved — 2026-07-21

---

## Problem Statement

O produto já possui formato criptográfico e core de segredos, mas não tem um ciclo de vida de sessões real: hoje o `secret-management` avança contra um fake determinístico (`SessionAccess`) porque não existe um `SessionManager` de produção. Sem ele, nenhuma operação privilegiada de segredos pode ser exposta via IPC (gate externo **G1**), e o app não tem sequer uma fronteira de desbloqueio confiável. Esta fatia entrega esse ciclo de vida: senha mestra global (GMP), sessões nomeadas com `auth_mode` global/própria, bloqueio/desbloqueio, políticas de inatividade e proteção de senha.

## Goals

- [ ] Entregar o gate de **senha mestra global (GMP)**: criar no 1º uso, desbloquear (abrindo todas as sessões `global`) e trocar.
- [ ] Entregar o ciclo de vida de sessões: criar/nomear/listar/renomear/excluir e bloquear/desbloquear, com `auth_mode` **global** (padrão) e **própria** (`own`) isolada.
- [ ] Aplicar política de bloqueio por inatividade (padrão 15 min), bloqueio em eventos do Windows (best-effort) e bloqueio total ao fechar o app (fail-closed).
- [ ] Entregar proteção de senha: comprimento mínimo + indicador de força, atraso progressivo, dica sob demanda e avisos de exposição/ausência de recuperação.
- [ ] **Entregar o `SessionManager` de produção** que satisfaz o contrato do gate **G1** de `secret-management` (estado desbloqueado, guards de lock/epoch, acesso ao `SessionContent`, commit cifrado autorizado e revalidado).

## Out of Scope

| Item | Motivo |
| --- | --- |
| CRUD de segredos, clipboard (SECRET-*) | Fatia própria (`secret-management`); o payload nasce `secrets: []` e é preenchido depois sem mudar o formato. |
| Sincronização e conflitos (SYNC-*) | Fatia futura; blobs são apenas locais nesta fatia. |
| OAuth e provedores de nuvem | Depende de SYNC-*. |
| Atualização/updater (UPDATE-*) | Fatia independente. |
| Modo somente leitura por sessão | Faz parte de SYNC-* (depende de origem remota). |
| Validação profunda de eventos Windows lock/suspend | Best-effort aqui; validação a fundo em `windows-tauri-proof` (PT-05). |
| Hardening de zeroização de memória | Best-effort aqui; endurecido em PT-04. |
| zxcvbn / força de senha avançada | Heurística local nesta fatia; melhoria futura. |
| Persistência anti-bypass do atraso progressivo | Em memória nesta fatia; persistência é decisão futura. |

---

## User Stories

### P1: Desbloqueio de entrada pela senha mestra global (GMP) ⭐ MVP

**User Story:** Como usuário, quero desbloquear o aplicativo com uma senha mestra global que abre de uma vez todas as minhas sessões globais, para ter conveniência de senha única no dia a dia.

**Why P1:** Toda operação sobre sessões globais depende de uma fronteira de desbloqueio de entrada confiável (gate global).

**Acceptance Criteria:**

1. WHEN o app inicia e o keyring global ainda não existe THEN o sistema SHALL apresentar o fluxo de criação da GMP (1º uso) e, ao criar, gerar `salt_global` + GMK aleatória, derivar a gKEK e gravar `keyring.vault`, sem persistir a senha em claro. *(SESSION-01)*
2. WHEN o app inicia e o keyring global já existe THEN o sistema SHALL apresentar a tela de desbloqueio de entrada e exigir a GMP antes de liberar qualquer sessão global. *(SESSION-02)*
3. WHEN a GMP correta é fornecida THEN o sistema SHALL desfazer o wrap da GMK e abrir de uma vez todas as sessões `global`, mantendo bloqueadas as sessões `own`. *(SESSION-02)*
4. WHEN a GMP incorreta é fornecida THEN o sistema SHALL negar o desbloqueio, aplicar atraso progressivo global e não revelar informação útil. *(SESSION-03, SESSION-18)*
5. WHEN o usuário troca a GMP THEN o sistema SHALL exigir a GMP atual, desfazer o wrap da GMK com a gKEK atual e reenrolá-la com a gKEK' derivada da nova GMP (mesma GMK), respeitando força mínima. *(SESSION-04)*
6. WHEN o app é encerrado ou o usuário aciona bloqueio global THEN o sistema SHALL descartar a GMK e bloquear todas as sessões (fail-closed). *(SESSION-14)*

**Independent Test:** No 1º uso, criar a GMP e comprovar que `keyring.vault` foi criado sem senha em claro; reiniciar, desbloquear pela GMP e comprovar que sessões globais abrem juntas e as `own` permanecem bloqueadas; errar a GMP e observar atraso crescente; trocar a GMP e comprovar que os conteúdos anteriores continuam abríveis.

---

### P1: Ciclo de vida de sessões ⭐ MVP

**User Story:** Como usuário, quero criar, nomear, listar, renomear e excluir sessões, para separar contextos como trabalho, uso pessoal e projetos.

**Why P1:** Sessões são a unidade de organização e de fronteira de segurança do produto.

**Acceptance Criteria:**

1. WHEN o usuário cria uma sessão THEN o sistema SHALL adotar `auth_mode = global` por padrão e permitir marcar "usar senha própria" para criá-la como `own` isolada, persistindo apenas o necessário para abri-la e gerando o cofre com payload `secrets: []` já cifrado. *(SESSION-05)*
2. WHEN uma sessão `global` é criada THEN o sistema SHALL exigir o app desbloqueado e envolver a `root_key` da sessão com a GMK, sem pedir senha nova. *(SESSION-05)*
3. WHEN uma sessão `own` é criada THEN o sistema SHALL exigir uma senha própria com força mínima e permitir dica opcional. *(SESSION-05, SESSION-17, SESSION-19)*
4. WHEN o usuário cria ou renomeia uma sessão THEN o sistema SHALL rejeitar nomes já usados por outra sessão sem diferenciar maiúsculas de minúsculas (comparação normalizada). *(SESSION-06)*
5. WHEN a lista de sessões é apresentada THEN o sistema SHALL funcionar mesmo com o app bloqueado e retornar id, nome, `auth_mode`, estado de bloqueio, política e existência de dica — nunca segredos. *(SESSION-07)*
6. WHEN o usuário renomeia uma sessão THEN o sistema SHALL exigir que aquela sessão esteja desbloqueada e que o novo nome seja único. *(SESSION-08)*
7. WHEN o usuário exclui uma sessão THEN o sistema SHALL exigir confirmação explícita e a senha mestra válida daquela sessão, apagando o cofre e a entrada do registro. *(SESSION-09)*
8. WHEN o registro é adulterado (nome divergente do autenticado na AAD do cofre) THEN o sistema SHALL detectar a divergência no unlock e falhar fechado. *(SESSION-13)*

**Independent Test:** Criar "Trabalho" (global) e "Pessoal" (own), rejeitar uma terceira "trabalho"; listar com app bloqueado e comprovar ausência de segredos; renomear apenas uma sessão desbloqueada; excluir exigindo a senha correta; adulterar o `name` no registro e comprovar falha fechada no unlock.

---

### P1: Bloqueio, desbloqueio e isolamento entre sessões ⭐ MVP

**User Story:** Como usuário, quero bloquear e desbloquear sessões individualmente e ter certeza de que desbloquear uma não desbloqueia outra indevidamente, para proteção proporcional por contexto.

**Why P1:** É a garantia central de isolamento do modelo de ameaças (C-01).

**Acceptance Criteria:**

1. WHEN uma sessão `own` está bloqueada THEN o sistema SHALL exigir a senha própria dela para desbloquear, mesmo com a GMP já desbloqueada e outras sessões abertas (sem desbloqueio transitivo). *(SESSION-10)*
2. WHEN a senha própria correta de uma sessão `own` é fornecida THEN o sistema SHALL desfazer o wrap AEAD, verificar o nome autenticado e carregar a `content_key` em memória protegida. *(SESSION-10)*
3. WHEN uma sessão é bloqueada manualmente ou por política THEN o sistema SHALL removê-la do estado desbloqueado e zeroizar seu material, impedindo leitura/modificação até novo desbloqueio, independentemente das demais. *(SESSION-11)*
4. WHEN o usuário aciona "bloquear todas" ou o app encerra THEN o sistema SHALL bloquear todas as sessões e descartar a GMK. *(SESSION-11, SESSION-14)*
5. WHEN qualquer operação privilegiada é solicitada THEN o sistema SHALL revalidar no core Rust o estado desbloqueado, a existência da sessão, o `auth_mode` e os limites, independentemente do estado da WebView. *(SESSION-24)*

**Independent Test:** Com a GMP aberta, comprovar que uma sessão `own` continua bloqueada e exige a senha dela; bloquear uma sessão e comprovar que as demais seguem abertas; comprovar que nenhuma senha/chave cruza o IPC.

---

### P1: `auth_mode` e conversão global ↔ própria ⭐ MVP

**User Story:** Como usuário, quero converter uma sessão entre "usa a senha global" e "tem senha própria", para ajustar o isolamento sem recriar a sessão.

**Why P1:** É o mecanismo de opt-out de isolamento introduzido pela AD-022 e precisa ser seguro contra rebaixamento.

**Acceptance Criteria:**

1. WHEN o usuário converte uma sessão `global` → `own` THEN o sistema SHALL exigir a GMP desbloqueada, derivar a proteção da nova senha própria (força mínima) e reenrolar a `root_key`, sem alterar o conteúdo. *(SESSION-12)*
2. WHEN o usuário converte uma sessão `own` → `global` THEN o sistema SHALL exigir a senha própria atual e reenrolar a `root_key` com a GMK, sem alterar o conteúdo. *(SESSION-12)*
3. WHEN uma sessão é persistida THEN o sistema SHALL autenticar seu `auth_mode` na AAD do header do cofre. *(SESSION-13)*
4. WHEN um atacante tenta rebaixar `own` → `global` editando metadados sem a chave correta THEN o sistema SHALL falhar a autenticação AEAD e recusar o cofre. *(SESSION-13)*

**Independent Test:** Converter nos dois sentidos exigindo a senha atual apropriada e comprovar que o conteúdo permanece abrível; adulterar o `auth_mode` no header e comprovar recusa por falha de autenticação.

---

### P1: Política de bloqueio e eventos ⭐ MVP

**User Story:** Como usuário, quero que sessões bloqueiem sozinhas após inatividade e em eventos do sistema, para reduzir a janela de exposição.

**Why P1:** Bloqueio automático é um controle de segurança padrão (C-13) e afeta todas as sessões.

**Acceptance Criteria:**

1. WHEN uma sessão é criada THEN o sistema SHALL configurar 15 minutos de inatividade por padrão e permitir ajuste de 1 minuto até "nunca", exigindo confirmação explícita para "nunca". *(SESSION-14)*
2. WHEN ocorre inatividade ≥ o limite de uma sessão desbloqueada (e ≠ "nunca") THEN o sistema SHALL bloqueá-la automaticamente. *(SESSION-14)*
3. WHEN ocorre interação intencional dentro de uma sessão THEN o sistema SHALL reiniciar somente o cronômetro daquela sessão; com o app minimizado SHALL continuar contando inatividade. *(SESSION-15)*
4. WHEN uma sessão é criada THEN o sistema SHALL ativar por padrão o bloqueio ao bloquear e ao suspender o Windows, permitindo desativar individualmente cada reação (best-effort nesta fatia). *(SESSION-16)*

**Independent Test:** Configurar políticas diferentes por sessão e comprovar cronômetros independentes; "tocar" uma sessão e ver só o timer dela reiniciar; simular evento do Windows e observar o bloqueio best-effort; comprovar que fechar o app sempre bloqueia.

---

### P1: Proteção da senha mestra e tentativas ⭐ MVP

**User Story:** Como usuário, quero orientação para senhas fortes, dica sob demanda e proteção contra tentativas repetidas, para reduzir o risco de acesso indevido.

**Why P1:** O acesso ao app (GMP) e a cada sessão `own` depende exclusivamente da senha no v1 (VAULT-04).

**Acceptance Criteria:**

1. WHEN uma senha mestra (GMP ou própria) é criada ou trocada THEN o sistema SHALL exigir o comprimento mínimo e exibir um indicador de força compreensível. *(SESSION-17)*
2. WHEN tentativas incorretas se repetem (na GMP ou numa sessão `own`) THEN o sistema SHALL aplicar atraso progressivo em memória, sem apagar sessão ou cofre. *(SESSION-18)*
3. WHEN a sessão está bloqueada THEN o sistema SHALL revelar a dica somente após a ação explícita "Mostrar dica". *(SESSION-19)*
4. WHEN o usuário cria ou altera a dica THEN o sistema SHALL avisar que ela é visível sem senha e não deve conter a senha nem partes óbvias dela. *(SESSION-20)*
5. WHEN o usuário troca uma senha mestra THEN o sistema SHALL exigir a senha atual válida (GMP atual para a GMP; senha própria atual para a sessão). *(SESSION-21)*
6. WHEN o usuário usa o produto ao longo do tempo, ou tenta acessar após esquecer a senha THEN o sistema SHALL avisar que o v1 não possui recuperação — perder a GMP torna todas as sessões globais inacessíveis e perder a senha própria torna a sessão inacessível — sem oferecer atalho que contorne a senha. *(SESSION-22)*

**Independent Test:** Validar limite/força e atraso crescente na GMP e numa sessão `own`; comprovar que a dica só aparece após "Mostrar dica" e que o aviso de exposição é exibido; validar troca com senha atual e o aviso de ausência de recuperação.

---

### P1: Contrato de integração para o gate G1 ⭐ MVP

**User Story:** Como a feature `secret-management`, preciso de um `SessionManager` de produção que substitua o fake `SessionAccess`, para expor comandos de segredos com autorização real (destravar T14+).

**Why P1:** É o objetivo que motiva esta fatia; sem ele o gate G1 permanece aberto.

**Acceptance Criteria:**

1. WHEN `secret-management` solicita acesso a uma sessão THEN o `SessionManager` SHALL fornecer o mesmo contrato consumido pelo fake: estado desbloqueado, guard de lock/epoch, ordem determinística por UUID e acesso ao `SessionContent`. *(SESSION-23)*
2. WHEN um commit cifrado de conteúdo é solicitado por `secret-management` THEN o `SessionManager` SHALL revalidar a autorização (sessão desbloqueada + epoch atual) antes de avançar o conteúdo e a revisão na mesma linearização. *(SESSION-23)*
3. WHEN uma sessão é bloqueada ou sua epoch avança durante uma operação THEN o `SessionManager` SHALL negar o commit e preservar a última versão confirmada. *(SESSION-23)*
4. WHEN o processo encerra ou ocorre falha THEN o sistema SHALL evitar gravar segredos, senhas ou chaves em logs e arquivos temporários. *(SESSION-25)*

**Independent Test:** Trocar o fake pela implementação real nos testes de integração de `secret-management` e comprovar que a suíte permanece verde; comprovar deny de commit após lock/epoch; varrer logs/temporários por canários e não encontrar nenhum.

---

## Edge Cases

- WHEN o keyring global está corrompido ou tem versão futura THEN o sistema SHALL falhar fechado, preservar o arquivo e manter o app bloqueado com mensagem clara. *(SESSION-01)*
- WHEN o CSPRNG falha na criação de GMP/sessão THEN o sistema SHALL abortar a criação sem gravar estado parcial. *(SESSION-01, SESSION-05)*
- WHEN um cofre de sessão está ausente, corrompido ou tem versão futura THEN o sistema SHALL falhar fechado e preservar o arquivo. *(SESSION-10)*
- WHEN os parâmetros de KDF do cofre estão fora dos limites defensivos THEN o sistema SHALL recusar abrir sem alocar. *(SESSION-10, C-17)*
- WHEN o usuário tenta renomear uma sessão bloqueada THEN o sistema SHALL rejeitar e pedir desbloqueio. *(SESSION-08)*
- WHEN senha incorreta e sessão inexistente ocorrem THEN o sistema SHALL retornar o mesmo tipo de erro genérico onde couber, sem vazar existência. *(SESSION-03)*
- WHEN o usuário escolhe "nunca" para inatividade THEN o sistema SHALL exigir confirmação explícita. *(SESSION-14)*

---

## Requirement Traceability

| Requirement ID | Story | Fonte (VAULT) | Phase | Status |
| --- | --- | --- | --- | --- |
| SESSION-01 | Criar GMP (1º uso) | VAULT-05, VAULT-01 | Design | Pending |
| SESSION-02 | Desbloqueio de entrada pela GMP abre globais | VAULT-05, VAULT-01 (AC15/16) | Design | Pending |
| SESSION-03 | GMP incorreta nega sem vazar | VAULT-04, VAULT-01 (AC2) | Design | Pending |
| SESSION-04 | Trocar a GMP | VAULT-05, VAULT-04 (AC6) | Design | Pending |
| SESSION-05 | Criar sessão (global padrão / own opt-out) + nome | VAULT-01, VAULT-05 | Design | Pending |
| SESSION-06 | Unicidade de nome case-insensitive | VAULT-01 (AC13) | Design | Pending |
| SESSION-07 | Listar sessões (bloqueado, sem segredos) | VAULT-01 (AC9) | Design | Pending |
| SESSION-08 | Renomear (só desbloqueada) + unicidade | VAULT-01 (AC13/14) | Design | Pending |
| SESSION-09 | Excluir (confirmação + senha) | VAULT-01 (AC10) | Design | Pending |
| SESSION-10 | Desbloquear own (sem desbloqueio transitivo) | VAULT-01 (AC8), VAULT-03 | Design | Pending |
| SESSION-11 | Bloquear sessão (manual/política/all) | VAULT-01 (AC3/7), VAULT-02 | Design | Pending |
| SESSION-12 | Converter auth_mode global↔own | VAULT-05 (AC17) | Design | Pending |
| SESSION-13 | auth_mode autenticado na AAD (anti-rebaixamento) | VAULT-05 (AC18) | Design | Pending |
| SESSION-14 | Política de inatividade + bloqueio total no exit | VAULT-01 (AC4/7) | Design | Pending |
| SESSION-15 | Timer por sessão; conta minimizado | VAULT-01 (AC5) | Design | Pending |
| SESSION-16 | Reação a lock/suspend do Windows (best-effort) | VAULT-01 (AC6) | Design | Pending |
| SESSION-17 | Comprimento mínimo + indicador de força | VAULT-04 (AC1) | Design | Pending |
| SESSION-18 | Atraso progressivo (global + own) | VAULT-04 (AC2) | Design | Pending |
| SESSION-19 | Dica sob demanda | VAULT-04 (AC3/4) | Design | Pending |
| SESSION-20 | Aviso de exposição da dica | VAULT-04 (AC5) | Design | Pending |
| SESSION-21 | Trocar senha exige senha atual | VAULT-04 (AC6) | Design | Pending |
| SESSION-22 | Aviso de ausência de recuperação | VAULT-04 (AC7/8) | Design | Pending |
| SESSION-23 | Contrato SessionManager para G1 (unlock/epoch/commit) | VAULT-01, VAULT-03 | Design | Pending |
| SESSION-24 | Revalidação de autoridade no core (WebView não confiável) | VAULT-01 (AC11), C-10 | Design | Pending |
| SESSION-25 | Sem plaintext/senha/chave em logs e temporários | VAULT-01 (AC11), C-15 | Design | Pending |

**ID format:** `SESSION-[NUMBER]`

**Status values:** Pending → In Design → In Tasks → Implementing → Verified

**Coverage:** 25 requisitos; todos mapeados a VAULT-01…05. Mapeamento para tasks pendente até `tasks.md`.

---

## Success Criteria

- [ ] O usuário cria a GMP no 1º uso, reinicia, desbloqueia e vê todas as sessões globais abertas e as `own` bloqueadas.
- [ ] Sessões `own` nunca abrem por desbloqueio transitivo; conversão de `auth_mode` exige a senha atual apropriada; rebaixamento sem chave falha.
- [ ] Políticas de bloqueio independentes funcionam; fechar o app sempre bloqueia (fail-closed).
- [ ] Nenhuma senha, chave ou material derivado cruza o IPC; nenhum canário aparece em logs/temporários.
- [ ] **A suíte de integração de `secret-management` passa com o `SessionManager` real no lugar do fake `SessionAccess`, satisfazendo a evidência de liberação do gate G1.**
- [ ] `pnpm check` verde (Rust + frontend) e builds normal/security-proof compilam separados.
