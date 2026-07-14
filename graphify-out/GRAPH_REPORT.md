# Graph Report - .  (2026-07-14)

## Corpus Check
- cluster-only mode — file stats not available

## Summary
- 627 nodes · 523 edges · 131 communities (38 shown, 93 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS · INFERRED: 2 edges (avg confidence: 0.95)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `9a1289e8`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- SKILL.md
- README.md
- Implementation Decisions
- Modelo de Ameaças — Secrets Storage v1
- What You Must Do When Invoked
- compilerOptions
- Política de commits, versões e releases
- Tasks
- tauri.conf.json
- scripts
- Roadmap
- devDependencies
- Design
- Release Branch Script
- Process
- Tech Lead's Club - Spec-Driven Development
- Process
- Output: 7 Files in .specs/codebase/
- Phase: Codebase Concerns
- Process
- compilerOptions
- graphify reference: extra exports and benchmark
- Process
- During Implementation
- Testing Infrastructure
- Process
- default.json
- graphify reference: query, path, explain
- graphify reference: add a URL and watch a folder
- graphify reference: commit hook and native CLAUDE.md integration
- graphify reference: incremental update and cluster-only
- Changelog
- App.vue
- Regras do projeto
- graphify reference: GitHub clone and cross-repo merge
- graphify reference: transcribe video and audio
- extraction-spec.md
- vite-env.d.ts
- Debounced Rebuild
- Folder Watcher
- URL Ingestion
- Graph Export Formats
- MCP Graph Server
- Token Reduction Benchmark
- Extraction Confidence Rubric
- Deterministic Node IDs
- Graph Hyperedges
- Semantic Similarity Edges
- Cross-Repository Graph
- Monorepo Graph Merge
- CLAUDE.md Graphify Integration
- Post-Commit Graph Rebuild
- Constrained Query Expansion
- Graph Traversal
- Shortest Concept Path
- Graph Work Memory
- Domain-Prompted Transcription
- Whisper
- Cluster-Only Refresh
- Incremental Extraction
- Replace on Re-extract
- Community Detection
- Graphify Pipeline
- Graph Honesty Rules
- Semantic Extraction
- Structural Extraction
- Atomic Conventional Commits
- Four Adaptive Phases
- TLC Spec-Driven Development
- Seven Codebase Documents
- Testing Gate Matrix
- Graceful Search Degradation
- Structural Code Search
- Evidence-Backed Codebase Concerns
- Living Risk Documentation
- Context Budget Zones
- Feature Design
- Gray Area Discussion
- Locked Context Decisions
- Feature Scope Guardrail
- Workflow Auto-Sizing
- Testes unitários de frontend
- Gate completo de qualidade
- Gate de release Windows
- Testes Rust
- Artifact attestation
- Authenticode
- Versão canônica do aplicativo
- Conventional Commits
- Release pelo GitHub Actions
- Publicação imutável
- Instalador NSIS
- Main protegida e histórico linear
- Environment protegido release
- Release PR
- Semantic Versioning 2.0.0
- Tag anotada e assinada
- Assinatura do Tauri Updater
- Mandatory Graphify Update Rule
- TDD Gate and Atomic Commit
- Requirement Traceability
- Atomic Tasks
- Test Coverage Matrix
- Validate and Verify
- Fundação do aplicativo desktop
- Keep a Changelog
- Semantic Versioning
- Distribuição Windows e atualização assinada
- Contribution Workflow
- Secure Vault v1 Context
- Authenticated Updates
- Independent Security Sessions
- Secret Management
- Secure Vault v1 Specification
- Zero-Knowledge Cloud Sync
- Mandatory Security Controls
- Security-Blocking Prototypes
- Secrets Storage v1 Threat Model
- Trust Boundaries and Data Flow
- Tauri 2 Rust Vite Tailwind Stack
- M1 Usable Local Vault
- Product Roadmap
- M3 Secure v1 Distribution
- M0 Security Foundation
- M2 Zero-Knowledge Sync

## God Nodes (most connected - your core abstractions)
1. `compilerOptions` - 16 edges
2. `Modelo de Ameaças — Secrets Storage v1` - 15 edges
3. `Política de commits, versões e releases` - 14 edges
4. `What You Must Do When Invoked` - 12 edges
5. `Process` - 12 edges
6. `scripts` - 11 edges
7. `/graphify` - 11 edges
8. `Tech Lead's Club - Spec-Driven Development` - 11 edges
9. `Tasks` - 11 edges
10. `Design` - 9 edges

## Surprising Connections (you probably didn't know these)
- `tauri.conf.json Canonical Version Source` --semantically_similar_to--> `tauri.conf.json Canonical Version Source`  [EXTRACTED] [semantically similar]
  README.md → .specs/project/STATE.md
- `Safe Release PR Boundary` --semantically_similar_to--> `Reviewed Release PR`  [EXTRACTED] [semantically similar]
  README.md → .specs/project/STATE.md
- `Tauri Updater Artifacts` --semantically_similar_to--> `Draft Release with Signed Updater Artifacts`  [EXTRACTED] [semantically similar]
  README.md → .specs/project/STATE.md
- `Core-Controlled Updater` --semantically_similar_to--> `AD-021 Core-Controlled Updater Policy`  [EXTRACTED] [semantically similar]
  README.md → .specs/project/STATE.md

## Import Cycles
- None detected.

## Hyperedges (group relationships)
- **Fluxo de geração e publicação de versão** — _specs_project_releases_release_pr, _specs_project_releases_signed_tag, _specs_project_releases_github_actions_release, _specs_project_releases_immutable_publication [EXTRACTED 1.00]
- **Camadas independentes de autenticidade da release** — _specs_project_releases_tauri_updater_signature, _specs_project_releases_authenticode, _specs_project_releases_artifact_attestation [EXTRACTED 1.00]
- **Graphify Extraction Pipeline** — _agents_skills_graphify_skill_structural_extraction, _agents_skills_graphify_skill_semantic_extraction, _agents_skills_graphify_skill_community_detection [EXTRACTED 1.00]
- **TLC Adaptive Delivery Workflow** — _agents_skills_tlc_spec_driven_readme_four_adaptive_phases, _agents_skills_tlc_spec_driven_skill_auto_sizing, _agents_skills_tlc_spec_driven_readme_atomic_commits [EXTRACTED 1.00]
- **TLC Spec-Driven Delivery Pipeline** — agents_skills_tlc_spec_driven_references_specify_specify, agents_skills_tlc_spec_driven_references_tasks_atomic_tasks, agents_skills_tlc_spec_driven_references_implement_execute, agents_skills_tlc_spec_driven_references_validate_validate_verify [EXTRACTED 1.00]

## Communities (131 total, 93 thin omitted)

### Community 0 - "SKILL.md"
Cohesion: 0.05
Nodes (29): Code Analysis Tools, Detection, Fallback Notice, Search Scope, Tool Priority, Usage Examples, When to Use, Context Limits (+21 more)

### Community 1 - "README.md"
Cohesion: 0.06
Nodes (30): 🤖 Compatibility, 📋 Complete Command Reference, 🧠 Context Management, Design (when needed), Do's ✅, Don'ts ❌, Execute (always), ❓ FAQ (+22 more)

### Community 2 - "Implementation Decisions"
Cohesion: 0.06
Nodes (28): Agent's Discretion, Clipboard, Cofre Seguro v1 — Contexto, Deferred Ideas, Feature Boundary, Implementation Decisions, Pesquisa e movimentação, Recuperação (+20 more)

### Community 3 - "Modelo de Ameaças — Secrets Storage v1"
Cohesion: 0.07
Nodes (30): 10. Postura contra acesso físico e hardware, 11. Protótipos e testes que bloqueiam o design final, 12. Decisões ainda abertas, 13. Gates de release e manutenção, 14. Referências primárias, 1. Resumo executivo, 2. Método e vocabulário, 3. Objetivos de segurança (+22 more)

### Community 4 - "What You Must Do When Invoked"
Cohesion: 0.07
Nodes (26): For /graphify add and --watch, For /graphify query, For the commit hook and native CLAUDE.md integration, For --update and --cluster-only, /graphify, Honesty Rules, Interpreter guard for subcommands, Part A - Structural extraction for code files (+18 more)

### Community 5 - "compilerOptions"
Cohesion: 0.08
Nodes (25): DOM, DOM.Iterable, ES2020, src/**/*.d.ts, src/**/*.ts, src/**/*.tsx, src/**/*.vue, compilerOptions (+17 more)

### Community 6 - "Política de commits, versões e releases"
Cohesion: 0.08
Nodes (24): 10. Artefatos de cada release, 11. Hotfix e vulnerabilidade, 12. Configuração pendente no GitHub, 13. Referências oficiais, 1. Princípios, 2. Modelo Git, 3. Conventional Commits, 4. Semantic Versioning (+16 more)

### Community 7 - "Tasks"
Cohesion: 0.09
Nodes (22): 1.5. Load Test Coverage Matrix, 1. Review Design, 2. Break Into Atomic Tasks, 3. Define Dependencies, 4. Create Execution Plan, 5. Validate Before Presenting (MANDATORY), 6. ASK About MCPs and Skills, Diagram-Definition Cross-Check (+14 more)

### Community 8 - "tauri.conf.json"
Cohesion: 0.09
Nodes (22): icons/128x128@2x.png, icons/128x128.png, icons/32x32.png, icons/icon.ico, nsis, app, security, windows (+14 more)

### Community 9 - "scripts"
Cohesion: 0.09
Nodes (21): dependencies, @tauri-apps/api, vue, name, packageManager, private, scripts, build (+13 more)

### Community 10 - "Roadmap"
Cohesion: 0.11
Nodes (17): Branches, Commits, Contribuindo, Pull requests, Constraints, Goals, Scope, Secrets Storage (+9 more)

### Community 11 - "devDependencies"
Cohesion: 0.10
Nodes (21): jsdom, devDependencies, jsdom, tailwindcss, @tailwindcss/vite, @tauri-apps/cli, typescript, vite (+13 more)

### Community 12 - "Design"
Cohesion: 0.10
Nodes (19): 1.5. Research (Optional but Recommended), 1. Load Context, 2. Define Architecture, 3. Identify Code Reuse, 4. Define Components and Interfaces, 5. Define Data Models, Code Reuse Analysis, [Component Name] (+11 more)

### Community 13 - "Release Branch Script"
Cohesion: 0.14
Nodes (17): AD-018 Release Policy, AD-021 Core-Controlled Updater Policy, tauri.conf.json Canonical Version Source, Completed Release Branch Script, Draft Release with Signed Updater Artifacts, Ephemeral Release Configuration, Reviewed Release PR, Core-Controlled Updater (+9 more)

### Community 14 - "Process"
Cohesion: 0.12
Nodes (16): 0. List Atomic Steps (MANDATORY when Tasks phase was skipped), 1. Pick Task, 2. Verify Dependencies, 3. State Implementation Plan, 4. Write Tests First (RED), 4b. Implement (GREEN), 5. Gate Check (VERIFY), 6. Post-Gate Review (+8 more)

### Community 15 - "Tech Lead's Club - Spec-Driven Development"
Cohesion: 0.15
Nodes (13): Auto-Sizing: The Core Principle, Code Analysis, Code Exploration → codenavi, Commands, Context Loading Strategy, Diagrams → mermaid-studio, Knowledge Verification Chain, Output Behavior (+5 more)

### Community 16 - "Process"
Cohesion: 0.17
Nodes (12): 1. Describe the Task, 2. Pre-Implementation Check, 3. Implement, 4. Verify, 5. Commit, 6. Track, Guardrails, Process (+4 more)

### Community 17 - "Output: 7 Files in .specs/codebase/"
Cohesion: 0.18
Nodes (11): 1. STACK.md, 2. ARCHITECTURE.md, 3. CONVENTIONS.md, 4. STRUCTURE.md, 5. TESTING.md, 6. INTEGRATIONS.md, 7. CONCERNS.md, Brownfield Mapping (+3 more)

### Community 18 - "Phase: Codebase Concerns"
Cohesion: 0.20
Nodes (10): 1. Gather Evidence, 2. Classify and Document, 3. Prioritize by Risk, How CONCERNS.md Gets Used, Phase: Codebase Concerns, Process, Template: `.specs/codebase/CONCERNS.md`, What Belongs vs. What Doesn't (+2 more)

### Community 19 - "Process"
Cohesion: 0.20
Nodes (10): 1. Analyze the Feature, 2. Present Gray Areas, 3. Deep-Dive Each Area, 4. Scope Guardrail (CRITICAL), 5. Write context.md, Process, Specify: Discuss Gray Areas, Template: `.specs/features/[feature]/context.md` (+2 more)

### Community 20 - "compilerOptions"
Cohesion: 0.20
Nodes (9): vite.config.ts, vitest.config.ts, compilerOptions, allowSyntheticDefaultImports, composite, module, moduleResolution, skipLibCheck (+1 more)

### Community 21 - "graphify reference: extra exports and benchmark"
Cohesion: 0.22
Nodes (8): graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag), Step 8 - Token reduction benchmark (only if total_words > 5000)

### Community 22 - "Process"
Cohesion: 0.22
Nodes (9): 1. Check Completed Tasks, 2. Verify Acceptance Criteria, 3. Check Edge Cases, 4. Run Build-Level Gate Check (MANDATORY), 5. Code Quality Check (MANDATORY), 6. Interactive UAT (if user-facing feature), 7. Generate Fix Plans (if issues found), 8. Report (+1 more)

### Community 23 - "During Implementation"
Cohesion: 0.25
Nodes (8): After Each Change, Before Coding, Coding Principles, During Implementation, Goal-Driven, Simplicity, Surgical Changes, Test Integrity

### Community 24 - "Testing Infrastructure"
Cohesion: 0.25
Nodes (7): Gate Check Commands, Parallelism Assessment, Test Coverage Matrix, Test Execution, Test Frameworks, Test Organization, Testing Infrastructure

### Community 25 - "Process"
Cohesion: 0.29
Nodes (7): 1. Clarify Requirements, 2. Capture User Stories with Priorities, 3. Write Acceptance Criteria, Process, Specify, Template: `.specs/[feature]/spec.md`, Tips

### Community 26 - "default.json"
Cohesion: 0.29
Nodes (6): main, description, identifier, permissions, $schema, windows

### Community 27 - "graphify reference: query, path, explain"
Cohesion: 0.33
Nodes (5): For /graphify explain, For /graphify path, graphify reference: query, path, explain, Step 0 — Constrained query expansion (REQUIRED before traversal), Step 1 — Traversal

### Community 28 - "graphify reference: add a URL and watch a folder"
Cohesion: 0.50
Nodes (3): For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 29 - "graphify reference: commit hook and native CLAUDE.md integration"
Cohesion: 0.50
Nodes (3): For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration

### Community 30 - "graphify reference: incremental update and cluster-only"
Cohesion: 0.50
Nodes (3): For --cluster-only, For --update (incremental re-extraction), graphify reference: incremental update and cluster-only

### Community 31 - "Changelog"
Cohesion: 0.50
Nodes (3): Added, Changelog, [Unreleased]

## Knowledge Gaps
- **445 isolated node(s):** `name`, `private`, `version`, `packageManager`, `type` (+440 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **93 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Modelo de Ameaças — Secrets Storage v1` connect `Modelo de Ameaças — Secrets Storage v1` to `Implementation Decisions`?**
  _High betweenness centrality (0.013) - this node is a cross-community bridge._
- **What connects `name`, `private`, `version` to the rest of the system?**
  _445 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `SKILL.md` be split into smaller, more focused modules?**
  _Cohesion score 0.0507399577167019 - nodes in this community are weakly interconnected._
- **Should `README.md` be split into smaller, more focused modules?**
  _Cohesion score 0.06451612903225806 - nodes in this community are weakly interconnected._
- **Should `Implementation Decisions` be split into smaller, more focused modules?**
  _Cohesion score 0.06451612903225806 - nodes in this community are weakly interconnected._
- **Should `Modelo de Ameaças — Secrets Storage v1` be split into smaller, more focused modules?**
  _Cohesion score 0.06666666666666667 - nodes in this community are weakly interconnected._
- **Should `What You Must Do When Invoked` be split into smaller, more focused modules?**
  _Cohesion score 0.07407407407407407 - nodes in this community are weakly interconnected._