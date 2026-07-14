# Contribuindo

Este projeto usa histórico linear, Conventional Commits e desenvolvimento orientado por pull requests. A política completa de versionamento e publicação está em [RELEASES.md](./.specs/project/RELEASES.md).

## Branches

A branch `main` é protegida e deve permanecer publicável. Toda alteração entra por uma branch curta e uma pull request.

Formato recomendado:

```text
<tipo>/<issue-opcional>-<descricao-curta>
```

Exemplos:

```text
feat/42-sessoes-nomeadas
fix/clipboard-race
docs/modelo-ameacas
chore/atualiza-tauri
```

Tipos de branch: `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore` e `hotfix`.

## Commits

Todo commit segue [Conventional Commits 1.0](https://www.conventionalcommits.org/pt-br/v1.0.0/):

```text
<tipo>(<escopo>): <descricao>
```

- Tipo e escopo em inglês; descrição em português.
- Descrição no imperativo, em minúsculas, sem ponto final e preferencialmente com até 72 caracteres.
- Um commit representa uma mudança lógica e deixa testes/documentação coerentes.
- Use `!` e o trailer `BREAKING CHANGE:` quando houver incompatibilidade.
- Referencie issues com trailers como `Refs: #123` ou `Closes: #123`.

Tipos aceitos:

| Tipo | Uso |
| --- | --- |
| `feat` | Nova capacidade observável. |
| `fix` | Correção de comportamento, inclusive `fix(security)`. |
| `perf` | Melhoria mensurável de desempenho. |
| `refactor` | Mudança interna sem alterar comportamento. |
| `docs` | Documentação apenas. |
| `test` | Testes apenas. |
| `build` | Build, empacotamento ou dependências. |
| `ci` | GitHub Actions e automação. |
| `chore` | Manutenção que não cabe nos tipos anteriores. |
| `revert` | Reversão explícita de commit anterior. |

Escopos preferidos: `app`, `ui`, `core`, `vault`, `crypto`, `storage`, `sessions`, `clipboard`, `sync`, `oauth`, `updater`, `release`, `deps`, `ci` e `docs`.

Exemplos:

```text
feat(sessions): adiciona bloqueio independente por inatividade
fix(sync): preserva as duas versões durante conflito
fix(security): remove token oauth dos logs de erro
feat(storage)!: altera o envelope criptografico do cofre
```

## Pull requests

- O título da PR também segue Conventional Commits; ele se torna o commit final do squash.
- A descrição explica problema, solução, risco, testes e impacto de segurança/migração.
- Prefira PRs pequenas e revisáveis; mudanças independentes pertencem a PRs diferentes.
- O merge padrão é **Squash and merge**. Merge commits e rebase merge ficam desabilitados no GitHub.
- Checks obrigatórios devem passar antes do merge.
- Mudanças em criptografia, formato do cofre, updater, OAuth, permissões Tauri ou workflows de release exigem revisão de segurança explícita antes da versão estável.

Enquanto existir somente um mantenedor, a ruleset exige PR e checks, mas zero aprovações externas. Quando houver um segundo mantenedor ativo, passa a exigir ao menos uma aprovação e proteção por CODEOWNERS nas áreas críticas.
