# State

**Last Updated:** 2026-07-14
**Current Work:** M0 — executar protótipos de segurança que bloqueiam o design criptográfico

---

## Recent Decisions (Last 60 days)

### AD-001: Público e modelo de produto (2026-07-13)

**Decision:** Produto open source para cofres individuais, acessível a usuários técnicos e ao público geral.
**Reason:** Atender uso pessoal sem restringir a solução a um nicho exclusivamente técnico.
**Trade-off:** Recursos de equipes e compartilhamento ficam fora do v1.
**Impact:** A interface deve ser simples, mas as garantias e configurações de segurança precisam ser transparentes e auditáveis.

### AD-002: Plataforma inicial (2026-07-13)

**Decision:** Entregar o v1 somente para Windows e manter macOS e Linux no roadmap.
**Reason:** Reduzir a superfície inicial e permitir hardening específico da plataforma.
**Trade-off:** Adoção inicial limitada a Windows.
**Impact:** O formato do cofre deve ser portável, enquanto proteções locais podem ser específicas do Windows.

### AD-003: Recuperação local (2026-07-13) — SUPERADA POR AD-010

**Status:** Superada; preservada somente como histórico da decisão anterior.

**Decision:** Recuperar cofres por kit local criado pelo usuário, sem recuperação por terceiros.
**Reason:** Manter o modelo zero-knowledge sem tornar o esquecimento da senha necessariamente fatal.
**Trade-off:** Perder simultaneamente senha e kit implica perda definitiva do acesso.
**Impact:** O fluxo de criação precisa confirmar backup do kit e o design deve definir revogação e rotação após uso.

### AD-004: Sincronização no v1 (2026-07-13)

**Decision:** O v1 sincronizará entre múltiplos dispositivos via OneDrive ou Google Drive.
**Reason:** Permitir continuidade entre computadores sem operar infraestrutura própria de conteúdo.
**Trade-off:** OAuth, conflitos, rollback e disponibilidade da nuvem ampliam o modelo de ameaças.
**Impact:** O armazenamento remoto conterá apenas blobs cifrados; conflitos nunca poderão causar perda silenciosa.

### AD-005: Tipos de segredo do v1 (2026-07-13)

**Decision:** Suportar senhas, chaves de API, tokens genéricos, notas secretas e chaves SSH.
**Reason:** Cobrir uso geral e técnico sem introduzir anexos binários no primeiro formato.
**Trade-off:** TOTP, arquivos, cartões e identidades ficam adiados.
**Impact:** O modelo de dados precisa aceitar campos sensíveis tipados e extensibilidade futura.

### AD-006: Aprovação da especificação inicial (2026-07-13)

**Decision:** A visão, o roadmap e a especificação inicial do Cofre Seguro v1 foram aprovados pelo usuário.
**Reason:** Os objetivos, limites e requisitos refletem o produto pretendido.
**Trade-off:** Alterações posteriores de escopo precisarão ser avaliadas e rastreadas explicitamente.
**Impact:** O trabalho pode avançar para discussão das áreas ambíguas antes do design.

### AD-007: Sessões de segurança independentes (2026-07-13)

**Decision:** O aplicativo permitirá múltiplas sessões de segurança persistentes e nomeadas pelo usuário para representar contextos como “Trabalho”, “Pessoal” ou projetos específicos. Os nomes serão únicos sem diferenciar maiúsculas de minúsculas e poderão ser alterados somente enquanto a sessão estiver desbloqueada. Cada sessão terá sua própria senha mestra, estado de bloqueio e período configurável de bloqueio automático, inclusive a opção explícita de nunca bloquear automaticamente.
**Reason:** Segredos de contextos e níveis de confidencialidade diferentes precisam de separação identificável e políticas proporcionais sem obrigar o usuário a aplicar o mesmo nível de atrito a tudo.
**Trade-off:** Múltiplas senhas e estados aumentam a complexidade da interface, do gerenciamento de chaves e dos testes; escolher “nunca” reduz a proteção daquela sessão.
**Impact:** A sessão é um contêiner persistente, não uma execução temporária. A criação e renomeação precisam validar unicidade normalizada; desbloquear uma sessão não desbloqueia nenhuma outra e entrar em uma sessão bloqueada sempre exige sua própria senha mestra.

### AD-008: Limpeza configurável do clipboard (2026-07-13)

**Decision:** A limpeza automática do clipboard será configurável, com padrão de 5 minutos, e haverá uma ação “Limpar agora”.
**Reason:** Equilibrar conveniência com a redução do tempo de exposição de um segredo copiado.
**Trade-off:** Cinco minutos ampliam a janela de exposição em relação a um intervalo curto; a limpeza continua sujeita às limitações do clipboard e do sistema operacional.
**Impact:** A interface deve informar o temporizador, permitir configuração e não afirmar sucesso quando a limpeza não puder ser confirmada.

### AD-009: Sincronização com semântica inspirada no Git (2026-07-13)

**Decision:** A sincronização seguirá o modelo conceitual de enviar e obter mudanças (push/pull), tentará mesclar mudanças automaticamente e encaminhará conflitos não resolvidos para decisão explícita do usuário, preservando todas as versões relevantes.
**Reason:** Evitar perda silenciosa e dar controle ao usuário quando dois dispositivos alterarem o mesmo conteúdo.
**Trade-off:** Histórico, detecção de ancestralidade e resolução de conflitos tornam o formato e a experiência mais complexos.
**Impact:** “Inspirada no Git” define o comportamento, não obriga a usar Git internamente nem permite que conteúdo legível seja enviado ao provedor.

### AD-010: Kit de recuperação adiado (2026-07-13)

**Decision:** O v1 não terá kit de recuperação nem mecanismo substituto por enquanto.
**Reason:** O mecanismo precisa de mais reflexão antes de introduzir material adicional capaz de recuperar acesso.
**Trade-off:** Perder a senha mestra de uma sessão implica perda definitiva de acesso aos dados daquela sessão no v1.
**Impact:** O kit sai dos objetivos e do roadmap do v1 e permanece como ideia futura; o onboarding deve explicar claramente a ausência de recuperação.

### AD-011: Acesso físico avançado no modelo de ameaças (2026-07-13)

**Decision:** O modelo de ameaças avaliará explicitamente adversários tecnicamente capacitados com acesso ao equipamento, separando ataques offline de ataques contra um sistema já comprometido durante o uso.
**Reason:** Reduzir o risco de extração de senhas, chaves ou dados cifrados por quem conhece hardware e mecanismos de baixo nível.
**Trade-off:** Algumas ameaças podem apenas ser mitigadas ou depender de recursos como hardware compatível, configuração segura do Windows e proteção de disco; não haverá promessa de segurança absoluta.
**Impact:** A pesquisa de M0 deve avaliar derivação de chave resistente a ataques offline, proteção de memória, apagamento, armazenamento apoiado por hardware e garantias/limites do Windows antes do design criptográfico.

### AD-012: Bloqueio orientado por inatividade e eventos do sistema (2026-07-13)

**Decision:** O bloqueio automático usa 15 minutos por padrão, conta inatividade independentemente por sessão e reinicia somente após interação intencional dentro dela; continua contando enquanto o aplicativo está minimizado e pode ser configurado de 1 minuto até “nunca”. Novas sessões bloqueiam por padrão ao bloquear ou suspender o Windows, mas cada reação pode ser desativada individualmente; fechar o aplicativo bloqueia todas.
**Reason:** Aplicar proteção proporcional por sessão sem confundir tempo de uso ativo com tempo de exposição abandonada.
**Trade-off:** Políticas diferentes podem deixar algumas sessões abertas após eventos do Windows; escolher “nunca” exige que o usuário aceite explicitamente esse risco.
**Impact:** O design deve implementar cronômetros independentes, reconhecer interações intencionais dentro da sessão, ativar por padrão os dois eventos do Windows e exigir confirmação da opção “nunca”.

### AD-013: Visibilidade e operações entre sessões (2026-07-13)

**Decision:** Nomes e quantidade de sessões permanecem visíveis quando bloqueadas; a pesquisa consulta todas as sessões desbloqueadas; mover um segredo exige origem e destino desbloqueados; excluir uma sessão exige confirmação e sua senha mestra.
**Reason:** Manter navegação e organização convenientes sem atravessar as fronteiras criptográficas de sessões bloqueadas.
**Trade-off:** Nomes e quantidade de sessões tornam-se metadados visíveis antes do desbloqueio.
**Impact:** A interface e o índice de pesquisa devem respeitar dinamicamente o conjunto de sessões desbloqueadas.

### AD-014: Política da senha mestra (2026-07-13)

**Decision:** Senhas mestras terão comprimento mínimo, indicador de força, dica opcional e atraso progressivo após erros. A dica será sincronizada como metadado não secreto para a aplicação, aparecerá na tela bloqueada somente após “Mostrar dica” e terá aviso de que é visível sem senha e não deve conter a senha nem partes óbvias dela. A troca exige a senha atual e o aplicativo avisará periodicamente que não há recuperação no v1, com cadência a definir futuramente.
**Reason:** Orientar escolhas melhores e reduzir tentativas repetidas sem criar uma falsa promessa de recuperação.
**Trade-off:** A dica é metadado exposto a quem acessa a tela bloqueada e pode revelar informação; o atraso progressivo também pode afetar o usuário legítimo.
**Impact:** O design deve sincronizar e autenticar a dica sem tratá-la como segredo ou prova de acesso. Um futuro fluxo “Esqueci minha senha” e a cadência exata dos lembretes continuam adiados.

### AD-015: Sincronização automática e modo somente leitura (2026-07-13)

**Decision:** A sincronização ocorrerá automaticamente inclusive para sessões bloqueadas, transportando apenas blobs cifrados sem descriptografá-los. O modo somente leitura será configurado por sessão em cada dispositivo, continuará recebendo atualizações, não produzirá alterações nos segredos naquele dispositivo e exigirá a senha mestra para habilitar edição.
**Reason:** Manter dispositivos atualizados com menor esforço e permitir contextos de consulta sem edição acidental.
**Trade-off:** A sincronização bloqueada exige separar rigorosamente transporte cifrado de descriptografia; o modo somente leitura adiciona estado independente por sessão e dispositivo.
**Impact:** A arquitetura de sincronização deve operar sobre envelopes cifrados e autenticados, e a transição de somente leitura para edição deve passar pela autenticação da sessão.

### AD-016: Resolução e retenção de conflitos (2026-07-13)

**Decision:** Conflitos não resolvidos serão tratados por um mecanismo dedicado, comparados campo a campo e oferecerão manter o valor local, remoto ou ambos. Durante os 7 dias finais dos 30 dias de pendência haverá aviso persistente e notificação diária. Ao expirar, as versões tornam-se entradas permanentes “local” e “remota”, numeradas apenas quando houver múltiplas versões da mesma origem. “Manter ambos” cria entradas separadas para campos de valor único, e uma resolução manual pode ser desfeita por 7 dias.
**Reason:** Dar ao usuário controle granular e tempo previsível para impedir perda acidental em edições concorrentes.
**Trade-off:** A materialização evita perda, mas pode criar entradas duplicadas e aumentar o armazenamento quando conflitos forem ignorados.
**Impact:** Expiração significa encerrar a pendência, nunca apagar versões; o modelo de dados precisa registrar origem, numeração condicional e janela reversível de 7 dias.

### AD-017: Stack de frontend e direção visual inicial (2026-07-13)

**Decision:** O aplicativo usará Tauri 2 com core em Rust e frontend construído com Vite e Tailwind CSS. A primeira implementação seguirá um visual funcional e genérico; uma identidade mais única e característica será explorada depois da validação dos fluxos e da arquitetura.
**Reason:** Entregar rapidamente uma base consistente e ajustável sem transformar a definição de identidade visual em bloqueio para as decisões de segurança e usabilidade.
**Trade-off:** A primeira versão pode parecer menos diferenciada e o refinamento posterior exigirá uma etapa explícita de design visual.
**Impact:** Componentes e tokens devem ser organizados desde o início para permitir evolução estética sem reescrever os fluxos. O bundle do frontend permanece local, sob CSP e capabilities mínimas do Tauri.

### AD-018: Padrão de commits, versionamento e releases (2026-07-13)

**Decision:** O projeto usará Conventional Commits com descrição em português, branches curtas, PRs e squash merge sobre uma `main` protegida. Versões seguem SemVer, com série `0.x` antes da estabilidade, Release PR revisada, tag anotada e assinada `vX.Y.Z`, build no GitHub Actions e GitHub Release publicada de forma imutável. NSIS será o instalador/updater primário do v1.
**Reason:** Tornar histórico, incremento de versão, artefatos e origem do build previsíveis e auditáveis sem permitir que uma mensagem de commit publique automaticamente uma versão sensível.
**Trade-off:** O fluxo adiciona uma PR e confirmação manual por release; assinatura, smoke tests e imutabilidade tornam correções pós-publicação uma nova versão obrigatória.
**Impact:** `tauri.conf.json` será a fonte canônica da versão; workflows validarão manifests e tags, usarão permissões mínimas e separarão assinatura Tauri, Authenticode e attestations. A configuração remota está documentada em `.specs/project/RELEASES.md`.

### AD-019: Aprovação do modelo de ameaças do v1 (2026-07-14)

**Decision:** O modelo de ameaças foi aprovado como base para o design e a implementação do v1, incluindo riscos residuais, limites de garantia e protótipos bloqueadores documentados.
**Reason:** As fronteiras de confiança e os controles obrigatórios estão suficientemente definidos para orientar o scaffold e os experimentos de M0.
**Trade-off:** A aprovação não certifica controles ainda não implementados e mantém KDF, AEAD, formato, memória protegida e checkpoints bloqueados pelos protótipos correspondentes.
**Impact:** O projeto pode criar a fundação executável e avançar nos protótipos, mantendo os gates de release do modelo de ameaças.

### AD-020: Vue 3 e TypeScript no frontend (2026-07-14)

**Decision:** O frontend usará Vue 3 com TypeScript, Vite e Tailwind CSS.
**Reason:** A combinação foi definida pelo usuário e oferece uma base tipada, componentizada e empacotada localmente para a interface Tauri.
**Trade-off:** Vue adiciona runtime e dependências em relação a HTML/TypeScript puro, exigindo auditoria e atualização controlada.
**Impact:** Componentes, testes e configuração do frontend devem seguir Vue 3, sem conteúdo remoto em runtime e sob CSP estrita.

---

## Active Blockers

Nenhum.

## Lessons Learned

Nenhuma registrada.

## Quick Tasks Completed

Nenhuma.

## Deferred Ideas

- [ ] Suporte a macOS — Capturado durante: inicialização do projeto
- [ ] Suporte a Linux — Capturado durante: inicialização do projeto
- [ ] Extensão de navegador e preenchimento automático — Capturado durante: escopo do v1
- [ ] Aplicativos móveis — Capturado durante: roadmap
- [ ] Compartilhamento de cofres — Capturado durante: escopo do v1
- [ ] TOTP e anexos — Capturado durante: tipos de segredo
- [ ] Passkeys, biometria e chaves físicas — Capturado durante: desbloqueio
- [ ] Kit local de recuperação ou outro mecanismo de recuperação de acesso — Capturado durante: revisão do escopo do v1
- [ ] Definir a cadência exata dos lembretes de ausência de recuperação — Capturado durante: política da senha mestra

## Todos

- [x] Revisar e aprovar o modelo de ameaças do v1, incluindo riscos residuais e limites explícitos de garantia.
- [ ] Executar os protótipos críticos definidos no modelo de ameaças antes de escolher algoritmos e parâmetros finais.
- [ ] Definir arquitetura e formato criptográfico após aprovação do modelo de ameaças.
- [ ] Criar/configurar o repositório remoto e aplicar rulesets de `main` e tags `v*`, squash merge, environment `release` e immutable releases.
- [ ] Implementar os workflows de CI e release após o scaffold Tauri/Vite/Tailwind existir.
- [ ] Definir e contratar a estratégia de certificado Authenticode antes da primeira distribuição pública.

## Preferences

**Model Guidance Shown:** never
