# Formato Criptográfico Versionado — Especificação

**Milestone:** M0 — Fundação de segurança
**Depende de:** [Modelo de ameaças aprovado](../secure-vault/threat-model.md) (AD-019)
**Documentos relacionados:** [Cofre Seguro v1](../secure-vault/spec.md) · [Roadmap](../../project/ROADMAP.md) · [State](../../project/STATE.md)

> **Atenção — reabre o modelo de ameaças aprovado:** a introdução da senha mestra global (GMP) e do modo `auth_mode = global` altera o raio de exposição e a premissa de isolamento total entre sessões. Isso **rebaixa** os pontos afetados do [modelo de ameaças](../secure-vault/threat-model.md) para "Revisão pendente" e **exige nova aprovação humana** antes de virar base de design (ver [../ui-screens/context.md](../ui-screens/context.md) D-04/D-05). Nada aqui certifica os controles.

## Problem Statement

O modelo de ameaças define *requisitos* de criptografia (controles C-01…C-06, C-17), mas o produto ainda não tem um *formato* concreto: nenhuma decisão de derivação de senha, hierarquia de chaves, envelope autenticado, versionamento ou migração está fixada. Sem esse formato precisamente especificado e testável, não é possível implementar o cofre, gerar vetores de teste reproduzíveis nem submeter o desenho a revisão independente. Esta feature especifica o formato criptográfico versionado do v1 — as garantias, os campos autenticados e as regras de evolução — deixando explicitamente os *parâmetros numéricos finais* presos aos protótipos que o modelo de ameaças exige (PT-01, PT-02).

## Goals

- [ ] Definir derivação da senha mestra, hierarquia de chaves e envelope autenticado de modo que uma cópia do cofre não revele nada sem um ataque offline caro contra a senha daquela sessão.
- [ ] Definir um formato binário versionado, autoexplicativo e migrável, com todos os metadados de segurança autenticados antes de qualquer interpretação.
- [ ] Garantir isolamento criptográfico entre sessões, qualificado por `auth_mode`: sessões `own` permanecem plenamente isoladas (desbloquear uma não fornece material para outra); sessões `global` compartilham o domínio de confiança da senha mestra global (GMP) e abrem em conjunto no desbloqueio do app.
- [ ] Especificar rotação de chaves e troca de senha sem reescrever todo o conteúdo cifrado nem invalidar o cofre.
- [ ] Produzir vetores de teste reproduzíveis e um plano de revisão independente do formato.

## Out of Scope

| Feature | Reason |
| --- | --- |
| Parâmetros numéricos finais de KDF (memória, iterações, paralelismo) | Presos ao benchmark PT-01 em hardware suportado; a spec fixa a *estratégia*, não os números |
| Escolha final entre candidatos de AEAD e estratégia de nonce | Presa a PT-02 (vetores, teste de adulteração, revisão de misuse-resistance) |
| Lógica de merge, DAG e resolução de conflitos de sincronização | Pertence à feature de sincronização; aqui só definimos os *campos autenticados* que o formato transporta |
| Armazenamento/rotação de tokens OAuth | Feature de sincronização; pode reutilizar as primitivas, mas não faz parte do formato do cofre |
| Mecanismo completo de checkpoint/âncora anti-rollback | Decisão de design da sincronização; o formato apenas reserva e autentica os campos necessários (revisão, sequência por dispositivo, compromisso com pais) |
| Proteção de memória, zeroização e paginação | Feature "Prova de integração Windows e Tauri" (PT-04, PT-06) |
| Implementação em Rust do leitor/escritor | Fase Execute desta feature |
| Papel opcional de DPAPI/TPM na vinculação ao dispositivo | Decisão aberta (seção 12 #7); não pode ser a única chave do cofre e fica fora do formato base |

---

## User Stories

> Nesta feature os "usuários" do formato são: (a) o **core do aplicativo**, que lê e grava cofres; (b) o **auditor/revisor independente**, que verifica o desenho contra vetores; e (c) o **mantenedor futuro**, que precisa migrar cofres entre versões. As histórias descrevem invariantes verificáveis do formato.

### P1: Derivar acesso da senha mestra por sessão ⭐ MVP

**User Story:** Como core do aplicativo, quero derivar o material de chave de uma sessão a partir da sua senha mestra usando um KDF memory-hard, para que uma cópia offline do cofre só possa ser atacada a um custo alto e configurável.

**Why P1:** Toda confidencialidade do cofre depende da resistência da derivação a ataque offline (T-AUTH-01, T-STOR-01).

**Acceptance Criteria:**

1. WHEN uma sessão é criada THEN o formato SHALL registrar um salt aleatório por sessão, o identificador do algoritmo de KDF, seus parâmetros e a versão do formato dentro do envelope.
2. WHEN a senha mestra é derivada THEN o KDF SHALL ser memory-hard (Argon2id como candidato de referência) e produzir uma KEK que nunca é persistida.
3. WHEN o cofre é persistido THEN o formato SHALL NOT armazenar a senha mestra nem qualquer verificador barato que acelere ataque offline; a verificação de senha SHALL ocorrer pela decifragem autenticada do envelope de chaves.
4. WHEN os parâmetros de KDF são lidos THEN o core SHALL rejeitar valores fora dos limites defensivos definidos (mínimo seguro e máximo anti-DoS) antes de alocar memória.
5. WHEN dois cofres usam a mesma senha mestra THEN seus saltos e materiais derivados SHALL ser independentes por causa do salt por sessão.

**Independent Test:** Criar duas sessões com a mesma senha; confirmar saltos distintos, KEKs distintas e que nenhum campo do arquivo permite verificar a senha sem executar o KDF completo.

---

### P1: Derivar e envolver a chave mestra global ⭐ MVP

**User Story:** Como core do aplicativo, quero derivar uma chave de envelopamento global (gKEK) da senha mestra global (GMP) e usá-la para envolver uma chave mestra global aleatória (GMK) num keyring versionado, para que o desbloqueio do app abra em conjunto todas as sessões `global` sem persistir GMP, gKEK ou GMK em claro.

**Why P1:** O modo `auth_mode = global` depende de uma raiz global que só existe após unwrap autenticado da GMK; sem ela não há desbloqueio conjunto nem troca de GMP por reenvelope (T-AUTH-01, C-01/C-03).

**Acceptance Criteria:**

1. WHEN o keyring global é criado THEN o formato SHALL registrar um `salt_global` aleatório, o identificador e parâmetros do KDF, o `aead_id` e a versão do formato dentro do envelope do keyring.
2. WHEN a gKEK é derivada THEN o KDF SHALL ser memory-hard (Argon2id) sobre a GMP e o `salt_global`, e a gKEK SHALL NOT ser persistida.
3. WHEN a GMK é gerada THEN ela SHALL ser aleatória (independente da GMP) e persistida apenas como `wrapped_gmk = AEAD(gKEK, GMK, aad = header do keyring)`; GMK e gKEK SHALL NOT aparecer em claro.
4. WHEN o keyring é persistido THEN o formato SHALL NOT armazenar a GMP nem qualquer verificador barato; a verificação da GMP SHALL ocorrer pela decifragem autenticada do `wrapped_gmk`.
5. WHEN a GMP é trocada THEN o formato SHALL rederivar a gKEK' e reenvolver a **mesma** GMK, sem alterar as sessões nem seus conteúdos.
6. WHEN o app é desbloqueado com a GMP THEN o core SHALL, após unwrap da GMK, disponibilizar as `root_key` de **todas** as sessões `auth_mode = global` (abrem em conjunto), enquanto as sessões `auth_mode = own` SHALL permanecer inacessíveis até que a senha própria de cada uma seja fornecida.

**Independent Test:** Criar o keyring com uma GMP; confirmar que nenhum campo permite verificar a GMP sem executar o KDF, que a GMK só se obtém por unwrap autenticado, que desbloquear a GMP abre todas as sessões globais e nenhuma sessão own, e que trocar a GMP mantém a mesma GMK após reenvelope.

---

### P1: Envelopar chaves em hierarquia por propósito ⭐ MVP

**User Story:** Como core do aplicativo, quero uma hierarquia de chaves em que a KEK derivada da senha envolva uma chave raiz aleatória, da qual se derivam subchaves por propósito, para separar responsabilidades e permitir rotação.

**Why P1:** A hierarquia é o que permite trocar a senha sem reencriptar tudo e isola propósitos criptográficos (C-03).

**Acceptance Criteria:**

1. WHEN uma sessão é criada THEN o formato SHALL gerar uma chave raiz (`root_key`) aleatória independente da senha e envolvê-la (wrap) conforme o `auth_mode` autenticado no header: em `auth_mode = own`, com a `KEK_propria = Argon2id(senha_propria, salt_da_sessão)`; em `auth_mode = global`, com a `K_sessao = HKDF(GMK, info = "ssv:session-wrap:v1:" ‖ session_uuid)`.
2. WHEN subchaves são necessárias THEN elas SHALL ser derivadas da chave raiz por uma KDF de expansão (HKDF como candidato) com rótulo de propósito, versão e época distintos.
3. WHEN a chave raiz é envolvida THEN o wrap SHALL ser autenticado, de modo que uma chave de envelopamento incorreta — senha própria errada em `own` ou GMK ausente/incorreta em `global` — resulte em falha de autenticação, não em chave silenciosamente inválida.
4. WHEN o formato define subchaves THEN cada propósito (ex.: cifragem de conteúdo, autenticação de metadados, derivações futuras) SHALL usar rótulo separado para nunca reutilizar a mesma chave em contextos diferentes.
5. WHEN o `auth_mode` do header é adulterado THEN a autenticação SHALL falhar: por estar na AAD, rebaixar `own`→`global` sem a GMK correta, ou `global`→`own` sem a senha própria, não produz unwrap válido da `root_key`.

**Independent Test:** Decifrar o envelope com a chave de envelopamento correta e obter a chave raiz; confirmar que subchaves de propósitos diferentes são distintas, que uma senha/GMK errada falha na autenticação do wrap e que trocar o `auth_mode` no header rejeita.

---

### P1: Cifrar conteúdo e metadados com AEAD ⭐ MVP

**User Story:** Como core do aplicativo, quero cifrar cada objeto do cofre com criptografia autenticada cobrindo também seus metadados associados, para que nenhuma alteração de conteúdo ou de cabeçalho passe despercebida.

**Why P1:** Confidencialidade e detecção de adulteração são o núcleo do formato (T-STOR-02, T-SYNC-02).

**Acceptance Criteria:**

1. WHEN um objeto é cifrado THEN o formato SHALL usar um AEAD moderno sobre o plaintext, com dados associados (AAD) incluindo versão do formato, UUID da sessão, `auth_mode`, tipo e ID do objeto, época da chave, revisão e ancestralidade.
2. WHEN qualquer byte de ciphertext, tag ou AAD é alterado THEN a decifragem SHALL falhar e o dado adulterado SHALL NOT ser interpretado.
3. WHEN um objeto é cifrado THEN a estratégia de nonce SHALL garantir unicidade por chave (contador determinístico, nonce aleatório de tamanho seguro ou AEAD misuse-resistant), com a decisão final revisada em PT-02.
4. WHEN o parsing do envelope ocorre THEN a autenticação SHALL preceder qualquer alocação grande ou interpretação estrutural (autenticar antes de interpretar).

**Independent Test:** Gerar um objeto cifrado; alterar sistematicamente cada campo (versão, UUID, tipo, época, revisão, um byte do ciphertext, um byte da tag) e confirmar rejeição em todos os casos.

---

### P1: Versionar e migrar o formato ⭐ MVP

**User Story:** Como mantenedor futuro, quero um formato binário versionado e autoexplicativo, para evoluir algoritmos e estrutura sem corromper cofres existentes nem aceitar dados de versões desconhecidas de forma insegura.

**Why P1:** O produto vive em `0.x` e evoluirá; sem versionamento a migração vira reescrita destrutiva (seção 12 #4; C-17).

**Acceptance Criteria:**

1. WHEN um cofre é gravado THEN o formato SHALL iniciar por um cabeçalho contendo um magic identificador e a versão do formato, ambos incluídos na autenticação.
2. WHEN o core abre um cofre de versão suportada anterior THEN ele SHALL migrá-lo para a versão corrente por um caminho de migração definido, preservando os dados e sem perda silenciosa.
3. WHEN o core encontra uma versão de formato mais nova do que suporta THEN ele SHALL falhar de forma segura (fail-closed), informar o usuário e SHALL NOT tentar interpretar ou sobrescrever o cofre.
4. WHEN campos ou tipos desconhecidos aparecem dentro de uma versão suportada THEN o core SHALL preservá-los intactos onde a política de forward-compat permitir, sem descartá-los silenciosamente.
5. WHEN o formato é revisado THEN a mudança SHALL incrementar a versão e ser acompanhada de novos vetores de teste.

**Independent Test:** Abrir um cofre de versão anterior sintético e confirmar migração correta; apresentar um cofre com versão superior forjada e confirmar recusa fail-closed sem escrita.

---

### P1: Rotacionar chaves e trocar a senha mestra ⭐ MVP

**User Story:** Como core do aplicativo, quero trocar a senha mestra e rotacionar chaves reencriptando apenas o envelope de chaves (e, quando necessário, avançando a época), para não reescrever todo o conteúdo a cada troca.

**Why P1:** Troca de senha é requisito do v1 (VAULT-04) e a rotação limita o impacto de uma chave comprometida (C-03).

**Acceptance Criteria:**

1. WHEN a senha mestra é trocada THEN o formato SHALL rederivar a KEK a partir da nova senha e reenvolver a mesma chave raiz, sem alterar o conteúdo já cifrado.
2. WHEN a senha é trocada THEN o formato SHALL exigir prova da senha atual pela decifragem bem-sucedida do envelope atual antes de gravar o novo.
3. WHEN uma época de chave é avançada THEN o formato SHALL registrar a época nos metadados autenticados para que objetos novos usem a chave da época corrente e objetos antigos permaneçam decifráveis.
4. WHEN a troca de senha ou rotação é interrompida THEN o formato e a gravação SHALL preservar o último envelope válido, sem estado intermediário que impeça o desbloqueio.

**Independent Test:** Trocar a senha e confirmar que o conteúdo antigo continua decifrável com a chave raiz reenvolvida; interromper a troca no meio e confirmar recuperação do envelope válido anterior.

---

### P2: Transportar campos de integridade para sincronização

**User Story:** Como core do aplicativo, quero que o formato carregue e autentique os campos de identidade, revisão e ancestralidade de cada objeto, para que a sincronização detecte adulteração, reordenação e rollback sem que a lógica de merge viva no formato.

**Why P2:** A sincronização é outra feature, mas depende de o formato reservar e autenticar esses campos desde o v1 (T-SYNC-03, T-SYNC-04; C-06).

**Acceptance Criteria:**

1. WHEN um objeto é revisado THEN o formato SHALL atribuir um identificador de revisão autenticado e imutável.
2. WHEN um objeto tem predecessores THEN o formato SHALL registrar o compromisso com os pais (ancestralidade) dentro da AAD.
3. WHEN um dispositivo grava THEN o formato SHALL incluir um identificador de dispositivo e uma sequência monotônica por dispositivo, autenticados.
4. WHEN o formato reserva campos de frescor THEN ele SHALL permitir referenciar um checkpoint conhecido, deixando o mecanismo de âncora para a feature de sincronização.

**Independent Test:** Gerar duas revisões encadeadas; confirmar que a ancestralidade e a sequência por dispositivo estão na AAD e que alterá-las quebra a autenticação.

---

### P1: Vetores de teste e plano de revisão independente ⭐ MVP

**User Story:** Como auditor independente, quero vetores de teste reproduzíveis e um plano de revisão do formato, para verificar o desenho sem depender do restante do aplicativo.

**Why P1:** O modelo de ameaças exige evidência reproduzível (PT-02) e revisão independente antes da versão estável (gates de release).

**Acceptance Criteria:**

1. WHEN o formato é especificado THEN o projeto SHALL publicar vetores de teste determinísticos (entradas fixas, saltos e nonces fixos, saídas esperadas) para KDF, wrap de chaves e AEAD.
2. WHEN um vetor de adulteração é aplicado THEN o resultado esperado SHALL ser rejeição autenticada documentada.
3. WHEN a revisão independente é planejada THEN o plano SHALL definir escopo, entregáveis e critérios de aceitação da revisão do formato.
4. WHEN os vetores são executados em qualquer implementação conforme THEN os resultados SHALL bater exatamente com os valores publicados.

**Independent Test:** Rodar a suíte de vetores contra a implementação de referência e contra uma releitura independente; confirmar correspondência exata e rejeição de todos os vetores de adulteração.

---

## Edge Cases

- WHEN o gerador aleatório do sistema falha ou retorna entropia insuficiente THEN o formato/core SHALL abortar a criação da sessão em vez de gerar salt/nonce fracos.
- WHEN os parâmetros de KDF gravados excedem os limites máximos anti-DoS THEN o core SHALL recusar abrir o cofre e informar corrupção/incompatibilidade, sem alocar a memória solicitada.
- WHEN o arquivo do cofre está truncado antes do fim do envelope autenticado THEN o core SHALL rejeitá-lo e preservar a última cópia válida.
- WHEN um nonce se repetiria para a mesma chave THEN a estratégia escolhida SHALL tornar isso impossível ou seguro por construção.
- WHEN o cabeçalho declara um algoritmo desconhecido dentro de uma versão suportada THEN o core SHALL falhar de forma segura sem adivinhar primitiva.
- WHEN dois processos gravam o mesmo cofre THEN o formato SHALL permitir detectar escrita concorrente por revisão/sequência autenticadas (coordenação de locking fica no design de persistência).
- WHEN uma migração é interrompida THEN o core SHALL manter o cofre na versão original íntegra até a migração concluir atômica.

---

## Requirement Traceability

| Requirement ID | Story | Controles do modelo | Phase | Status |
| --- | --- | --- | --- | --- |
| KDF-01 | Derivar acesso da senha mestra | C-01, C-02 | Design | Pending |
| KDF-02 | Limites defensivos e ausência de verificador barato | C-02, C-03, C-17 | Design | Pending |
| KEY-01 | Chave raiz aleatória envolvida pela KEK | C-01, C-03 | Design | Pending |
| KEY-02 | Subchaves por propósito/época/versão | C-03 | Design | Pending |
| GKEY-01 | Keyring global: GMP→gKEK envolve GMK (raiz global) | C-01, C-02, C-03 | Design | Pending |
| GKEY-02 | Wrap da root_key por `auth_mode` autenticado (global via HKDF(GMK,uuid); own via senha própria) | C-01, C-03 | Design | Pending |
| AEAD-01 | AEAD sobre conteúdo e AAD de metadados | C-05 | Design | Pending |
| AEAD-02 | Estratégia de nonce e autenticar-antes-de-interpretar | C-05, C-17 | Design | Pending |
| FMT-01 | Cabeçalho versionado e autenticado | C-05, C-17 | Design | Pending |
| FMT-02 | Migração e fail-closed em versão superior | C-17 | Design | Pending |
| FMT-03 | Forward-compat de campos desconhecidos | C-17 | Design | Pending |
| ROT-01 | Rotação de chaves e troca de senha por reenvelope | C-03 | Design | Pending |
| INTEG-01 | Campos autenticados de revisão/ancestralidade/sequência | C-05, C-06 | Design | Pending |
| TEST-01 | Vetores de teste reproduzíveis | PT-02 | Design | Pending |
| TEST-02 | Plano de revisão independente do formato | Gates de release | Design | Pending |

**Coverage:** 15 requisitos, 0 mapeados para tarefas, 15 ainda não mapeados.

---

## Open Decisions (herdadas do modelo de ameaças §12)

Estas decisões permanecem abertas nesta spec e serão resolvidas em `context.md` (áreas cinzentas) e/ou `design.md` após os protótipos:

1. KDF e parâmetros finais (compatibilidade FIPS ou não) — bloqueado por PT-01.
2. AEAD, tamanho de nonce e estratégia de uso — bloqueado por PT-02.
3. Estrutura exata da hierarquia de chaves, épocas e rotação.
4. Representação binária versionada e limites máximos de cada campo.
5. Mecanismo de checkpoint/âncora e promessa exata anti-rollback (parcialmente na feature de sync).

---

## Success Criteria

- [ ] Uma cópia do cofre não permite verificar a senha nem obter qualquer chave sem executar o KDF completo daquela sessão.
- [ ] Alterar qualquer byte de ciphertext, tag ou metadado autenticado é detectado antes do uso.
- [ ] Duas sessões `own` com a mesma senha produzem materiais criptográficos independentes; sessões `own` permanecem isoladas, enquanto sessões `global` compartilham o domínio de confiança da GMP e abrem em conjunto no desbloqueio do app.
- [ ] Trocar a senha mestra mantém todo o conteúdo antigo decifrável sem reescrevê-lo.
- [ ] Trocar a GMP (reenvelope da mesma GMK no keyring) mantém todas as sessões `global` acessíveis sem reescrever conteúdo nem tocar nas sessões.
- [ ] Abrir uma versão superior à suportada falha de forma segura, sem escrita; versões anteriores suportadas migram sem perda.
- [ ] Existe uma suíte de vetores de teste determinísticos que qualquer implementação conforme reproduz exatamente, incluindo vetores de adulteração que devem ser rejeitados.
- [ ] Existe um plano de revisão independente do formato com escopo e critérios definidos.
