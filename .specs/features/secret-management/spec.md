# Gerenciamento de segredos — Especificação

**Milestone:** M1 — Cofre local utilizável
**Status:** Approved — 2026-07-19
**Parent requirements:** `SECRET-01`, `SECRET-02` e `SECRET-03` de [secure-vault/spec.md](../secure-vault/spec.md)
**Dependency:** sessões locais persistentes e criptografadas; a execução não pode usar o store frontend-only como fonte de verdade

## Problem Statement

O aplicativo já apresenta telas para listar, criar e visualizar segredos, mas elas ainda usam fixtures ou navegação simulada e não realizam CRUD. O usuário precisa administrar os cinco tipos de segredo do v1 dentro de sessões desbloqueadas, com persistência autenticada no core, pesquisa limitada às sessões acessíveis, movimentação sem perda e exposição transitória controlada.

## Goals

- [ ] Criar, consultar, editar e excluir os cinco tipos de segredo do v1.
- [ ] Persistir alterações somente dentro do payload cifrado e autenticado da sessão.
- [ ] Impedir qualquer leitura ou mutação quando a sessão não estiver autorizada.
- [ ] Pesquisar apenas dados permitidos de sessões atualmente desbloqueadas.
- [ ] Mover segredos entre sessões desbloqueadas sem duplicação ou perda parcial.
- [ ] Revelar e copiar valores sensíveis somente por ação explícita, com limpeza configurável do clipboard.
- [ ] Remover fixtures e `localStorage` como fontes de verdade sem migrar dados fictícios.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Campos personalizados | Não fazem parte dos cinco esquemas definidos para o v1. |
| TOTP, anexos, cartões e identidades | Fora do escopo do v1. |
| Gerador de senhas | Pode ser adicionado como feature independente. |
| Histórico e lixeira de segredos | A retenção sincronizada pertence ao desenho posterior de versionamento/sync. |
| Compartilhamento entre usuários | Fora do v1 individual. |
| Importação ou exportação | Fluxos próprios, com formato e riscos distintos. |
| OAuth, sincronização e conflitos remotos | Pertencem ao M2; esta spec altera somente o estado local. |
| Implementar sessões locais | É dependência desta feature, não parte dela. |
| Migrar fixtures ou dados do store placeholder | Esses dados nunca foram considerados um cofre seguro. |

---

## Secret Types

Todos os tipos possuem `id`, `type`, `name`, `created_at`, `updated_at` e revisão monotônica. `name` é obrigatório, mas não precisa ser único dentro da sessão.

| Tipo | Campos do v1 | Valores sensíveis por padrão |
| --- | --- | --- |
| Senha | usuário, senha, URL e notas | senha e notas |
| API key | chave, ambiente e escopos | chave |
| Token | valor, expiração opcional e notas | valor e notas |
| Nota secreta | texto | texto |
| Chave SSH | chave pública opcional, chave privada e passphrase opcional | chave privada e passphrase |

Pelo menos um campo de conteúdo apropriado ao tipo deve ser informado. Limites exatos de bytes e normalização pertencem ao Design, mas devem ser validados no core antes de duplicar buffers, criptografar ou gravar.

## User Stories

### P1: Executar o CRUD dos cinco tipos ⭐ MVP

**User Story:** Como usuário, quero criar, consultar, alterar e excluir segredos dentro de uma sessão para centralizar minhas credenciais.

**Why P1:** É a função central do cofre.

**Acceptance Criteria:**

1. WHEN uma sessão autorizada está desbloqueada THEN o sistema SHALL permitir criar registros dos cinco tipos definidos nesta spec.
2. WHEN um segredo é criado ou alterado THEN o core SHALL validar tipo, campos, tamanhos e revisão antes de modificar o payload da sessão.
3. WHEN uma alteração é confirmada THEN o sistema SHALL persistir o payload completo dentro do envelope cifrado e autenticado da sessão, sem gravar uma cópia legível.
4. WHEN uma edição usa revisão obsoleta THEN o sistema SHALL rejeitá-la sem sobrescrever uma alteração mais recente.
5. WHEN o usuário confirma a exclusão THEN o sistema SHALL remover o registro do estado local confirmado sem prometer apagamento físico de backups ou versões futuras.
6. WHEN uma gravação falha THEN o sistema SHALL preservar a última versão válida do vault e não confirmar a alteração na interface.
7. WHEN a sessão está bloqueada ou sua epoch mudou THEN o sistema SHALL negar create, read, update e delete, inclusive para operações iniciadas anteriormente.

**Independent Test:** Em storage temporário, executar CRUD de cada tipo, reiniciar o manager, validar o conteúdo recuperado e injetar lock, revisão concorrente e falha de gravação antes do commit.

**Requirements:** SECMGMT-01…06

---

### P1: Consultar valores com exposição controlada ⭐ MVP

**User Story:** Como usuário, quero visualizar metadados e revelar valores sensíveis somente quando necessário.

**Why P1:** O segredo precisa ser utilizável sem permanecer exposto por padrão.

**Acceptance Criteria:**

1. WHEN uma lista de segredos é exibida THEN o sistema SHALL retornar somente identificador, tipo, nome e metadados necessários à listagem, sem valores sensíveis.
2. WHEN o detalhe é aberto em sessão desbloqueada THEN o sistema SHALL mascarar campos sensíveis por padrão.
3. WHEN o usuário solicita revelar um campo THEN o sistema SHALL revelar somente aquele valor e somente enquanto a sessão e a view continuam autorizadas.
4. WHEN ocorre lock, troca de sessão, navegação para fora, fechamento da view ou invalidação de epoch THEN a interface SHALL remover imediatamente valores revelados do DOM e do estado.
5. WHEN um erro ocorre THEN respostas, logs, panic, URLs e storage da WebView SHALL omitir nomes de campos sensíveis e seus valores.

**Independent Test:** Usar canários distintos por campo, revelar um de cada vez e verificar sua ausência após cada evento de limpeza e em todos os canais de saída observados.

**Requirements:** SECMGMT-07…09

---

### P1: Pesquisar somente sessões acessíveis ⭐ MVP

**User Story:** Como usuário, quero localizar segredos nas sessões desbloqueadas sem expor sessões bloqueadas.

**Why P1:** A busca é necessária para uso cotidiano com múltiplas sessões.

**Acceptance Criteria:**

1. WHEN uma pesquisa é executada THEN o core SHALL consultar somente sessões desbloqueadas e autorizadas no início e no commit da operação.
2. WHEN uma sessão bloqueia durante a pesquisa THEN resultados provenientes dela SHALL ser descartados antes da resposta.
3. WHEN uma sessão está bloqueada THEN sua existência MAY contribuir apenas para a lista normal de sessões, nunca para resultados ou contagens da pesquisa de segredos.
4. WHEN a consulta é vazia THEN o sistema SHALL retornar a listagem permitida pela paginação, sem consultar conteúdo de sessões bloqueadas.
5. WHEN a consulta é processada THEN o sistema SHALL não persistir seu texto em histórico, telemetria, URL ou logs.

**Independent Test:** Pesquisar com duas sessões abertas e uma bloqueada, bloquear uma origem durante a operação e confirmar resultados, contagens e canais de saída.

**Requirements:** SECMGMT-10, SECMGMT-11

---

### P1: Mover segredos sem perda ⭐ MVP

**User Story:** Como usuário, quero mover um segredo entre sessões para reorganizar meus dados sem criar cópias acidentais.

**Why P1:** A organização entre contextos é parte do modelo de múltiplas sessões.

**Acceptance Criteria:**

1. WHEN um segredo é movido THEN origem e destino SHALL estar desbloqueados e autorizados durante toda a operação.
2. WHEN origem e destino são iguais THEN o sistema SHALL rejeitar a operação sem alterar estado.
3. WHEN a movimentação é confirmada THEN exatamente uma cópia lógica SHALL existir no destino e nenhuma na origem.
4. WHEN qualquer lock, conflito de revisão, serialização ou gravação falha THEN o sistema SHALL evitar confirmar um estado com perda do único registro.
5. WHEN o tipo é suportado no destino THEN todos os campos e timestamps aplicáveis SHALL ser preservados, registrando uma nova revisão.
6. WHEN a movimentação conclui THEN buffers intermediários legíveis SHALL ser descartados e não SHALL aparecer em respostas ou artefatos.

**Independent Test:** Mover cada tipo entre vaults temporários, injetar falha em cada fronteira de commit e verificar o invariante “uma cópia confirmada ou origem intacta”.

**Requirements:** SECMGMT-12, SECMGMT-13

---

### P1: Copiar e limpar o clipboard ⭐ MVP

**User Story:** Como usuário, quero copiar um valor sensível com aviso e limpeza automática para reduzir sua exposição.

**Why P1:** Copiar é um fluxo principal de uso e cria exposição fora do processo.

**Acceptance Criteria:**

1. WHEN o usuário copia um campo sensível THEN o sistema SHALL exigir sessão desbloqueada, escrever somente o valor solicitado e informar o prazo de limpeza.
2. WHEN nenhum prazo foi configurado THEN o sistema SHALL usar 5 minutos.
3. WHEN o prazo expira THEN o sistema SHALL solicitar limpeza somente se o clipboard ainda contiver o valor ou marcador de propriedade correspondente, sem apagar conteúdo posterior do usuário.
4. WHEN o usuário aciona “Limpar agora” THEN o sistema SHALL aplicar a mesma verificação de propriedade antes de limpar.
5. WHEN a sessão bloqueia THEN o sistema SHALL solicitar limpeza antecipada dos valores que ela colocou no clipboard.
6. WHEN o sistema operacional não permite confirmar a limpeza THEN a interface SHALL informar a limitação sem alegar erradicação.
7. WHEN um valor é copiado THEN o sistema SHALL não registrá-lo em notificações persistentes, logs, analytics ou estado durável.

**Independent Test:** Copiar um canário, substituir o clipboard por outro conteúdo antes do timeout, testar “Limpar agora”, lock e falha do adaptador, verificando que conteúdo posterior não é apagado.

**Requirements:** SECMGMT-14, SECMGMT-15

## Edge Cases

- WHEN um tipo ou campo desconhecido é recebido THEN o core SHALL rejeitá-lo sem descartar campos válidos já persistidos.
- WHEN um payload usa versão futura THEN o sistema SHALL falhar fechado e preservar o vault original.
- WHEN um valor excede o limite THEN o sistema SHALL rejeitá-lo antes de duplicar buffers ou iniciar criptografia.
- WHEN dois segredos têm o mesmo nome THEN o sistema SHALL permitir ambos e diferenciá-los por sessão, tipo e identificador.
- WHEN uma data de expiração é inválida THEN o sistema SHALL rejeitar o input sem alterar o registro anterior.
- WHEN uma chave SSH pública não corresponde à privada THEN o sistema MAY alertar ou rejeitar conforme decisão do Design, sem tentar corrigir silenciosamente.
- WHEN um segredo é removido enquanto sua tela está aberta THEN uma ação posterior SHALL falhar por revisão obsoleta.
- WHEN a paginação muda durante alterações concorrentes THEN o sistema SHALL evitar duplicar ou omitir silenciosamente registros na mesma resposta.
- WHEN o clipboard contém um valor igual colocado por outro aplicativo THEN o sistema SHALL usar o melhor mecanismo de propriedade disponível e documentar limites da plataforma.
- WHEN o frontend ainda possui fixtures THEN builds funcionais SHALL não apresentá-las como dados reais.

## Design Decisions

O Design resolve as escolhas abaixo; permanecem sujeitas à aprovação do documento:

1. **Busca:** pesquisar nome, tipo e metadados não sensíveis; valores, notas, tokens e chaves não entram no índice.
2. **Campos:** cinco esquemas fixos no v1, sem campos personalizados.
3. **Clipboard:** default de 5 minutos, opções limitadas e sem “nunca limpar”.
4. **Exclusão e revelação:** confirmação explícita; reveal granular, temporário e removido ao perder foco ou autorização.

## Requirement Traceability

| Requirement ID | Parent | Story | Phase | Status |
| --- | --- | --- | --- | --- |
| SECMGMT-01 | SECRET-01 | Tipos e modelo comum | Tasks | In Tasks |
| SECMGMT-02 | SECRET-01 | Criar e persistir | Tasks | In Tasks |
| SECMGMT-03 | SECRET-01 | Consultar e editar | Tasks | In Tasks |
| SECMGMT-04 | SECRET-01 | Excluir | Tasks | In Tasks |
| SECMGMT-05 | SECRET-01 | Revisão e concorrência | Tasks | In Tasks |
| SECMGMT-06 | SECRET-01 | Persistência fail-closed | Tasks | In Tasks |
| SECMGMT-07 | SECRET-01 | Listagem sem valores | Tasks | In Tasks |
| SECMGMT-08 | SECRET-01 | Reveal controlado | Tasks | In Tasks |
| SECMGMT-09 | SECRET-01 | Redaction e limpeza de UI | Tasks | In Tasks |
| SECMGMT-10 | SECRET-03 | Busca somente desbloqueadas | Tasks | In Tasks |
| SECMGMT-11 | SECRET-03 | Privacidade da consulta | Tasks | In Tasks |
| SECMGMT-12 | SECRET-03 | Movimentação autorizada | Tasks | In Tasks |
| SECMGMT-13 | SECRET-03 | Movimentação sem perda | Tasks | In Tasks |
| SECMGMT-14 | SECRET-02 | Cópia e limpeza | Tasks | In Tasks |
| SECMGMT-15 | SECRET-02 | Limites honestos do clipboard | Tasks | In Tasks |

**Coverage:** 15 requisitos definidos; `SECRET-01…03` cobertos; 15 em Tasks; 15/15 mapeados para 24 tarefas no rascunho.

## Success Criteria

- [ ] CRUD dos cinco tipos persiste e reabre corretamente em uma sessão local segura.
- [ ] Nenhuma operação sensível funciona após lock ou mudança de epoch.
- [ ] Listas e buscas nunca retornam valores secretos nem dados de sessões bloqueadas.
- [ ] Reveal é granular e removido em todos os eventos de perda de autorização.
- [ ] Movimentação mantém uma única cópia confirmada ou preserva integralmente a origem.
- [ ] Clipboard usa 5 minutos por padrão, não apaga conteúdo posterior e comunica limitações.
- [ ] Fixtures e `localStorage` deixam de ser fontes de verdade sem migração para vault real.
- [ ] Canários não aparecem em logs, URLs, storage da WebView, erros ou artefatos persistidos legíveis.
