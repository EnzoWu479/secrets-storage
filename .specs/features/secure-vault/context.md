# Cofre Seguro v1 — Contexto

**Gathered:** 2026-07-13
**Spec:** `.specs/features/secure-vault/spec.md`
**Status:** Threat model drafted; awaiting security review

---

## Feature Boundary

O v1 entrega um aplicativo Windows local-first e zero-knowledge para múltiplas sessões de segurança individuais, gerenciamento dos cinco tipos de segredo definidos, sincronização cifrada via OneDrive ou Google Drive e atualizações autenticadas. Recuperação de acesso fica fora do v1.

---

## Implementation Decisions

### Stack e direção visual

- O aplicativo desktop usa Tauri 2 com core em Rust.
- O frontend usa Vite e Tailwind CSS.
- A primeira direção visual será funcional, limpa e genérica, priorizando hierarquia, acessibilidade e validação dos fluxos de segurança.
- Uma identidade mais única e característica será explorada posteriormente, sem bloquear a arquitetura ou a primeira implementação.
- Dependências, assets e estilos do frontend devem ser empacotados localmente; a interface não carregará código remoto em runtime.

### Sessões e bloqueio

- “Sessão” é um contêiner persistente de segredos e configurações, não apenas uma execução temporária do aplicativo.
- O usuário pode criar múltiplas sessões de segurança para separar dados por contexto e atribuir um nome visível a cada uma, como “Trabalho”, “Pessoal” ou um projeto específico.
- Nomes de sessões são únicos sem diferenciar maiúsculas de minúsculas; por exemplo, “Trabalho” e “trabalho” entram em conflito.
- O usuário pode renomear uma sessão somente enquanto ela estiver desbloqueada.
- Cada sessão tem sua própria senha mestra, estado de bloqueio e política configurável de bloqueio automático.
- O temporizador conta a inatividade desde o desbloqueio ou desde a última interação intencional dentro daquela sessão.
- Atividade em uma sessão reinicia somente o cronômetro dela, sem afetar outras sessões desbloqueadas.
- A política usa 15 minutos por padrão e é selecionada em um controle contínuo de 1 minuto até a opção “nunca”; escolher “nunca” exige confirmação explícita de que a sessão não terá bloqueio automático.
- Enquanto o aplicativo está minimizado, o tempo continua contando como inatividade.
- Novas sessões vêm configuradas para bloquear tanto ao bloquear quanto ao suspender o Windows; cada uma dessas reações pode ser desativada individualmente por sessão.
- Fechar o aplicativo bloqueia todas as sessões.
- Uma sessão desbloqueada não concede acesso a outra sessão bloqueada; a senha mestra da sessão bloqueada continua obrigatória.
- Nomes e quantidade de sessões permanecem visíveis quando elas estão bloqueadas.
- Excluir uma sessão exige confirmação e a senha mestra da própria sessão.

### Senha mestra e tentativas

- A criação e a troca da senha mestra exigem comprimento mínimo e exibem um indicador de força.
- Tentativas incorretas geram atraso progressivo.
- A sessão pode ter uma dica de senha sincronizada entre dispositivos como metadado não secreto para a aplicação.
- Na tela bloqueada, a dica aparece somente após o usuário clicar em “Mostrar dica”.
- Antes de salvar a dica, o aplicativo avisa que ela pode ser vista sem senha e não deve conter a senha mestra nem partes óbvias dela.
- Trocar a senha exige a senha atual.
- O aplicativo avisa periodicamente que o v1 não oferece recuperação de acesso.
- Um futuro fluxo “Esqueci minha senha” precisa de mecanismo próprio e continua fora do v1.

### Pesquisa e movimentação

- Uma pesquisa pode consultar simultaneamente todas as sessões atualmente desbloqueadas.
- Mover um segredo entre sessões exige que origem e destino estejam desbloqueados.

### Clipboard

- A limpeza automática é configurável e usa 5 minutos como padrão.
- A interface oferece a ação “Limpar agora”.
- A experiência deve comunicar limitações do sistema operacional quando a limpeza não puder ser garantida.

### Sincronização e conflitos

- A sincronização ocorre automaticamente e segue o princípio conceitual do Git: obter mudanças, enviar mudanças e reconciliar históricos.
- Sessões bloqueadas continuam sincronizando automaticamente seus blobs cifrados, sem descriptografia.
- O modo somente leitura é configurado por sessão em cada dispositivo.
- Nesse modo, a sessão recebe atualizações normalmente, mas não produz alterações em segredos naquele dispositivo.
- Habilitar edição novamente exige a senha mestra da sessão.
- O aplicativo tenta resolver conflitos automaticamente quando isso puder ser feito sem perda.
- Conflitos restantes são apresentados para resolução campo a campo, oferecendo “manter local”, “manter remoto” e “manter ambos”. Para campos de valor único, “manter ambos” cria duas entradas separadas.
- Um mecanismo dedicado acompanha os conflitos pendentes e suas versões.
- Conflitos são preservados por 30 dias; durante os 7 dias finais, o aplicativo mantém um aviso persistente e envia uma notificação diária sobre a expiração.
- Ao completar 30 dias sem resolução, nenhuma versão é apagada: elas são transformadas em entradas permanentes identificadas como “local” e “remota”.
- Se houver mais de uma versão da mesma origem, a numeração é usada somente nesse caso: “local 1”, “local 2”, “remota 1”, “remota 2” e assim por diante.
- Depois de uma resolução manual, as versões anteriores permanecem disponíveis por mais 7 dias para permitir desfazer.
- Essa referência não determina que Git seja usado internamente e não altera o requisito de sincronizar somente dados cifrados.

### Recuperação

- O kit de recuperação foi retirado do v1 sem mecanismo substituto por enquanto.
- A experiência deve avisar que perder a senha mestra implica perder o acesso à sessão correspondente.

### Agent's Discretion

- Nenhuma discricionariedade técnica foi concedida para os controles criptográficos; eles dependem do modelo de ameaças e de pesquisa em fontes oficiais.

---

## Specific References

- Usar o Git como referência de comportamento para sincronização e conflitos, não necessariamente como tecnologia de armazenamento.

---

## Deferred Ideas

- Kit local de recuperação ou outro mecanismo seguro de recuperação de acesso.
- Cadência exata para o lembrete periódico de que não existe recuperação de acesso.

---

## Technical Questions for Design

- Definir, pelo modelo de ameaças, quais ataques com acesso físico serão mitigados, parcialmente mitigados, aceitos ou considerados fora do modelo.
