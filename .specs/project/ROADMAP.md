# Roadmap

**Current Milestone:** M0 — Fundação de segurança
**Status:** In Progress

---

## M0 — Fundação de segurança

**Goal:** Produzir uma arquitetura implementável e revisável antes de manipular segredos reais.
**Target:** Modelo de ameaças aprovado, formato criptográfico especificado, protótipos críticos validados e critérios de segurança documentados.

### Features

**Modelo de ameaças e requisitos de segurança** — COMPLETE (reaberto em revisão por AD-022)

- Definir ativos, fronteiras de confiança, adversários e ataques previstos.
- Cobrir memória, disco, logs, clipboard, arquivos temporários, IPC, interface, cadeia de atualização, OAuth e sincronização.
- Documentar ameaças mitigadas, parcialmente mitigadas, aceitas e fora do modelo.
- Revisar e aprovar o [modelo de ameaças do v1](../features/secure-vault/threat-model.md) antes de fechar o formato criptográfico.
- ⚠️ **Reaberto (AD-022, 2026-07-15):** a senha mestra global (GMP) e o `auth_mode = global` alteram isolamento e raio de exposição; SEC-01/SEC-03 voltam a `Em revisão` e o modelo exige **nova aprovação humana** antes de servir de base estável de design.

**Formato criptográfico versionado** — PLANNED

- Definir derivação da senha mestra, hierarquia de chaves, criptografia autenticada e rotação.
- Definir formato do cofre, metadados autenticados e estratégia de migração.
- Criar vetores de teste e plano de revisão independente.

**Prova de integração Windows e Tauri** — PLANNED

- Validar isolamento entre interface e comandos privilegiados.
- Validar armazenamento seguro local e comportamento de memória possível no Windows.
- Validar empacotamento sem incluir credenciais ou chaves privadas.

---

## M1 — Cofre local utilizável

**Goal:** Entregar um cofre Windows local com sessões de segurança independentes e bloqueáveis, sem dependência da nuvem.

### Features

**Sessões de segurança e desbloqueio por senha mestra** — PLANNED

- Criar múltiplas sessões persistentes e nomeadas pelo usuário para separar contextos como trabalho, uso pessoal ou projetos específicos, cada uma com sua própria senha mestra e estado de bloqueio.
- Exigir nomes únicos sem diferenciar maiúsculas de minúsculas e permitir renomear somente sessões desbloqueadas.
- Criar, desbloquear, bloquear e trocar a senha mestra de cada sessão sem desbloquear as demais.
- Aplicar política configurável de bloqueio por inatividade, com padrão de 15 minutos e intervalo de 1 minuto até “nunca”, reiniciada somente por interação intencional dentro da própria sessão.
- Ativar por padrão o bloqueio da sessão ao bloquear ou suspender o Windows, permitir desativar cada evento individualmente e bloquear todas as sessões ao encerrar o aplicativo.
- Exibir indicador de força e exigir comprimento mínimo da senha mestra; permitir dica sincronizada como metadado não secreto, revelada sob demanda na tela bloqueada com aviso contra incluir a senha ou partes óbvias dela.
- Exigir a senha atual para troca e aplicar limitação progressiva de tentativas locais.
- Exibir periodicamente que não existe recuperação de senha no v1.
- Evitar persistência acidental de material sensível.

**Gerenciamento de segredos** — PLANNED

- Armazenar senhas, API keys, tokens, notas secretas e chaves SSH.
- Pesquisar simultaneamente nas sessões desbloqueadas e exigir origem e destino desbloqueados para mover registros entre sessões.
- Exibir nomes e quantidade de sessões mesmo quando bloqueadas; exigir confirmação e senha mestra para excluir uma sessão.
- Copiar valores com limpeza automática configurável, padrão de 5 minutos, avisos adequados e ação “Limpar agora”.

---

## M2 — Sincronização zero-knowledge

**Goal:** Sincronizar com segurança entre dispositivos Windows usando armazenamento controlado pelo usuário.

### Features

**Conexão OAuth com provedor** — PLANNED

- Autorizar OneDrive ou Google Drive com escopo mínimo.
- Guardar tokens locais com proteção adequada e permitir revogação.
- Nunca enviar senha mestra, chaves de dados ou conteúdo legível ao provedor.

**Sincronização offline e multidispositivo** — PLANNED

- Operar offline e reconciliar mudanças posteriormente.
- Sincronizar automaticamente blobs cifrados mesmo para sessões bloqueadas, sem descriptografá-las.
- Oferecer modo somente leitura por sessão e por dispositivo, recebendo atualizações sem produzir alterações em segredos e exigindo a senha mestra para habilitar edição.
- Tentar mesclar edições concorrentes automaticamente e encaminhar conflitos não resolvidos para decisão campo a campo, com opções de manter local, remoto ou ambos.
- Preservar conflitos por 30 dias, com aviso persistente e notificação diária nos 7 dias finais; ao expirar, materializar as versões como entradas permanentes locais e remotas, numeradas somente quando necessário.
- Manter versões anteriores por 7 dias após resolução manual para permitir desfazer; ao escolher “manter ambos” para valor único, criar entradas separadas.
- Resistir a corrupção, rollback remoto e replay conforme o modelo de ameaças.

---

## M3 — Distribuição segura do v1

**Goal:** Publicar uma versão Windows verificável, atualizável e pronta para auditoria pública.

### Features

**Auto-update via GitHub** — PLANNED

- Publicar releases por pipeline controlado e reproduzível onde viável.
- Verificar autenticidade de manifestos e pacotes antes da instalação.
- Falhar de forma segura e informar claramente erros de atualização.
- Aplicar a [política de commits, SemVer e releases](./RELEASES.md): Release PR, tag assinada, build em GitHub Actions, draft verificado e publicação imutável.
- Publicar NSIS como instalador/updater primário do v1, com assinatura Tauri obrigatória e Authenticode para distribuição pública.

**Hardening e validação de segurança** — PLANNED

- Executar testes unitários, integração, propriedades, fuzzing e casos de corrupção.
- Revisar dependências, permissões Tauri, CSP, IPC e cadeia de build.
- Preparar auditoria independente, política de divulgação e resposta a vulnerabilidades.
- Avaliar ataques offline e ataques avançados com acesso físico ao equipamento, incluindo proteções disponíveis no hardware e no Windows e seus limites.

**Experiência e documentação pública** — PLANNED

- Criar onboarding compreensível para usuários não técnicos.
- Documentar backups, restauração de cópias válidas, conflitos, limites e práticas seguras.
- Publicar código-fonte, processo de build e política de releases.

---

## M4 — Expansão de plataformas

**Goal:** Levar o cofre a outros desktops sem reduzir as garantias do Windows.

### Features

**Suporte a macOS** — PLANNED

**Suporte a Linux** — PLANNED

**Compatibilidade segura entre plataformas** — PLANNED

- Compartilhar o formato versionado do cofre.
- Adaptar armazenamento seguro e hardening às garantias de cada sistema.

---

## Future Considerations

- Aplicativos móveis.
- Extensão de navegador e preenchimento automático.
- TOTP, anexos, cartões e identidades.
- Compartilhamento criptografado entre usuários.
- Suporte opcional a passkeys, biometria ou chaves físicas como fatores adicionais.
- Kit local de recuperação ou outro mecanismo de recuperação de acesso.
- Cadência exata dos lembretes periódicos de ausência de recuperação.
