# Cofre Seguro v1 — Especificação

## Problem Statement

Usuários mantêm senhas e segredos técnicos dispersos em arquivos, notas, navegadores e serviços que podem expor conteúdo ou dificultar controle e recuperação. O produto deve oferecer um cofre individual, local-first e zero-knowledge, capaz de sincronizar dados já cifrados pelo armazenamento em nuvem escolhido sem entregar a terceiros os meios de leitura.

## Goals

- [ ] Permitir o ciclo completo dos cinco tipos de segredo do v1 em sessões de segurança independentes e bloqueáveis.
- [ ] Manter todo conteúdo sensível cifrado e autenticado quando persistido ou sincronizado.
- [ ] Sincronizar entre dispositivos sem perda silenciosa e continuar operando offline.
- [ ] Atualizar o aplicativo somente a partir de artefatos autenticados.

## Out of Scope

| Feature | Reason |
| --- | --- |
| macOS e Linux | Planejados após hardening do v1 Windows |
| Cofres compartilhados | Exigem identidade, autorização e criptografia multiusuário próprias |
| Extensão de navegador/autofill | Aumenta significativamente a superfície de ataque |
| TOTP e anexos | Ampliam modelo de dados e riscos de exposição |
| Recuperação remota | Viola o objetivo zero-knowledge ou cria custódia adicional |
| Kit de recuperação no v1 | Requer mais análise de segurança e experiência; foi adiado sem substituto por enquanto |
| Proteção absoluta de host comprometido | Malware com controle do sistema pode observar dados durante uso legítimo |

---

## User Stories

> **Atenção — modelo de ameaças reaberto:** a introdução da senha mestra global (GMP) com sessões globais desbloqueadas em conjunto **contradiz** a garantia original de isolamento total ("estados de bloqueio independentes, sem desbloqueio transitivo") e exige **re-aprovação do modelo de ameaças** antes da implementação. Ver `../../features/ui-screens/context.md` (D-04 e D-05).

### P1: Criar e desbloquear sessões de segurança ⭐ MVP

**User Story:** Como usuário individual, quero desbloquear o aplicativo com uma senha mestra global (GMP) que abre de uma vez todas as minhas sessões globais e, quando precisar, criar sessões com senha própria isoladas, cada uma com sua política de bloqueio, para separar contextos como trabalho, uso pessoal ou projetos específicos e aplicar proteção proporcional à confidencialidade dos dados.

**Why P1:** Toda outra capacidade depende de uma fronteira de desbloqueio confiável.

**Acceptance Criteria:**

1. WHEN o usuário cria uma sessão de segurança THEN o sistema SHALL adotar a senha mestra global (GMP) por padrão (sessão global) e permitir marcar “usar senha própria” para torná-la uma sessão com senha própria (`own`) isolada, derivando e persistindo em ambos os casos somente os dados necessários para verificar e abrir aquela sessão, nunca a senha em claro.
2. WHEN uma senha mestra incorreta (global ou própria) é fornecida THEN o sistema SHALL negar acesso à sessão correspondente sem revelar informação útil sobre seu conteúdo.
3. WHEN uma sessão é bloqueada manualmente ou por política automática THEN o sistema SHALL impedir leitura e modificação de seus segredos até novo desbloqueio, independentemente do estado das demais sessões.
4. WHEN uma sessão é criada THEN o sistema SHALL configurar 15 minutos de inatividade por padrão e permitir ajuste em um controle de 1 minuto até “nunca”, exigindo confirmação explícita ao escolher “nunca”.
5. WHEN ocorre uma interação intencional dentro de uma sessão THEN o sistema SHALL reiniciar somente o cronômetro daquela sessão; enquanto o aplicativo está minimizado SHALL continuar contando o período como inatividade para cada sessão aplicável.
6. WHEN uma sessão é criada THEN o sistema SHALL ativar por padrão o bloqueio ao bloquear e ao suspender o Windows, permitindo desativar individualmente cada reação naquela sessão.
7. WHEN o aplicativo é encerrado THEN o sistema SHALL bloquear todas as sessões e descartar o material descriptográfico mantido para uso.
8. WHEN uma sessão com senha própria permanece bloqueada THEN o sistema SHALL exigir a senha própria dela para desbloqueá-la, mesmo com a GMP já desbloqueada e outras sessões abertas (sem desbloqueio transitivo); as sessões globais, por sua vez, desbloqueiam em conjunto com a GMP.
9. WHEN a lista de sessões é apresentada THEN o sistema SHALL permitir visualizar nomes e quantidade de sessões bloqueadas sem revelar seus segredos.
10. WHEN o usuário exclui uma sessão THEN o sistema SHALL exigir confirmação explícita e a senha mestra daquela sessão.
11. WHEN o processo é encerrado ou ocorre falha THEN o sistema SHALL evitar gravar segredos, senhas mestras ou chaves descriptográficas em logs e arquivos temporários.
12. WHEN o usuário cria uma sessão THEN o sistema SHALL permitir atribuir a ela um nome visível que identifique seu contexto.
13. WHEN o usuário cria ou renomeia uma sessão THEN o sistema SHALL rejeitar nomes já usados por outra sessão sem diferenciar maiúsculas de minúsculas.
14. WHEN o usuário renomeia uma sessão THEN o sistema SHALL exigir que aquela sessão esteja desbloqueada.
15. WHEN o aplicativo é iniciado THEN o sistema SHALL apresentar uma tela de desbloqueio de entrada que exige a senha mestra global (GMP) antes de liberar acesso a qualquer sessão global.
16. WHEN a GMP é desbloqueada THEN o sistema SHALL abrir de uma vez todas as sessões globais e manter bloqueadas as sessões com senha própria até que suas respectivas senhas sejam fornecidas.
17. WHEN o usuário alterna uma sessão entre global e própria THEN o sistema SHALL exigir a senha atual apropriada — a GMP para converter uma sessão global e a senha própria para converter uma sessão com senha própria — e re-derivar a proteção para o novo `auth_mode`.
18. WHEN uma sessão é persistida ou sincronizada THEN o sistema SHALL autenticar seu `auth_mode` (global ou própria) de forma que não possa ser rebaixado de própria para global sem a chave correta.

**Independent Test:** Desbloquear o aplicativo pela GMP e comprovar que todas as sessões globais abrem de uma vez enquanto as sessões com senha própria permanecem bloqueadas; criar sessões nomeadas “Trabalho” e “Pessoal”, rejeitar uma terceira chamada “trabalho”, renomear somente uma sessão desbloqueada; configurar políticas diferentes, comprovar cronômetros independentes e validar os eventos do Windows; comprovar que uma sessão com senha própria exige a senha dela mesmo com a GMP aberta, que a alternância global↔própria exige a senha atual apropriada e que o `auth_mode` não pode ser rebaixado sem a chave correta, sem conteúdo legível nos artefatos persistidos previstos pelo teste.

---

### P1: Proteger a senha mestra e as tentativas de acesso ⭐ MVP

**User Story:** Como usuário, quero orientação para criar senhas mestras fortes — tanto a senha mestra global (GMP) quanto as senhas próprias de sessão — e proteção contra tentativas repetidas para reduzir o risco de acesso indevido.

**Why P1:** O acesso ao aplicativo depende da GMP e cada sessão com senha própria depende exclusivamente da sua senha no v1.

**Acceptance Criteria:**

1. WHEN uma senha mestra (a GMP ou a senha própria de uma sessão) é criada ou trocada THEN o sistema SHALL exigir o comprimento mínimo definido e exibir um indicador de força compreensível.
2. WHEN tentativas incorretas se repetem, tanto na tela de desbloqueio da GMP quanto em uma sessão com senha própria, THEN o sistema SHALL aplicar atraso progressivo sem apagar automaticamente a sessão nem o cofre.
3. WHEN o usuário configura uma sessão THEN o sistema SHALL permitir uma dica de senha, sincronizá-la entre dispositivos como metadado não secreto para a aplicação e não tratá-la como autenticação ou recuperação.
4. WHEN a sessão está bloqueada THEN o sistema SHALL revelar a dica somente após a ação explícita “Mostrar dica”.
5. WHEN o usuário cria ou altera a dica THEN o sistema SHALL avisar que ela é visível sem senha e não deve conter a senha mestra nem partes óbvias dela.
6. WHEN o usuário troca uma senha mestra THEN o sistema SHALL exigir a senha atual válida; em particular, trocar a GMP SHALL exigir a GMP atual e trocar a senha própria de uma sessão SHALL exigir a senha própria atual.
7. WHEN o usuário utiliza o produto ao longo do tempo THEN o sistema SHALL avisar periodicamente que o v1 não possui recuperação de acesso — nem para a GMP nem para as senhas próprias — deixando a cadência exata para decisão futura.
8. WHEN o usuário esquece uma senha THEN o sistema SHALL informar a ausência de recuperação no v1 e não oferecer atalho que contorne a senha; perder a GMP SHALL tornar todas as sessões globais inacessíveis e perder a senha própria SHALL tornar a sessão correspondente inacessível.

**Independent Test:** Validar os limites de senha, o indicador de força e o atraso progressivo; sincronizar a dica, comprovar que ela só aparece após “Mostrar dica” e que o aviso de exposição é apresentado; validar a troca com senha atual e a impossibilidade de troca ou acesso sem a senha correta.

---

### P1: Gerenciar segredos ⭐ MVP

**User Story:** Como usuário, quero criar, localizar, consultar, alterar e excluir segredos para centralizar minhas credenciais.

**Why P1:** É a função central do produto.

**Acceptance Criteria:**

1. WHEN uma sessão está desbloqueada THEN o sistema SHALL permitir registros de senha, API key, token genérico, nota secreta e chave SSH.
2. WHEN um registro é salvo ou alterado THEN o sistema SHALL persistir seu conteúdo dentro da proteção criptográfica autenticada do cofre.
3. WHEN uma sessão está bloqueada THEN o sistema SHALL impedir consulta, pesquisa, cópia ou alteração de qualquer segredo daquela sessão.
4. WHEN um valor é copiado THEN o sistema SHALL avisar o usuário e solicitar ao sistema a limpeza automática após período configurável, usando 5 minutos como padrão.
5. WHEN houver conteúdo sensível copiado THEN o sistema SHALL oferecer a ação “Limpar agora” para solicitar sua remoção imediata do clipboard.
6. WHEN uma entrada é excluída THEN o sistema SHALL refletir a exclusão no estado local e na sincronização sem prometer apagamento físico impossível de backups ou versões do provedor.
7. WHEN uma pesquisa é executada THEN o sistema SHALL consultar simultaneamente somente as sessões desbloqueadas às quais o usuário tem acesso naquele momento.
8. WHEN um segredo é movido entre sessões THEN o sistema SHALL exigir que origem e destino estejam desbloqueados.

**Independent Test:** Executar o ciclo CRUD de cada tipo, bloquear o cofre e verificar que nenhuma operação sensível permanece disponível.

---

### P1: Sincronizar via nuvem ⭐ MVP

**User Story:** Como usuário com vários computadores, quero sincronizar o cofre pelo OneDrive ou Google Drive para acessar meus segredos sem expô-los ao provedor.

**Why P1:** Sincronização multidispositivo faz parte do v1 acordado.

**Acceptance Criteria:**

1. WHEN o usuário escolhe um provedor THEN o sistema SHALL executar OAuth 2.0 com escopos mínimos e permitir desconexão/revogação.
2. WHEN dados são enviados ao provedor THEN o sistema SHALL enviar somente conteúdo cifrado, autenticado e metadados mínimos necessários.
3. WHEN a rede ou provedor está disponível THEN o sistema SHALL sincronizar automaticamente os blobs cifrados inclusive de sessões bloqueadas, sem descriptografá-las; durante indisponibilidade SHALL manter o cofre local utilizável e retomar sem perda ao recuperar conectividade.
4. WHEN o sistema sincroniza THEN ele SHALL executar operações conceituais de envio e obtenção de mudanças (push/pull), sem exigir que o armazenamento interno use Git.
5. WHEN o modo somente leitura está ativo para uma sessão em um dispositivo THEN o sistema SHALL permitir visualizar os segredos acessíveis e impedir que aquele dispositivo produza alterações nesses segredos.
6. WHEN uma sessão está em modo somente leitura THEN o sistema SHALL continuar recebendo atualizações remotas normalmente.
7. WHEN o usuário tenta habilitar edição para uma sessão em modo somente leitura THEN o sistema SHALL exigir sua senha mestra válida.
8. WHEN dispositivos alteram campos concorrentemente THEN o sistema SHALL tentar uma mesclagem automática segura; caso não seja possível, SHALL preservar as versões e exigir resolução explícita campo a campo, sem sobrescrita silenciosa.
9. WHEN um conflito manual é apresentado THEN o sistema SHALL oferecer por campo as ações “manter local”, “manter remoto” e “manter ambos”.
10. WHEN “manter ambos” é escolhido para um campo de valor único THEN o sistema SHALL criar entradas separadas para preservar os dois resultados.
11. WHEN um conflito permanece sem resolução THEN o sistema SHALL preservar suas versões por 30 dias e, durante os 7 dias finais, manter um aviso persistente e emitir uma notificação diária sobre a expiração.
12. WHEN um conflito completa 30 dias sem resolução THEN o sistema SHALL encerrar a pendência transformando todas as versões em entradas permanentes identificadas por origem como “local” e “remota”, sem descartar nenhuma delas.
13. WHEN houver múltiplas versões da mesma origem ao materializar um conflito expirado THEN o sistema SHALL numerar somente essas entradas como “local 1”, “local 2”, “remota 1”, “remota 2” e assim por diante.
14. WHEN uma resolução manual é concluída THEN o sistema SHALL preservar as versões anteriores por mais 7 dias e permitir desfazer durante esse período.
15. WHEN conteúdo remoto está truncado, corrompido ou não autenticado THEN o sistema SHALL rejeitá-lo e preservar a última cópia local válida.
16. WHEN o provedor apresenta uma versão anterior ou repetida THEN o sistema SHALL detectar ou sinalizar possível rollback/replay conforme as garantias definidas no modelo de ameaças.

**Independent Test:** Sincronizar uma sessão bloqueada sem descriptografá-la; validar o modo somente leitura; criar conflitos de campo único e múltiplas versões locais/remotas; comprovar aviso persistente e notificação diária nos 7 dias finais, materialização permanente e numeração condicional após 30 dias, além do prazo de 7 dias para desfazer uma resolução; injetar blob adulterado e confirmar rejeição.

---

### P1: Atualizar com autenticidade ⭐ MVP

**User Story:** Como usuário, quero receber atualizações pelo GitHub para corrigir vulnerabilidades sem instalar código não autorizado.

**Why P1:** Atualização segura é parte da postura de segurança e manutenção do v1.

**Acceptance Criteria:**

1. WHEN uma atualização é publicada THEN o sistema SHALL verificar a autenticidade exigida pelo updater antes da instalação.
2. WHEN manifesto, assinatura ou pacote é inválido THEN o sistema SHALL rejeitar a atualização e manter a versão instalada funcional.
3. WHEN uma atualização falha THEN o sistema SHALL informar o erro sem incluir segredos ou tokens em logs.
4. WHEN uma versão é mais antiga que a instalada THEN o sistema SHALL bloquear ou alertar contra downgrade conforme política definida no design.
5. WHEN o pipeline de release é executado THEN o sistema SHALL manter chaves privadas de assinatura fora do repositório e dos artefatos públicos.

**Independent Test:** Aceitar uma atualização de teste válida e rejeitar pacotes adulterados, manifestos inválidos e tentativa de downgrade.

---

### P2: Hardening e transparência de segurança

**User Story:** Como usuário e auditor, quero conhecer as garantias e limites do produto para avaliar se ele atende ao meu risco.

**Why P2:** Essencial para uma versão estável, embora possa evoluir depois da primeira vertical funcional.

**Acceptance Criteria:**

1. WHEN o projeto declarar uma ameaça mitigada THEN o sistema SHALL possuir requisito, controle e evidência de teste ou revisão correspondentes.
2. WHEN logs ou relatórios forem produzidos THEN o sistema SHALL aplicar redaction e não incluir segredos, senhas mestras, chaves descriptográficas ou tokens OAuth.
3. WHEN o usuário consultar a documentação THEN o sistema SHALL apresentar ameaças mitigadas, aceitas e fora do modelo em linguagem compreensível.
4. WHEN uma dependência ou release for preparado THEN o processo SHALL executar os gates de segurança definidos para aquela etapa.
5. WHEN o modelo de ameaças for elaborado THEN ele SHALL avaliar adversários com acesso físico e técnico ao equipamento, distinguir ataques offline de host comprometido durante o uso e documentar controles, dependências do Windows/hardware e riscos residuais.

**Independent Test:** Auditar amostras de logs e documentação contra uma matriz de ameaças e evidências.

---

## Edge Cases

- WHEN o cofre está vazio THEN o sistema SHALL orientar a criação do primeiro segredo sem reduzir as proteções.
- WHEN armazenamento local fica sem espaço durante uma gravação THEN o sistema SHALL preservar a última versão válida e reportar falha.
- WHEN o aplicativo encerra durante gravação ou sincronização THEN o sistema SHALL recuperar um estado autenticado sem aceitar gravação parcial.
- WHEN o relógio do dispositivo está incorreto THEN o sistema SHALL evitar depender apenas do horário local para integridade ou ordenação de conflitos.
- WHEN o token OAuth expira ou é revogado THEN o sistema SHALL manter acesso local e solicitar nova autorização sem apagar dados.
- WHEN o usuário troca de provedor THEN o sistema SHALL evitar excluir a única cópia válida e exigir confirmação das etapas destrutivas.
- WHEN o clipboard não puder ser limpo de forma confiável THEN o sistema SHALL informar a limitação sem afirmar que o conteúdo foi apagado.
- WHEN múltiplas instâncias acessam o cofre THEN o sistema SHALL impedir corrupção ou coordenar acesso de forma segura.
- WHEN dados excedem limites definidos THEN o sistema SHALL rejeitar de forma controlada, sem consumo ilimitado de memória ou disco.
- WHEN a interface renderiza conteúdo controlado pelo usuário THEN o sistema SHALL tratá-lo como dado não confiável e impedir execução/injeção.

---

## Requirement Traceability

| Requirement ID | Story | Phase | Status | Artefato de design |
| --- | --- | --- | --- | --- |
| VAULT-01 | Criar e desbloquear | Design | In Design | [local-sessions](../local-sessions/design.md), [crypto-format](../crypto-format/design.md) |
| VAULT-02 | Bloqueio e proteção local | Design | In Design | [local-sessions](../local-sessions/design.md) |
| VAULT-03 | Isolamento e políticas independentes entre sessões | Design | In Design | [local-sessions](../local-sessions/design.md), [crypto-format](../crypto-format/design.md) (KEY-01) |
| VAULT-04 | Política de senha mestra, dicas e tentativas | Design | In Design | [local-sessions](../local-sessions/design.md), [crypto-format](../crypto-format/design.md) (ROT-01) |
| VAULT-05 | Senha mestra global e modo de autenticação por sessão (global/própria) | Design | In Design | [local-sessions](../local-sessions/design.md) (GMP/`auth_mode`), [crypto-format](../crypto-format/design.md) (GKEY-01/02) |
| SECRET-01 | Tipos e CRUD de segredos | Tasks | In Tasks | [secret-management](../secret-management/tasks.md) |
| SECRET-02 | Clipboard e exposição transitória | Tasks | In Tasks | [secret-management](../secret-management/tasks.md) |
| SECRET-03 | Pesquisa e movimentação entre sessões | Tasks | In Tasks | [secret-management](../secret-management/tasks.md) |
| SYNC-01 | OAuth com escopo mínimo | Design | Pending | — |
| SYNC-02 | Conteúdo remoto zero-knowledge | Design | Pending | [crypto-format](../crypto-format/spec.md) (INTEG-01, campos reservados; sync adiada) |
| SYNC-03 | Offline, conflitos e integridade | Design | Pending | — |
| SYNC-04 | Rollback e replay remoto | Design | Pending | — |
| SYNC-05 | Sincronização automática e modo somente leitura | Design | Pending | — |
| SYNC-06 | Resolução e expiração de conflitos | Design | Pending | — |
| UPDATE-01 | Autenticidade de atualização | Design | Pending | — |
| UPDATE-02 | Downgrade e falha segura | Design | Pending | — |
| SEC-01 | Modelo de ameaças e evidências | Design | Em revisão | [threat-model](./threat-model.md) — reaberto por AD-022 (GMP), re-aprovação pendente |
| SEC-02 | Redaction e privacidade operacional | Design | Pending | — |
| SEC-03 | Acesso físico, ataques offline e limites do hardware | Design | Em revisão | [threat-model](./threat-model.md) — reaberto por AD-022 (GMP), re-aprovação pendente |
| RESIL-01 | Gravação atômica e recuperação | Design | Pending | — |

**Status values:** Pending → In Design → In Tasks → Implementing → Verified. `Em revisão` = artefato existia como aprovado e foi reaberto por decisão posterior (AD-022), aguardando nova aprovação humana antes de voltar a ser base estável.

**Coverage:** 20 requisitos — 3 em Tasks (SECRET-01…03, mapeados por [secret-management](../secret-management/tasks.md)), 5 em Design (VAULT-01…05, cobertos por [local-sessions](../local-sessions/design.md) + [crypto-format](../crypto-format/design.md)), 2 em revisão (SEC-01, SEC-03, por [threat-model](./threat-model.md) reaberto em AD-022) e 10 pendentes. O mockup visual [ui-screens](../ui-screens/tasks.md) (`Done`) cobre a *superfície de tela* de VAULT/SECRET/SYNC/UPDATE, porém sem lógica, criptografia ou persistência — não conta como implementação de nenhum requisito funcional acima.

---

## Success Criteria

- [ ] Todos os cenários P1 passam em Windows suportado sem segredo legível nos artefatos persistidos cobertos pelos testes.
- [ ] Alteração de qualquer byte em um cofre ou blob sincronizado protegido é detectada antes de seu uso.
- [ ] Testes de conflito demonstram ausência de perda silenciosa entre dois dispositivos.
- [ ] Sessões com senha própria e políticas diferentes mantêm estados de bloqueio independentes, sem desbloqueio transitivo; sessões globais, por sua vez, abrem juntas ao desbloquear a senha mestra global (GMP).
- [ ] O usuário cria múltiplas sessões persistentes com nomes únicos sem distinção de maiúsculas/minúsculas e renomeia somente sessões desbloqueadas.
- [ ] Novas sessões usam 15 minutos, reiniciam apenas por interação intencional na própria sessão, bloqueiam por padrão com bloqueio/suspensão do Windows e sempre bloqueiam ao fechar o aplicativo.
- [ ] Senhas fracas abaixo do mínimo são rejeitadas, tentativas repetidas sofrem atraso progressivo e nenhuma troca ocorre sem a senha atual.
- [ ] Dicas sincronizam como metadado não secreto, aparecem somente sob demanda na tela bloqueada e incluem aviso para não revelar a senha.
- [ ] O clipboard usa 5 minutos por padrão, aceita outro período configurado e oferece limpeza imediata, com limitações do sistema claramente informadas.
- [ ] Conflitos oferecem decisão campo a campo; após 30 dias viram entradas permanentes locais/remotas, com aviso persistente e diário nos 7 dias finais, sem perda de versões.
- [ ] Resoluções manuais podem ser desfeitas por 7 dias e “manter ambos” cria entradas separadas para valores únicos.
- [ ] Sessões bloqueadas sincronizam somente blobs cifrados; o modo somente leitura é independente por sessão e dispositivo, recebe atualizações e exige senha mestra para habilitar edição.
- [ ] Atualizações adulteradas e tentativas de downgrade cobertas pela política são rejeitadas.
- [ ] Cada ameaça declarada como mitigada possui controle rastreável e evidência verificável.
- [ ] Uma revisão independente de criptografia e cadeia de atualização é concluída antes da versão estável.
