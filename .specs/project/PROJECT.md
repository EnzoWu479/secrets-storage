# Secrets Storage

**Vision:** Aplicativo desktop open source, local-first e zero-knowledge para armazenar senhas, chaves de API e outros segredos em um cofre fortemente criptografado, sincronizado pelo armazenamento em nuvem escolhido pelo usuário.
**For:** Usuários individuais no Windows, incluindo pessoas técnicas e o público geral.
**Solves:** Centraliza segredos sensíveis sem confiar ao provedor de nuvem conteúdo legível ou material suficiente para descriptografá-los.

## Goals

- Permitir que 100% dos tipos de segredo do v1 sejam criados, consultados, alterados e removidos em sessões de segurança independentes, cada uma protegida por sua própria senha mestra.
- Garantir que nenhum segredo, senha mestra ou chave de dados seja enviado em texto aberto ao OneDrive, Google Drive, GitHub ou serviços de telemetria.
- Sincronizar o mesmo cofre entre dispositivos Windows sem perda silenciosa de dados, inclusive diante de edições concorrentes e falhas de rede.
- Distribuir atualizações autenticadas pelo mecanismo de auto-update do Tauri, com falha segura diante de artefatos ou metadados inválidos.

## Tech Stack

**Core:**

- Framework desktop: Tauri 2
- Backend: Rust
- Frontend: Vite e Tailwind CSS
- Direção visual inicial: interface funcional e genérica; identidade visual própria será explorada após validar estrutura, fluxos e segurança
- Persistência local: formato de cofre criptografado a definir na fase de design

**Integrações principais:**

- Tauri Updater para atualizações publicadas via GitHub
- OAuth 2.0 para autorização do OneDrive ou Google Drive
- APIs oficiais de armazenamento do Microsoft Graph e Google Drive
- Armazenamento seguro do Windows para material local apropriado, sujeito ao modelo de ameaças

As bibliotecas e primitivas criptográficas específicas serão escolhidas somente após pesquisa em documentação oficial, revisão do modelo de ameaças e definição do formato versionado do cofre.

## Scope

**v1 includes:**

- Aplicativo Windows para um único usuário, com múltiplas sessões de segurança persistentes, independentes e nomeadas pelo usuário para contextos como “Trabalho”, “Pessoal” ou projetos específicos; nomes são únicos sem diferenciar maiúsculas de minúsculas e podem ser alterados enquanto a sessão estiver desbloqueada.
- Cada sessão protegida por sua própria senha mestra, estado de bloqueio e política configurável por inatividade, com padrão de 15 minutos e intervalo de 1 minuto até a opção consciente e confirmada de nunca bloquear automaticamente.
- Registros de senha, chave de API, token genérico, nota secreta e chave SSH.
- Bloqueio manual e automático, limpeza de dados sensíveis da interface e proteção de operações enquanto bloqueado.
- Limpeza configurável do clipboard, com padrão de 5 minutos e ação imediata “Limpar agora”.
- Sincronização multidispositivo pelo OneDrive ou Google Drive escolhido via OAuth 2.0.
- Sincronização automática de blobs cifrados inclusive para sessões bloqueadas, sem descriptografá-las, com fluxo conceitual de push/pull e modo somente leitura configurado por sessão em cada dispositivo.
- Modo somente leitura que recebe atualizações normalmente, não produz alterações em segredos e exige a senha mestra para habilitar edição.
- Tentativa de mesclagem automática e resolução campo a campo pelo usuário quando restarem conflitos.
- Preservação de conflitos pendentes por 30 dias, com aviso persistente e notificação diária durante os 7 dias finais; ao expirar, as versões tornam-se entradas permanentes locais e remotas, sem perda silenciosa.
- Retenção por 7 dias das versões anteriores após resolução manual para permitir desfazer.
- Operação local durante indisponibilidade da nuvem e sincronização posterior.
- Auto-update por releases do GitHub com verificação de autenticidade.
- Exportação e backup somente em formato criptografado no fluxo normal.
- Modelo de ameaças, testes de segurança e documentação clara dos limites de proteção.

**Explicitly out of scope for v1:**

- macOS, Linux e plataformas móveis.
- Compartilhamento de cofres, famílias, equipes ou organizações.
- Extensão de navegador e preenchimento automático em outros aplicativos.
- Arquivos e anexos, cartões, documentos de identidade e geração de códigos TOTP.
- Recuperação por e-mail, suporte, Microsoft, Google ou qualquer serviço que conheça a senha mestra.
- Kit de recuperação ou outro mecanismo de recuperação de acesso no v1.
- Armazenamento ou sincronização de segredos em texto aberto.
- Promessa de segurança absoluta contra sistema operacional já comprometido, captura física coercitiva ou vulnerabilidades desconhecidas.

## Constraints

- Segurança: arquitetura zero-knowledge; provedores remotos recebem apenas dados cifrados e metadados mínimos inevitáveis.
- Plataforma inicial: Windows; portabilidade futura deve ser preservada onde não conflitar com a segurança do v1.
- Produto: open source e adequado a usuários técnicos e não técnicos.
- Criptografia: somente primitivas modernas, bibliotecas auditadas e formatos versionados; nenhuma criptografia própria.
- Atualizações: pacotes e manifestos devem ser autenticados, com proteção contra downgrade definida no design.
- Desenvolvimento: Conventional Commits, histórico linear por squash, SemVer e releases imutáveis geradas no GitHub Actions conforme [RELEASES.md](./RELEASES.md).
- Privacidade: telemetria desativada por padrão; nenhum segredo pode aparecer em logs, relatórios de erro ou área de transferência além do período necessário.
- Garantia: decisões criptográficas críticas exigem revisão independente antes de uma versão considerada estável.
- Ameaças físicas: o modelo de ameaças deve avaliar acesso offline ao disco e ataques avançados com acesso ao equipamento, documentando controles, pré-requisitos e limites sem prometer proteção absoluta contra um host comprometido durante o uso.
