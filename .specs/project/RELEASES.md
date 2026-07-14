# Política de commits, versões e releases

**Status:** Aprovada como padrão inicial

**Data:** 2026-07-13

**Aplica-se a:** Secrets Storage para Windows, distribuído pelo GitHub e atualizado pelo Tauri Updater

---

## 1. Princípios

1. `main` deve conter apenas mudanças revisadas e permanecer em estado publicável.
2. O histórico público será linear, legível e rastreável a pull requests.
3. Uma versão publicada é imutável. Qualquer correção gera uma nova versão.
4. Conventional Commits informa a mudança de versão, mas nunca publica sozinho: a decisão é confirmada numa Release PR.
5. O binário é sempre reconstruído no GitHub Actions a partir da tag; não são enviados binários produzidos numa máquina de desenvolvimento.
6. Assinatura do Tauri Updater, Authenticode e attestation têm funções diferentes e não se substituem.
7. A versão do aplicativo e a versão do formato criptográfico são independentes.

## 2. Modelo Git

### Branch principal

- Uma única branch permanente: `main`.
- Branches de trabalho são curtas e partem da `main` atualizada.
- Não haverá branches permanentes `develop`, `release/*` ou específicas de versão no v1.
- Correções urgentes usam `hotfix/<descricao>`, mas entram pela mesma PR e pelos mesmos gates.

### Merge

- Estratégia exclusiva: **Squash and merge**.
- O título convencional da PR forma o commit final na `main`.
- A branch é apagada após o merge.
- Push direto, force-push e exclusão da `main` são bloqueados.

### Ruleset da `main`

O repositório GitHub deve configurar:

- pull request obrigatória;
- histórico linear;
- checks obrigatórios e branch atualizada antes do merge;
- resolução de todas as conversas;
- bloqueio de force-push e exclusão;
- commits verificados, após validar o fluxo de squash com a conta e os bots usados;
- zero aprovações enquanto houver um único mantenedor; uma aprovação obrigatória assim que houver outro mantenedor ativo.

Os checks mínimos serão `format`, `lint`, `test-rust`, `test-frontend`, `security-audit` e `build-windows`. Os nomes podem mudar durante o scaffold, mas a cobertura não pode ser reduzida silenciosamente.

## 3. Conventional Commits

Formato:

```text
<type>(<scope>)!: <descricao>

<corpo opcional>

<trailers opcionais>
```

A descrição é escrita em português; `type` e `scope` permanecem em inglês para compatibilidade com ferramentas. As regras detalhadas e exemplos estão em [CONTRIBUTING.md](../../CONTRIBUTING.md).

Efeito padrão no versionamento:

| Commit | Versão estável (`>=1.0.0`) | Série inicial (`0.x`) |
| --- | --- | --- |
| `fix` | PATCH | PATCH |
| `perf` com efeito observável | PATCH | PATCH |
| `feat` | MINOR | MINOR |
| `!` ou `BREAKING CHANGE:` | MAJOR | MINOR |
| `docs`, `test`, `refactor`, `build`, `ci`, `chore` | Sem release isoladamente | Sem release isoladamente |

Se uma mudança de outro tipo altera comportamento público, o tipo está errado e deve ser corrigido para `fix` ou `feat`. Correção de segurança compatível usa `fix(security)`; uma correção incompatível segue a regra de breaking change.

## 4. Semantic Versioning

O aplicativo usa [Semantic Versioning 2.0.0](https://semver.org/):

```text
MAJOR.MINOR.PATCH[-pre-release]
```

### Antes da estabilidade

- Desenvolvimento inicial: `0.MINOR.PATCH`.
- Primeira distribuição testável planejada: `0.1.0-alpha.1`.
- Mudança incompatível durante `0.x`: incrementa `MINOR` e zera `PATCH`.
- O projeto só chega a `1.0.0` após concluir M0–M3, estabilizar formato/migrações/updater e realizar revisão independente de segurança.

### Depois de `1.0.0`

- `MAJOR`: mudança incompatível na experiência, configuração, formato do cofre, protocolo de sync ou política de update sem migração segura.
- `MINOR`: funcionalidade nova compatível ou depreciação anunciada.
- `PATCH`: correção compatível, inclusive de segurança e desempenho.

### Pré-releases

| Sufixo | Uso |
| --- | --- |
| `alpha.N` | Integração inicial; fluxos e formato ainda podem mudar. |
| `beta.N` | Funcionalidades do marco completas; foco em correções e compatibilidade. |
| `rc.N` | Candidato ao release; somente correções bloqueadoras. |
| Sem sufixo | Release estável. |

Exemplos de tags: `v0.1.0-alpha.1`, `v0.1.0-beta.1`, `v0.1.0-rc.1` e `v1.0.0`.

Metadados `+build` não serão usados em releases públicas, pois não alteram precedência e complicam nomes de artefato/updater.

## 5. Fontes de versão

- Fonte canônica: `src-tauri/tauri.conf.json > version`.
- `package.json` e `src-tauri/Cargo.toml`, quando possuírem a versão do aplicativo, devem espelhar o mesmo valor.
- A tag tem prefixo `v`; os arquivos usam apenas `X.Y.Z[-pre]`.
- O workflow falha se tag, configuração Tauri, manifests e `latest.json` divergirem.
- `vault_format_version`, schema de sync e épocas criptográficas têm versionamento próprio e não herdam automaticamente o SemVer do aplicativo.
- Uma versão ou tag publicada nunca é reutilizada, movida ou sobrescrita.

## 6. Changelog e notas

O projeto manterá `CHANGELOG.md` no formato Keep a Changelog, com `Unreleased` e as categorias aplicáveis: Added, Changed, Deprecated, Removed, Fixed e Security. O texto destinado ao usuário será em português.

Na Release PR:

1. mudanças relevantes desde a última tag são agrupadas e reescritas para usuários;
2. breaking changes e migrações aparecem no topo;
3. correções de segurança descrevem impacto sem revelar exploração antes da publicação coordenada;
4. GitHub generated release notes pode complementar autores e PRs, mas não substitui a revisão humana;
5. commits puramente internos não precisam aparecer, salvo se afetarem risco, build ou auditoria.

## 7. Fluxo de geração de versão

### Etapa A — Release PR

Uma branch `chore/release-vX.Y.Z` parte da `main` e cria uma PR com:

- versão proposta e justificativa do bump;
- atualização coerente dos manifests;
- `CHANGELOG.md` e notas do GitHub;
- migrations e compatibilidade documentadas;
- resultado dos gates completos;
- checklist especial para criptografia, sync, updater e dados persistidos.

O commit final será:

```text
chore(release): prepara vX.Y.Z
```

### Etapa B — Tag

Após o merge:

1. confirmar que o commit pertence à `main` e que a árvore está limpa;
2. criar tag **anotada e assinada** `vX.Y.Z` exatamente no commit da Release PR;
3. enviar a tag ao GitHub;
4. a ruleset `v*` restringe criação a mantenedores/automação de release e bloqueia atualização ou exclusão.

### Etapa C — GitHub Actions

O push da tag executa exclusivamente o workflow de release:

1. valida tag, versão, origem na `main` e inexistência da versão;
2. faz checkout pelo SHA da tag e instala dependências por lockfile;
3. executa format, lint, testes, auditorias e build limpo em runner Windows hospedado pelo GitHub;
4. produz o instalador NSIS como formato primário do v1;
5. assina o instalador com Authenticode quando o certificado estiver disponível;
6. gera artefato e `.sig` obrigatórios do Tauri Updater;
7. gera `latest.json`, `SHA256SUMS`, SBOM e attestations de proveniência quando suportadas;
8. cria um GitHub Release em **draft** e anexa todos os artefatos;
9. executa smoke test de instalação e verificação do update;
10. publica apenas após a verificação final do draft.

NSIS será a origem do `latest.json`. MSI não será publicado inicialmente para evitar dois caminhos de instalação/update sem necessidade comprovada.

### Etapa D — Publicação imutável

- Ativar immutable releases no GitHub quando disponível.
- O draft recebe todos os arquivos antes da publicação.
- `alpha`, `beta` e `rc` são marcadas como prerelease; release sem sufixo é estável.
- Falha depois da publicação gera uma nova versão PATCH ou prerelease incrementada; assets nunca são trocados.
- O endpoint estável do updater será `releases/latest/download/latest.json` e não deverá apontar para prereleases.

## 8. Canais

| Canal | Distribuição | Atualização automática |
| --- | --- | --- |
| Stable | GitHub Release normal | Sim, somente para outra versão estável superior. |
| Prerelease | GitHub Release marcada como prerelease | Não entra no canal estável; instalação consciente pelo testador. |
| Nightly | Artefato temporário de GitHub Actions | Sem GitHub Release e sem atualização automática no v1. |

Um canal beta automático separado poderá ser adicionado futuramente, com endpoint e preferência explícita do usuário.

## 9. Segurança do GitHub Actions

- Permissão padrão do `GITHUB_TOKEN`: somente leitura.
- Jobs de CI: `contents: read`.
- Job que cria/anexa release: `contents: write` somente nele.
- Attestation: `id-token: write` e `attestations: write` somente no job correspondente.
- Todas as actions são fixadas por SHA completo e passam por revisão antes de atualização.
- Builds de PR/fork nunca recebem segredos e não usam `pull_request_target` para executar código não confiável.
- Release usa environment `release`, limitado a tags `v*`, para segredos e aprovações disponíveis no plano do GitHub.
- Segredos `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` existem apenas no environment de release; a chave pública fica no aplicativo.
- A chave privada do updater possui backup offline e plano de rotação antes de `1.0.0`; perdê-la impede atualizar instalações existentes.
- Preferir Authenticode via serviço/HSM e OIDC. Arquivo PFX duradouro no CI é último recurso.
- Runners hospedados pelo GitHub são o padrão de release; self-hosted exige modelo de ameaça e hardening próprios.

Attestation prova a origem do build, não que o binário seja seguro. Authenticode identifica o editor no Windows; a assinatura Tauri autentica o pacote para o updater. As três proteções são mantidas separadamente.

## 10. Artefatos de cada release

| Artefato | Obrigatório |
| --- | --- |
| Instalador NSIS para Windows | Sim |
| Assinatura Authenticode | Para distribuição pública; estratégia ainda precisa ser contratada/configurada |
| Arquivo `.sig` do Tauri Updater | Sim e não desativável |
| `latest.json` | Sim para release estável |
| `SHA256SUMS` | Sim |
| Release notes/changelog | Sim |
| SBOM SPDX ou CycloneDX | Sim antes da versão estável |
| GitHub artifact/release attestation | Sim quando disponível para o plano/visibilidade do repositório |

## 11. Hotfix e vulnerabilidade

1. Correção é desenvolvida em branch privada quando a divulgação antecipada aumentar o risco, usando GitHub Security Advisory.
2. A mudança segue `fix(security)` e os mesmos gates; não existe bypass de assinatura ou teste.
3. A Release PR usa PATCH se compatível ou o incremento requerido para incompatibilidade.
4. Advisory e release são publicados de forma coordenada.
5. Uma chave de release comprometida aciona revogação, rotação e versão de emergência; release imutável não corrige comprometimento do signatário.

## 12. Configuração pendente no GitHub

Os workflows de CI e release e a configuração do NSIS/updater já existem no repositório. Ainda precisam ser aplicados no GitHub:

- rulesets de `main` e tags `v*`;
- merge por squash e exclusão automática de branches;
- checks obrigatórios;
- environment `release`;
- immutable releases;
- permissões mínimas do Actions e allowlist/SHA pinning;
- secrets de assinatura e backup offline;
- configuração das release notes e validação da primeira execução do workflow em draft.

## 13. Referências oficiais

- [Conventional Commits 1.0](https://www.conventionalcommits.org/pt-br/v1.0.0/)
- [Semantic Versioning 2.0.0](https://semver.org/)
- [GitHub — About releases](https://docs.github.com/en/repositories/releasing-projects-on-github/about-releases)
- [GitHub — Rulesets](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets)
- [GitHub — Immutable releases](https://docs.github.com/en/code-security/concepts/supply-chain-security/immutable-releases)
- [GitHub — Artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations)
- [GitHub — Secure use of Actions](https://docs.github.com/en/actions/reference/security/secure-use)
- [Tauri 2 — Updater and signing](https://v2.tauri.app/plugin/updater/)
- [Tauri 2 — Windows code signing](https://v2.tauri.app/distribute/sign/windows/)
