# Secrets Storage

Aplicativo desktop open source, local-first e zero-knowledge para armazenar senhas, chaves de API e outros segredos em um cofre criptografado. A proposta é sincronizar somente dados cifrados pelo OneDrive ou Google Drive escolhido pelo usuário, sem entregar ao provedor conteúdo legível ou material suficiente para descriptografá-lo.

> [!IMPORTANT]
> O projeto está no marco **M0 — Fundação de segurança**: o modelo de ameaças foi aprovado como base de design, mas os controles ainda dependem de implementação e evidência. Não há release utilizável para armazenar segredos reais.

## Princípios

- **Local-first:** o cofre continua utilizável sem conexão e sincroniza quando a rede volta.
- **Zero-knowledge:** provedores remotos recebem apenas blobs cifrados e os metadados mínimos necessários.
- **Segurança por sessão:** cada sessão persistente tem sua própria senha mestra, estado de bloqueio e política de inatividade.
- **Sem perda silenciosa:** edições concorrentes devem ser mescladas com segurança ou apresentadas para resolução explícita.
- **Atualizações autenticadas:** manifestos e pacotes inválidos devem falhar de forma segura.
- **Criptografia revisável:** o projeto usará primitivas modernas, bibliotecas auditadas e um formato versionado; nenhuma criptografia própria.

## Escopo planejado para o v1

- Aplicativo para Windows com múltiplas sessões independentes e nomeadas.
- Registros de senha, API key, token genérico, nota secreta e chave SSH.
- Bloqueio manual, automático por inatividade e integrado a eventos do Windows.
- Pesquisa nas sessões desbloqueadas e movimentação de segredos entre sessões abertas.
- Limpeza configurável do clipboard, com ação imediata para limpar agora.
- Operação offline e sincronização automática de blobs cifrados pelo OneDrive ou Google Drive.
- Modo somente leitura por sessão e dispositivo.
- Mesclagem automática quando segura e resolução de conflitos campo a campo quando necessária.
- Auto-update por releases autenticadas no GitHub.
- Exportação e backup cifrados no fluxo normal.

Não fazem parte do v1: macOS, Linux, dispositivos móveis, cofres compartilhados, extensão de navegador, autofill, TOTP, anexos e recuperação de acesso. Se a senha mestra for perdida, o v1 não oferecerá um mecanismo para recuperar a sessão.

## Stack planejada

| Camada | Tecnologia |
| --- | --- |
| Aplicativo desktop | Tauri 2 |
| Core | Rust |
| Frontend | Vue 3, TypeScript, Vite e Tailwind CSS |
| Autorização da nuvem | OAuth 2.0 |
| Provedores | Microsoft Graph e Google Drive API |
| Atualizações | Tauri Updater e GitHub Releases |
| Distribuição inicial | Windows / NSIS |

O formato do cofre, a hierarquia de chaves, as primitivas e os parâmetros criptográficos ainda serão definidos após a aprovação do modelo de ameaças e a validação dos protótipos críticos.

## Roadmap

| Marco | Objetivo | Estado |
| --- | --- | --- |
| M0 — Fundação de segurança | Aprovar o modelo de ameaças, especificar o formato criptográfico e validar protótipos críticos | Em planejamento |
| M1 — Cofre local utilizável | Entregar sessões e gerenciamento local de segredos sem depender da nuvem | Planejado |
| M2 — Sincronização zero-knowledge | Sincronizar com segurança entre dispositivos Windows | Planejado |
| M3 — Distribuição segura do v1 | Publicar uma versão verificável, atualizável e preparada para auditoria | Planejado |
| M4 — Expansão de plataformas | Levar o cofre ao macOS e Linux sem reduzir as garantias de segurança | Futuro |

Consulte o [roadmap detalhado](./.specs/project/ROADMAP.md) para os critérios de cada marco.

## Desenvolvimento

Pré-requisitos no Windows: Node.js 24 LTS, Rust stable com target MSVC, Microsoft C++ Build Tools, Windows SDK e WebView2.

```powershell
pnpm install --frozen-lockfile
pnpm check
pnpm dev
```

O scaffold Tauri 2, Vue 3, TypeScript e Tailwind está executável, mas contém apenas uma tela de fundação. Ainda não existe armazenamento de segredos nem implementação criptográfica.

Use `pnpm dev` para executar o aplicativo via Tauri e `pnpm build` para gerar o build desktop. Os comandos do frontend são hooks internos do Tauri, não scripts públicos.

## Distribuição e atualizações

O workflow de release é acionado somente por uma tag `vX.Y.Z`, valida a versão nos três manifests e se o commit pertence à `main`, executa todos os gates e cria uma GitHub Release em **draft**. O pacote primário é NSIS; `latest.json` e a assinatura do updater são gerados no mesmo build.

Antes da primeira release, crie no GitHub o environment protegido `release` e configure nele:

- variável `TAURI_UPDATER_PUBLIC_KEY` com a chave pública completa;
- secrets `TAURI_SIGNING_PRIVATE_KEY` e `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`;
- proteção de deployment limitada a tags `v*` e, quando disponível, aprovação manual.

Gere o par de chaves fora do repositório com `pnpm tauri signer generate`. Guarde a chave privada e seu backup em armazenamento seguro; nunca adicione a chave privada ao Git. O workflow injeta a configuração do updater apenas durante a release, deixando builds locais sem material de assinatura.

O updater está registrado no core Rust e não concede capability de atualização à WebView. A interface de busca, confirmação e reinício da atualização será implementada como um fluxo Rust controlado antes da primeira distribuição pública. A assinatura Authenticode também permanece um gate obrigatório para publicação pública.

Os próximos gates são:

1. executar os protótipos de segurança que bloqueiam decisões de arquitetura;
2. definir o formato criptográfico versionado;
3. configurar o environment e as chaves de assinatura no GitHub;
4. iniciar o cofre local somente após fechar as decisões bloqueadoras.

## Documentação

- [Visão, objetivos e escopo](./.specs/project/PROJECT.md)
- [Roadmap](./.specs/project/ROADMAP.md)
- [Especificação do cofre seguro v1](./.specs/features/secure-vault/spec.md)
- [Modelo de ameaças](./.specs/features/secure-vault/threat-model.md)
- [Decisões e estado do projeto](./.specs/project/STATE.md)
- [Política de versões e releases](./.specs/project/RELEASES.md)
- [Guia de contribuição](./CONTRIBUTING.md)

## Contribuindo

Contribuições devem partir de uma branch curta e entrar por pull request. Commits e títulos de PR seguem Conventional Commits, com tipo e escopo em inglês e descrição em português. Consulte [CONTRIBUTING.md](./CONTRIBUTING.md) para o fluxo completo e as exigências adicionais para mudanças sensíveis à segurança.

## Segurança

O modelo de ameaças foi aprovado como base de design, não como certificação da implementação. Não use o projeto para armazenar segredos reais antes que a implementação, os testes de segurança e a revisão independente estejam concluídos.

Vulnerabilidades não devem ser publicadas em issues enquanto um canal privado de divulgação ainda não estiver documentado.
