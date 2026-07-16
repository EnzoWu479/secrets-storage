# Plano de Revisão Independente — Formato Criptográfico (TEST-02)

**Spec:** [spec.md](./spec.md) · **Design:** [design.md](./design.md) · **Tasks:** [tasks.md](./tasks.md)
**Status:** Aberto — aguardando revisão independente antes de o formato virar base estável.

> Este documento **fecha a fatia de implementação** (`crypto::{kdf,keys,aead,keyring,envelope}` + vetores) definindo escopo, entregáveis e critérios de uma revisão criptográfica independente. Não é código. A revisão é **pré-requisito** para reabrir/fechar o gate **D-05** (modelo de ameaças, reaberto por AD-022) e para promover os parâmetros candidatos (PT-01/PT-02) a finais.

---

## 1. Contexto e por que revisar

O núcleo criptográfico implementa o **design candidato** da fatia "Sessões + desbloqueio": KDF (Argon2id), AEAD (XChaCha20-Poly1305), HKDF-SHA256, keyring global (GMP→gKEK→GMK) e envelope de sessão versionado com `auth_mode` na AAD, rotação/`rewrap` e migração fail-closed.

Nada aqui está certificado. Três travas explícitas permanecem:

- **PT-01** — parâmetros finais do Argon2id (`mem/iters/parallelism`).
- **PT-02** — decisão final de AEAD/nonce (XChaCha20-Poly1305 com nonce aleatório de 192 bits vs. alternativa/contador; questão FIPS).
- **D-05** — re-aprovação humana do modelo de ameaças (§11 da spec).

A revisão independente é o mecanismo para converter "candidato" em "base estável".

---

## 2. Escopo da revisão

Em escopo (o que a revisão **deve** cobrir):

| Área | Itens |
| --- | --- |
| **KDF** | Argon2id: uso correto da API, injeção de salt, limites de `KdfParams::validate` (mínimo seguro / teto anti-DoS), rejeição **antes** de alocar. |
| **Wrap de chave** | `aead::{wrap_key,unwrap_key}`: AAD = header, zeroização do material intermediário, ausência de verificador barato. |
| **AEAD** | `seal`/`open` XChaCha20-Poly1305: autenticação antes de interpretar, tratamento uniforme de falha (sem oráculo), manuseio do nonce. |
| **Derivação** | HKDF-SHA256: separação de domínio por rótulo/propósito/época/uuid; não reutilização de chave entre contextos. |
| **Keyring global** | GMP→gKEK→GMK; GMK independente da GMP; `change_gmp` reenvolve a **mesma** GMK; magic/versão. |
| **Envelope de sessão** | `auth_mode` na AAD (anti-rebaixamento own↔global); wrap condicional; `rewrap` (re-selagem do payload sob o novo header); migração e **fail-closed** em versão superior; preservação de campos desconhecidos (FMT-03). |
| **Fronteira** | Nenhum material de chave/senha cruza o IPC; segredos zeroizáveis (`Key32`). |
| **Serialização** | Layout CBOR (`ciborium`): estabilidade da AAD (header como bytes opacos), canonicidade, ambiguidades de parsing. |

Fora de escopo desta revisão (rastreado, não esquecido):

- **INTEG-01** (campos de sincronização/ancestralidade) → feature de sync.
- **Comandos Tauri / orquestração de app-unlock** (derivar gKEK, abrir sessões `global` em conjunto) → feature `local-sessions`.
- **Zeroização plena / memória protegida** (PT-04/PT-06) → feature "Prova de integração Windows e Tauri".

---

## 3. Entregáveis da revisão

1. **Parecer** sobre cada área da §2: adequado / ajustar / bloquear, com justificativa.
2. **Decisão sobre PT-01** — parâmetros Argon2id finais (ou faixa aceitável + método de calibração por dispositivo).
3. **Decisão sobre PT-02** — AEAD/nonce finais; se contador, especificar o esquema anti-reuso.
4. **Veredito sobre o layout CBOR** (`⚠️ §12 #4`): manter self-describing ou migrar para layout fixo.
5. **Recomendação sobre D-05** — se o modelo de ameaças pode ser re-aprovado com o formato atual.
6. **Lista priorizada** de mudanças obrigatórias antes de promover a base estável.

---

## 4. Material reproduzível (vetores)

A revisão deve reproduzir de forma independente os **vetores golden** de `src-tauri/src/crypto/vectors.rs` (TEST-01). Com as entradas fixas ali documentadas (params reduzidos, salts/nonces/rands constantes), qualquer implementação do design candidato deve produzir exatamente:

- `gKEK`, `KEK_propria` (Argon2id), `K_sessao`, `content_key` (HKDF) — chaves de 32 bytes em hex;
- `ciphertext` AEAD de referência (com AAD fixa);
- os envelopes **serializados completos** (`keyring`, `vault` em `own` e `global`) em CBOR/hex;
- recuperação: `unwrap_gmk` devolve a GMK; `unlock` devolve `root_key` e conteúdo esperados.

E os **vetores de adulteração**: inverter 1 bit do ciphertext/tag ⇒ rejeição por falha de autenticação; e (nos testes dos módulos `keyring`/`envelope`) mutação de **cada** campo do header (incl. `auth_mode`, `salt_global`, versão, uuid, época, nome) e rebaixamento `own`↔`global` ⇒ rejeição.

> Divergência em qualquer vetor golden indica diferença de parâmetro, de layout ou de implementação — e é sinal de investigação, não de "atualizar o vetor" sem entender a causa.

---

## 5. Critérios de aceitação

A fatia é considerada **revisada e apta a promoção** quando:

- [ ] Todas as áreas da §2 receberam parecer sem bloqueios pendentes.
- [ ] PT-01 e PT-02 têm decisão registrada (parâmetros/mecanismos finais).
- [ ] O veredito de layout CBOR está registrado (manter ou trocar).
- [ ] Os vetores de TEST-01 foram reproduzidos por caminho independente e conferem.
- [ ] Os vetores de adulteração confirmam rejeição autenticada em todos os casos listados.
- [ ] Há recomendação explícita sobre D-05 (re-aprovação do modelo de ameaças).
- [ ] A lista priorizada de mudanças obrigatórias foi endereçada (ou aceita como follow-up rastreado).

Enquanto qualquer item acima estiver aberto, o formato permanece **candidato** e não deve ser tratado como garantia de segurança.
