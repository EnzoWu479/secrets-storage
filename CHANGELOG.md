# Changelog

Todas as mudanças relevantes deste projeto serão documentadas neste arquivo.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/) e o projeto usa [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Unreleased]

## [0.1.7] - 2026-07-14

### Added

- Fundação do aplicativo desktop com Tauri 2, Vue 3, TypeScript e Tailwind CSS.
- Automação inicial de CI, distribuição Windows via NSIS e artefatos assinados para atualização.
- Verificação automática de atualizações ao abrir o aplicativo.
- Exibição da versão instalada na tela principal.

### Changed

- Pipeline de release mais rápido, reutilizando o cache das dependências Rust entre builds.
