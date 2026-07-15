---
name: release-version
description: >-
  Prepara uma nova versão do Secrets Storage: decide o bump SemVer a partir dos
  Conventional Commits desde a última tag, atualiza a versão nos quatro arquivos
  canônicos (tauri.conf.json, package.json, Cargo.toml, Cargo.lock), redige o
  CHANGELOG.md em pt-BR no formato Keep a Changelog e monta a branch de Release
  PR. Use SEMPRE que o pedido envolver "gerar versão", "cortar release", "bump de
  versão", "preparar release", "atualizar o changelog", "nova versão", "release
  PR" ou publicar uma versão nova — mesmo que o usuário não cite todos os passos.
  Não faz push da tag nem publica: para no ponto da Release PR, conforme a
  política do projeto.
---

# Gerar versão / Release PR

Esta skill executa a **Etapa A** da política de releases (`.specs/project/RELEASES.md` §7):
preparar a Release PR. As etapas de tag e publicação são deliberadamente humanas
e ficam de fora — o objetivo aqui é entregar uma branch pronta para revisão.

## Contexto que você precisa respeitar

- **Fonte canônica da versão:** `src-tauri/tauri.conf.json > version`. Os outros
  três arquivos apenas espelham esse valor. O workflow de release falha se
  qualquer um divergir da tag, então os quatro precisam bater exatamente.
- **Série 0.x:** o projeto ainda está em `0.MINOR.PATCH`. Nessa fase, um
  breaking change incrementa MINOR (não MAJOR). Veja a tabela abaixo.
- **Idioma:** o texto do changelog voltado ao usuário é em **português**;
  `type`/`scope` dos commits permanecem em inglês.
- **Nunca** reutilize, mova ou sobrescreva uma versão/tag já publicada.

## Efeito dos Conventional Commits na versão (série 0.x)

| Commit desde a última tag | Efeito |
| --- | --- |
| `fix`, `perf` com efeito observável | PATCH |
| `feat` | MINOR |
| `!` ou `BREAKING CHANGE:` | MINOR (zera PATCH) — porque estamos em 0.x |
| `docs`, `test`, `refactor`, `build`, `ci`, `chore` | Sozinhos não geram release |

Se só houver commits do último grupo desde a última tag, avise o usuário que não
há mudança que justifique uma release e confirme se ele quer prosseguir mesmo assim.

## Fluxo

### 1. Levantar o estado atual

```bash
node .claude/skills/release-version/scripts/bump_version.mjs --current   # versão atual
git tag --list | sort -V | tail -1                                       # última tag
git log <ultima-tag>..HEAD --oneline                                     # commits desde a tag
```

Confirme que a árvore de trabalho está limpa (`git status`). Se houver mudanças
soltas, pare e pergunte — a Release PR precisa partir de um estado conhecido.

### 2. Propor a versão

Classifique cada commit desde a última tag pela tabela acima, determine o maior
efeito e proponha a nova versão. **Apresente a justificativa ao usuário e espere
a confirmação** antes de editar qualquer arquivo — a decisão de bump nunca é
automática (política §1.4). Se o usuário já disse a versão explicitamente, use a dele.

### 3. Redigir o CHANGELOG antes de finalizar

Edite a seção `## [Unreleased]` de `CHANGELOG.md` **antes** de rodar o script com
`--changelog`. O script transforma exatamente o que estiver em Unreleased na seção
datada, então a curadoria acontece aqui:

- Agrupe as mudanças nas categorias aplicáveis do Keep a Changelog, nesta ordem:
  **Added, Changed, Deprecated, Removed, Fixed, Security**. Omita as vazias.
- Reescreva para o usuário final, em português — descreva o efeito percebido, não
  o commit interno. Commits puramente internos (`chore`, `refactor`, `ci`) só
  entram se afetarem risco, build ou auditoria.
- **Breaking changes e migrações vão no topo da seção**, antes das categorias.
- Correções de segurança descrevem o impacto **sem revelar detalhes de exploração**
  antes da publicação coordenada.

Deixe o conteúdo dentro de `## [Unreleased]`; não crie a seção datada à mão — o
script faz isso de forma consistente.

### 4. Aplicar o bump

```bash
node .claude/skills/release-version/scripts/bump_version.mjs <X.Y.Z> --changelog
```

Isso atualiza os quatro arquivos de versão e converte `## [Unreleased]` em
`## [X.Y.Z] - AAAA-MM-DD`, recriando um Unreleased vazio no topo. O script **falha
de propósito** se algum arquivo não for encontrado, se um padrão de versão for
ambíguo, ou se a seção Unreleased estiver vazia — trate qualquer erro como um
sinal para parar e investigar, não para editar à mão.

Revise o diff (`git diff`) e confira que os quatro arquivos mostram a mesma versão.

### 5. Montar a Release PR

Crie a branch a partir da `main` atualizada e faça o commit único:

```bash
git checkout -b chore/release-v<X.Y.Z>
git add package.json src-tauri/tauri.conf.json src-tauri/Cargo.toml src-tauri/Cargo.lock CHANGELOG.md
git commit -m "chore(release): prepara v<X.Y.Z>"
```

O título convencional é `chore(release): prepara vX.Y.Z` (política §7). **Não faça
push nem abra a PR sem o usuário pedir** — publicar uma branch é uma ação externa.

### 6. Entregar e explicar o que falta

Ao terminar, resuma para o usuário e deixe claro o que é responsabilidade humana:

1. Revisar o diff da Release PR e mergear por **squash** na `main`.
2. Após o merge, criar a tag **anotada e assinada** `v<X.Y.Z>` no commit da Release
   PR e dar push — é isso que dispara o workflow de release e a build assinada.
3. O release sai como **draft**; publicar manualmente após a verificação final
   (o updater só enxerga releases publicados, não drafts).

## Erros comuns a evitar

- Editar a versão em um arquivo só — os quatro precisam bater ou o workflow falha.
- Rodar o script com a seção Unreleased vazia — cure o changelog primeiro.
- Fazer push da tag antes do merge da Release PR — a tag deve apontar para o commit
  já revisado na `main`.
- Escrever o changelog descrevendo commits internos em vez do efeito para o usuário.
