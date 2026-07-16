# Mockup Visual das Telas — Decisões de Contexto

Decisões do usuário que resolvem áreas cinzentas e alteram o modelo de autenticação assumido pela spec de produto atual. Aplicam-se ao mockup (spec, design, tasks) e devem, se aprovado, propagar para os specs de produto.

---

## D-01 — Senha mestra global (padrão) + senha por sessão (opt-out)

**Decisão:** existe **uma senha mestra global** que protege o app. Por padrão, toda sessão usa a senha global. O usuário pode marcar uma sessão para **usar senha própria** — nesse caso a sessão não é aberta pela senha global.

**Por quê:** o usuário quer conveniência de uma senha única no dia a dia, mantendo a opção de isolar sessões sensíveis.

## D-02 — A senha global trava o app inteiro

**Decisão:** após o login com Google, o app exibe uma **tela de desbloqueio global**. Sem a senha global, nada do app é acessível. No primeiro uso, essa tela é de **criação** da senha global.

## D-03 — Desbloqueio global abre todas as sessões globais de uma vez

**Decisão:** digitar a senha global **desbloqueia simultaneamente** todas as sessões que usam a senha global. Sessões com senha própria permanecem bloqueadas até que a senha própria seja informada.

**Consequência de segurança (aceita conscientemente):** isso **remove o isolamento** ("desbloqueio transitivo") entre as sessões que compartilham a senha global — o oposto da garantia atual de `secure-vault/spec.md` (VAULT-03 / "sem desbloqueio transitivo"). Sessões com senha própria mantêm isolamento.

---

## D-04 — Modelo criptográfico canônico (global + opt-out)

Modelo de referência para propagar a mudança de forma consistente aos specs de produto. Preserva o envelope por sessão do [crypto-format](../crypto-format/design.md) e adiciona uma camada global.

**Vocabulário:**
- **GMP** — senha mestra global. **gKEK** — KEK derivada da GMP por Argon2id(salt_global). **GMK** — *global master key* aleatória (raiz global), envolvida pela gKEK.
- **Sessão global** — `auth_mode = global`: a `root_key` da sessão é envolvida por uma chave derivada da **GMK**. **Sessão própria** — `auth_mode = own`: a `root_key` é envolvida por uma KEK derivada da **senha própria** daquela sessão (modelo por-sessão atual).

**Keyring global** (novo arquivo, ex.: `keyring.vault`): envelope autenticado com `format_version`, `salt_global`, params de KDF, `aead_id` e `wrapped_gmk = AEAD(gKEK, GMK, aad = header)`. A GMK/gKEK **nunca** são persistidas em claro (mesmas regras de KDF-01/C-02).

**Wrap da root_key por sessão:**
- `global` → `wrapped_root_key = AEAD(K_sessao, root_key, aad = header)`, com `K_sessao = HKDF(GMK, info = "ssv:session-wrap:v1:" ‖ session_uuid)`.
- `own` → `wrapped_root_key = AEAD(KEK_propria, root_key, aad = header)`, com `KEK_propria = Argon2id(senha_propria, salt_da_sessão)`.
- **`auth_mode` vai na AAD do header** — virar `own`↔`global` sem a chave correta quebra a autenticação (impede rebaixar uma sessão própria para abrir com a GMK).

**Fluxos:**
- **Desbloquear app (T04):** derivar gKEK ← GMP, *unwrap* da GMK; então *unwrap* das `root_key` de **todas** as sessões `global` (abrem juntas — D-03). Sessões `own` seguem bloqueadas.
- **Criar sessão (T07):** padrão `global` (envolve a root_key com a GMK, sem pedir senha nova); toggle → `own` (pede senha própria + força + dica).
- **Alternar global↔own (T12):** reenvelope da `root_key` (como ROT-01); exige a senha/chave atual apropriada. Conteúdo cifrado não muda.
- **Trocar GMP (T16):** rederivar gKEK', re-*wrap* da **mesma** GMK; sessões e conteúdos não mudam.
- **Primeiro uso (T03):** gerar `salt_global` + GMK aleatória, derivar gKEK ← GMP, gravar o keyring.

**Consequência de segurança (aceita — D-03):** conhecer/quebrar a **GMP expõe todas as sessões `global` de uma vez** (raio de exposição maior que o modelo por-sessão). Sessões `own` mantêm isolamento total. Isso **contradiz** a garantia atual "sem desbloqueio transitivo" (C-01 / VAULT-03) e **reabre o modelo de ameaças** (ver D-05).

## D-05 — Re-aprovação do modelo de ameaças

O `secure-vault/threat-model.md` está **Aprovado** com a premissa de isolamento total entre sessões. Introduzir a GMP altera objetivos, ativos, adversários e controles (C-01, C-13) e o raio de exposição de T-AUTH-*. A propagação portanto **rebaixa o status para "Revisão pendente"** nos pontos afetados e **exige nova aprovação humana** antes de virar base de design. Nenhuma mudança aqui certifica os controles.

---

## Impacto a propagar (feito nesta rodada)

- **`secure-vault/spec.md`**: VAULT-01…04 + success criteria reescritos para GMP + opt-out.
- **`crypto-format/spec.md` + `design.md`**: keyring global, GMK, wrap por `auth_mode`, fluxos de app-unlock e troca de GMP.
- **`secure-vault/threat-model.md`**: objetivos, ativos, matriz de ameaças, controles e status de aprovação.
- **`local-sessions/design.md`**: gate de app-unlock, `auth_mode`, comandos novos.

> A propagação segue o modelo canônico D-04 e o gate de re-aprovação D-05.
