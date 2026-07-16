# Formato Criptográfico Versionado — Tasks

**Design:** [design.md](./design.md)
**Spec:** [spec.md](./spec.md)
**Status:** In Progress

## Execution Log

- **T1 — Fundação:** ✅ implementada e **compila**. `Cargo.toml` com as 10 crates candidatas (resolveram/baixaram sem conflito); `src/crypto/{mod,error,secret}.rs` criados; `pub mod crypto;` em `lib.rs`. `Key32` zeroizável (Drop + `zeroize`), `CryptoError` (thiserror, sem vazar segredo). `cargo test crypto::` → **1 teste passa** (`Key32`).
  - ⚠️ **Pendente de fechar T1:** rodar o gate completo `pnpm check:rust` (fmt + clippy `-D warnings`). O build emitiu um aviso `linker_messages` (nota do linker Windows sobre `.dll.lib/.exp`) — verificar se `-D warnings` o trata como erro e, se sim, decidir tratamento (allow no crate ou ignorar por ser mensagem de linker, não lint de código).
- **T2–T8:** ⬜ não iniciadas.
- **Próximo passo:** Fase 2 — `crypto::{kdf,keys,aead}` (T2/T3/T4).

---

> **Escopo desta quebra:** apenas a fatia **Sessões + desbloqueio** que o [design](./design.md) delimita — `crypto::{kdf,keys,aead,keyring,envelope}` + vetores de teste. Campos de sincronização (INTEG-01, história P2) ficam para a feature de sync. Parâmetros numéricos de KDF/AEAD entram como **candidatos** (`⚠️ PT-01/PT-02`), não finais.
>
> **Gate reaberto (AD-022 / D-05):** o modelo de ameaças está "em revisão". Este código implementa os **candidatos** do design para destravar a fatia; nada aqui certifica controles nem fixa parâmetros finais.

---

## Convenções desta fatia (todas as tarefas Rust)

- **Randomness injetável:** toda função que precisa de salt/nonce/chave aleatória recebe esses bytes **como parâmetro**. A geração via CSPRNG (`getrandom`/`OsRng`) fica num helper fino de produção. Isso torna os vetores de teste (T7) determinísticos sem `Math.random` no núcleo.
- **Autenticar antes de interpretar:** parsers validam magic + tamanho + limites, autenticam o AEAD e **só então** desserializam o conteúdo (AEAD-02).
- **Segredos zeroizáveis:** todo material de chave usa o tipo `Key32` (T1) com `Zeroize`/`Drop`; nada de chave/senha cruza o IPC (fronteira do design).
- **Clippy estrito:** o gate roda `clippy --all-targets --all-features -D warnings`. Código precisa ser warning-clean.
- **Verificação de API (Knowledge Chain Step 3):** confirmar a API atual das crates RustCrypto (`argon2`, `chacha20poly1305`, `hkdf`, `ciborium`) via Context7/docs antes de implementar — as assinaturas mudam entre versões.

---

## Execution Plan

### Phase 1 — Foundation (Sequential)

```
T1 (deps + módulo crypto + error + Key32)
```

### Phase 2 — Primitivos (Parallel)

```
        ┌→ T2 crypto::kdf     [P]
T1 ─────┼→ T3 crypto::keys    [P]
        └→ T4 crypto::aead    [P]
```

### Phase 3 — Envelopes (Parallel)

```
T2, T4 ──────────→ T5 crypto::keyring   [P]
T2, T3, T4 ──────→ T6 crypto::envelope  [P]
```

### Phase 4 — Vetores (Sequential)

```
T5, T6 → T7 crypto::vectors
```

### Phase 5 — Plano de revisão (Sequential)

```
T7 → T8 review-plan.md (doc — fecha a fatia)
```

---

## Task Breakdown

### T1: Fundação — dependências, módulo `crypto`, erro e `Key32`

**What:** adicionar as crates candidatas ao `Cargo.toml`, registrar `pub mod crypto;` em `lib.rs`, criar `crypto/mod.rs`, `crypto::error` (erros tipados com `thiserror`, sem vazar segredo) e `crypto::secret` (`Key32`: newtype de `[u8;32]` zeroizável no drop).
**Where:** `src-tauri/Cargo.toml` (modify), `src-tauri/src/lib.rs` (modify), `src-tauri/src/crypto/mod.rs` (new), `src-tauri/src/crypto/error.rs` (new), `src-tauri/src/crypto/secret.rs` (new)
**Depends on:** None
**Reuses:** `serde_json`/`thiserror` já no ecossistema; convenção de módulos do crate `secrets_storage_lib`
**Requirement:** base de KDF-01/AEAD-01 (tipos compartilhados)
**Tools:** Edit/Write · MCP: `context7` (resolver versões das crates) · Skill: NONE
**Done when:**
- [ ] `Cargo.toml` declara `argon2`, `chacha20poly1305`, `hkdf`, `sha2`, `zeroize`, `getrandom`, `ciborium`, `uuid` (v4), `serde` (derive), `thiserror` — versões resolvidas via Context7
- [ ] `crypto::error::CryptoError` cobre: KDF inválido/params fora dos limites, falha de autenticação (unwrap/open), versão superior (fail-closed), magic inválido, CBOR malformado — `Display` sem expor bytes de chave
- [ ] `Key32` zeroiza no `Drop`, não implementa `Debug`/`Display` que vaze conteúdo, e expõe `as_bytes`/construtor a partir de `[u8;32]`
- [ ] `+1` teste unitário: `Key32` constrói a partir de bytes e `as_bytes` retorna os mesmos
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T2: `crypto::kdf` — Argon2id (senha → KEK) + limites de parâmetros [P]

**What:** `derive_kek(password: &[u8], salt: &[u8], params: KdfParams) -> Result<Key32>` via Argon2id; `KdfParams { mem_kib, iters, parallelism }` com `validate()` que **rejeita antes de alocar** valores fora dos limites defensivos (mínimo seguro e máximo anti-DoS).
**Where:** `src-tauri/src/crypto/kdf.rs` (+ testes no mesmo arquivo `#[cfg(test)]`)
**Depends on:** T1
**Reuses:** `Key32`/`CryptoError` (T1), crate `argon2`
**Requirement:** KDF-01, KDF-02
**Tools:** Write · MCP: `context7` (API do `argon2`) · Skill: NONE
**Done when:**
- [ ] `derive_kek` é determinístico para (password, salt, params) fixos; salt/params vêm por parâmetro (injetável)
- [ ] Params candidatos documentados (`mem=64 MiB, iters=3, par=1` — `⚠️ PT-01`), **não** hardcoded como finais
- [ ] `validate()` rejeita `mem`/`iters`/`par` abaixo do mínimo e acima do máximo **sem** alocar a memória pedida (edge case anti-DoS)
- [ ] `+N` testes: determinismo com vetor fixo; salts diferentes → KEKs diferentes; params fora do limite → erro antes de alocar
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T3: `crypto::keys` — root_key, HKDF de subchaves e `K_sessao` [P]

**What:** `generate_root_key(rand: &[u8;32]) -> Key32` (bytes injetáveis); `derive_content_key(root: &Key32, epoch: u32) -> Key32` via `HKDF-SHA256(info = "ssv:content:v1:" ‖ epoch)`; `derive_session_wrap_key(gmk: &Key32, uuid: &[u8;16]) -> Key32` via `HKDF-SHA256(info = "ssv:session-wrap:v1:" ‖ uuid)`.
**Where:** `src-tauri/src/crypto/keys.rs` (+ testes `#[cfg(test)]`)
**Depends on:** T1
**Reuses:** `Key32` (T1), crates `hkdf`,`sha2`
**Requirement:** KEY-01, KEY-02, GKEY-02 (derivação de `K_sessao`)
**Tools:** Write · MCP: `context7` (API do `hkdf`) · Skill: NONE
**Done when:**
- [ ] Subchaves de propósitos/épocas diferentes são **distintas** (rótulos separados — nunca reutilizar chave entre contextos)
- [ ] `derive_content_key` muda com a época; `derive_session_wrap_key` muda com o uuid
- [ ] Rótulos `info` exatamente como no design (`ssv:content:v1:` / `ssv:session-wrap:v1:`)
- [ ] `+N` testes: determinismo vs `info` conhecido; épocas/uuids distintos → subchaves distintas; propósitos distintos → subchaves distintas
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T4: `crypto::aead` — XChaCha20-Poly1305 seal/open com AAD [P]

**What:** `seal(key: &Key32, nonce: &[u8;24], plaintext: &[u8], aad: &[u8]) -> Vec<u8>` e `open(key, nonce, ciphertext, aad) -> Result<Vec<u8>>` (falha de autenticação → `CryptoError`). `wrap`/`unwrap` de chave são `seal`/`open` sobre `Key32`.
**Where:** `src-tauri/src/crypto/aead.rs` (+ testes `#[cfg(test)]`)
**Depends on:** T1
**Reuses:** `Key32`/`CryptoError` (T1), crate `chacha20poly1305` (XChaCha20-Poly1305)
**Requirement:** AEAD-01, AEAD-02
**Tools:** Write · MCP: `context7` (API do `chacha20poly1305`) · Skill: NONE
**Done when:**
- [ ] Roundtrip `open(seal(...))` recupera o plaintext com AAD correta
- [ ] Alterar 1 byte de ciphertext, da tag, do nonce ou da AAD → `open` falha (não retorna plaintext)
- [ ] Chave errada → falha de autenticação
- [ ] Nonce de 192 bits (24 bytes) recebido por parâmetro (injetável); documentar `⚠️ PT-02` (nonce aleatório vs contador — decisão final presa a PT-02)
- [ ] `+N` testes: roundtrip; adulteração de ct/tag/nonce/aad; chave errada
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T5: `crypto::keyring` — keyring global (GMP→gKEK→GMK) [P]

**What:** `KeyringHeader { magic:"SSGK", format_version, kdf, salt_global, aead_id }` + `KeyringEnvelope { header, gmk_wrap:{nonce,ciphertext} }` com serde/CBOR canônico; `create_keyring(gmp, salt_global, params, gmk_rand, nonce) -> KeyringEnvelope` (gKEK ← Argon2id(GMP,salt_global); `wrapped_gmk = AEAD(gKEK, GMK, aad = header)`); `unwrap_gmk(gmp, &KeyringEnvelope) -> Result<Key32>`; `change_gmp(old_gmp, new_gmp, new_salt, params, nonce, &KeyringEnvelope) -> KeyringEnvelope` (reenvolve a **mesma** GMK).
**Where:** `src-tauri/src/crypto/keyring.rs` (+ testes `#[cfg(test)]`)
**Depends on:** T2, T4
**Reuses:** `kdf::derive_kek` (T2), `aead::{seal,open}` (T4), `Key32`/`CryptoError` (T1); crates `ciborium`,`serde`
**Requirement:** GKEY-01
**Tools:** Write · MCP: `context7` (API `ciborium`) · Skill: NONE
**Done when:**
- [ ] `unwrap_gmk` recupera a GMK criada por `create_keyring`; GMP errada → falha de autenticação (sem verificador barato)
- [ ] `change_gmp` mantém a **mesma** GMK (unwrap com a nova GMP == GMK original); GMP antiga errada → erro
- [ ] AAD = bytes canônicos do `KeyringHeader`; alterar `format_version`/params/`salt_global` no header quebra a autenticação
- [ ] Magic diferente de `SSGK` → rejeição
- [ ] `+N` testes: create→unwrap; GMP errada; change_gmp preserva GMK; adulteração de cada campo do header; magic inválido
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T6: `crypto::envelope` — cofre de sessão, `auth_mode` na AAD, migração [P]

**What:** `Header { magic:"SSV1", format_version, session_uuid, auth_mode, kdf, salt, aead_id, epoch, session_name }` + `VaultEnvelope { header, key_wrap, payload }` com serde/CBOR canônico; `create_vault(...)` e `unlock(...)` com **wrap condicional da root_key** por `auth_mode` (own: `KEK=Argon2id(senha,salt)`; global: `K_sessao=HKDF(GMK,uuid)`); `rewrap(...)` (rotação/troca de senha — mesma root_key); migração `format_version` < corrente e **fail-closed** em versão superior.
**Where:** `src-tauri/src/crypto/envelope.rs` (+ testes `#[cfg(test)]`)
**Depends on:** T2, T3, T4
**Reuses:** `kdf` (T2, own), `keys::derive_session_wrap_key`+`derive_content_key` (T3, global/conteúdo), `aead` (T4), `Key32`/`CryptoError` (T1); crates `ciborium`,`serde`,`uuid`
**Requirement:** KEY-01, GKEY-02, AEAD-01, FMT-01, FMT-02, FMT-03, ROT-01
**Tools:** Write · MCP: `context7` (API `ciborium`/`uuid`) · Skill: NONE
**Done when:**
- [ ] Roundtrip em `auth_mode = own` (senha) e `auth_mode = global` (GMK) recupera root_key e conteúdo
- [ ] Chave de envelopamento errada (senha errada em own / GMK errada em global) → falha de autenticação, não chave silenciosa
- [ ] Adulterar **cada** campo do header (versão, uuid, `auth_mode`, params, salt, epoch, nome) → rejeição; rebaixar `own`↔`global` sem a chave de destino → rejeição
- [ ] `format_version` > suportada → fail-closed (não interpreta, não sobrescreve); versão anterior suportada migra sem perda
- [ ] Campos desconhecidos dentro de versão suportada preservados (CBOR) — não descartados
- [ ] `rewrap` mantém o conteúdo antigo decifrável (mesma root_key); prova da senha atual antes de gravar
- [ ] Payload mínimo da fatia: `{ content_format, secrets: [] }` — exercita cifra/decifra sem fixar o modelo de segredos
- [ ] `+N` testes cobrindo todos os itens acima
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

---

### T7: `crypto::vectors` — vetores determinísticos + adulteração (TEST-01)

**What:** módulo de teste com **vetores reproduzíveis**: entradas fixas (GMP/senha, `salt_global`/salt, nonces, params Argon2id reduzidos) → saídas esperadas (hex) para `gKEK`, `wrapped_gmk`, `K_sessao`, `KEK`, `content_key`, `wrapped_root_key`, `payload`; mais vetores de **adulteração** (mutação de cada campo de header incl. `auth_mode`/`salt_global` + 1 byte de ct/tag → rejeição documentada).
**Where:** `src-tauri/src/crypto/vectors.rs` (`#[cfg(test)]`, registrado em `crypto/mod.rs`)
**Depends on:** T5, T6
**Reuses:** todos os módulos `crypto::*` (T2–T6)
**Requirement:** TEST-01
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Vetores com params Argon2id **reduzidos** (rápidos em teste) e nonces/salts fixos → saídas hex esperadas conferem exatamente
- [ ] Cada vetor de adulteração resulta em rejeição autenticada (documentada no teste)
- [ ] Vetores nomeados/comentados para servir de referência a uma releitura independente
- [ ] `+N` testes de vetor passam; suíte Rust inteira verde
- [ ] Gate check passes: `pnpm check:rust`
**Tests:** unit
**Gate:** full

**Commit:** `feat(crypto): formato versionado do cofre — KDF, keyring global, envelope de sessão e vetores`

---

### T8: Plano de revisão independente do formato (TEST-02)

**What:** documento de escopo/entregáveis/critérios da revisão independente do formato criptográfico (não é código; fecha a fatia).
**Where:** `.specs/features/crypto-format/review-plan.md` (new)
**Depends on:** T7
**Reuses:** spec §TEST-02, design §Vetores
**Requirement:** TEST-02
**Tools:** Write · Skill: NONE
**Done when:**
- [ ] Define escopo (KDF, wrap, AEAD, keyring, envelope, migração), entregáveis e critérios de aceitação da revisão
- [ ] Referencia os vetores de T7 como material reproduzível
- [ ] Registra os pontos presos a PT-01/PT-02 e ao gate D-05 (re-aprovação do modelo de ameaças)
**Tests:** none (documento)
**Gate:** none

---

## Pre-Approval Validation

### Check 1 — Task Granularity

| Task | Escopo | Status |
| --- | --- | --- |
| T1 | deps + 3 arquivos base coesos (mod/error/secret) | ✅ |
| T2 | 1 módulo (kdf) | ✅ |
| T3 | 1 módulo (keys) | ✅ |
| T4 | 1 módulo (aead) | ✅ |
| T5 | 1 módulo (keyring) | ✅ |
| T6 | 1 módulo (envelope) | ✅ |
| T7 | 1 módulo de vetores | ✅ |
| T8 | 1 documento | ✅ |

### Check 2 — Diagram ↔ Definition Cross-Check

| Task | Depends on (corpo) | Diagrama mostra | Status |
| --- | --- | --- | --- |
| T2 | T1 | T1 → T2 | ✅ |
| T3 | T1 | T1 → T3 | ✅ |
| T4 | T1 | T1 → T4 | ✅ |
| T5 | T2, T4 | T2,T4 → T5 | ✅ |
| T6 | T2, T3, T4 | T2,T3,T4 → T6 | ✅ |
| T7 | T5, T6 | T5,T6 → T7 | ✅ |
| T8 | T7 | T7 → T8 | ✅ |

Nenhum par `[P]` depende de outro na mesma fase (T2/T3/T4 entre si: não; T5/T6 entre si: não). ✅

### Check 3 — Test Co-location Validation

| Task | Camada criada | Matriz exige | Task diz | Status |
| --- | --- | --- | --- | --- |
| T1 | Core Rust puro (+config) | unit | unit | ✅ |
| T2–T6 | Core Rust puro | unit | unit | ✅ |
| T7 | Core Rust puro (vetores) | unit | unit | ✅ |
| T8 | Documento (sem linha na matriz) | none | none | ✅ |

Rust unitário é **parallel-safe** (TESTING.md) → `[P]` permitido em T2/T3/T4 e T5/T6. Sem violações; cada módulo traz seus testes na própria tarefa (sem deferimento).

---

## Parallel Execution Map

```
Phase 1:  T1
Phase 2:  T2  T3  T4              (todos [P], após T1)
Phase 3:  T5  T6                  ([P] entre si, após T2/T3/T4)
Phase 4:  T7                      (após T5, T6)
Phase 5:  T8                      (doc — após T7)
```

**Execução:** delegar cada `[P]` a um sub-agente por tarefa; o orquestrador aguarda a fase fechar antes de avançar. Fechamento de fase roda `pnpm check` + `pnpm build --no-bundle`.

---

## Out of Scope desta fatia (rastreado)

- **INTEG-01** (campos de revisão/ancestralidade/sequência) → feature de sincronização.
- **Parâmetros finais** de Argon2id (PT-01) e decisão final de AEAD/nonce (PT-02) → protótipos.
- **Comandos Tauri / orquestração de app-unlock** (derivar gKEK, abrir todas as sessões `global`) → feature `local-sessions` (Execute), reusa estes módulos.
- **Zeroização plena / memória protegida** (PT-04/PT-06) → feature "Prova de integração Windows e Tauri".
