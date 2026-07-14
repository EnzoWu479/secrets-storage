# Graph Report - .  (2026-07-13)

## Corpus Check
- 1 files · ~41,282 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 96 nodes · 94 edges · 12 communities (11 shown, 1 thin omitted)
- Extraction: 89% EXTRACTED · 11% INFERRED · 0% AMBIGUOUS · INFERRED: 10 edges (avg confidence: 0.84)
- Token cost: 0 input · 0 output

## Community Hubs (Navigation)
- Graphify Pipeline
- Secure Vault Requirements
- README Product Overview
- Spec-Driven Workflow
- Codebase Quality Mapping
- Execution and Validation
- Research and Feature Design
- Graph Query and Memory
- Security Foundation and State
- Project Stack and Initialization
- Quick Tasks and Handoff
- Graph Exports and MCP

## God Nodes (most connected - your core abstractions)
1. `Graphify Pipeline` - 9 edges
2. `Secure Vault v1 Specification` - 7 edges
3. `Four Adaptive Phases` - 6 edges
4. `Cofre criptografado` - 6 edges
5. `Semantic Extraction` - 5 edges
6. `Incremental Extraction` - 5 edges
7. `Feature Design` - 5 edges
8. `Product Roadmap` - 5 edges
9. `Structural Extraction` - 4 edges
10. `Domain-Prompted Transcription` - 4 edges

## Surprising Connections (you probably didn't know these)
- `Roadmap Creation` --implements--> `Product Roadmap`  [EXTRACTED]
  .agents/skills/tlc-spec-driven/references/roadmap.md → .specs/project/ROADMAP.md
- `Mandatory Graphify Update Rule` --conceptually_related_to--> `Project State`  [INFERRED]
  AGENTS.md → .specs/project/STATE.md
- `Living Risk Documentation` --references--> `Feature Design`  [EXTRACTED]
  .agents/skills/tlc-spec-driven/references/concerns.md → .agents/skills/tlc-spec-driven/references/design.md
- `Project Initialization` --implements--> `Secrets Storage Project`  [EXTRACTED]
  .agents/skills/tlc-spec-driven/references/project-init.md → .specs/project/PROJECT.md
- `Contribution Workflow` --references--> `Commit Version and Release Policy`  [EXTRACTED]
  CONTRIBUTING.md → .specs/project/RELEASES.md

## Hyperedges (group relationships)
- **Princípios de segurança do Secrets Storage** — readme_local_first, readme_zero_knowledge, readme_session_security, readme_no_silent_loss, readme_authenticated_updates, readme_reviewable_cryptography [EXTRACTED 1.00]
- **Graphify Extraction Pipeline** — _agents_skills_graphify_skill_structural_extraction, _agents_skills_graphify_skill_semantic_extraction, _agents_skills_graphify_skill_community_detection [EXTRACTED 1.00]
- **TLC Adaptive Delivery Workflow** — _agents_skills_tlc_spec_driven_readme_four_adaptive_phases, _agents_skills_tlc_spec_driven_skill_auto_sizing, _agents_skills_tlc_spec_driven_readme_atomic_commits [EXTRACTED 1.00]
- **Evidence-Driven Feature Planning** — _agents_skills_tlc_spec_driven_references_brownfield_mapping_brownfield_mapping, _agents_skills_tlc_spec_driven_references_concerns_evidence_backed_concerns, _agents_skills_tlc_spec_driven_references_design_feature_design [INFERRED 0.85]
- **TLC Spec-Driven Delivery Pipeline** — agents_skills_tlc_spec_driven_references_specify_specify, agents_skills_tlc_spec_driven_references_tasks_atomic_tasks, agents_skills_tlc_spec_driven_references_implement_execute, agents_skills_tlc_spec_driven_references_validate_validate_verify [EXTRACTED 1.00]
- **Secure Vault v1 Security Delivery** — specs_features_secure_vault_spec_secure_vault_v1, specs_features_secure_vault_threat_model_threat_model, specs_project_roadmap_product_roadmap, specs_project_releases_release_policy [INFERRED 0.85]

## Communities (12 total, 1 thin omitted)

### Community 0 - "Graphify Pipeline"
Cohesion: 0.10
Nodes (22): Debounced Rebuild, Folder Watcher, URL Ingestion, Token Reduction Benchmark, Extraction Confidence Rubric, Deterministic Node IDs, Graph Hyperedges, Semantic Similarity Edges (+14 more)

### Community 1 - "Secure Vault Requirements"
Cohesion: 0.12
Nodes (19): Roadmap Creation, Contribution Workflow, Secure Vault Implementation Decisions, Secure Vault v1 Context, Authenticated Updates, Independent Security Sessions, Secret Management, Secure Vault v1 Specification (+11 more)

### Community 2 - "README Product Overview"
Cohesion: 0.20
Nodes (10): Atualizações autenticadas, Cofre criptografado, Local-first, M0 — Fundação de segurança, Sem perda silenciosa, Criptografia revisável, Secrets Storage, Segurança por sessão (+2 more)

### Community 3 - "Spec-Driven Workflow"
Cohesion: 0.29
Nodes (8): Atomic Conventional Commits, Four Adaptive Phases, Quick Mode, TLC Spec-Driven Development, Surgical Changes, Context Budget Zones, Workflow Auto-Sizing, Sub-Agent Delegation

### Community 4 - "Codebase Quality Mapping"
Cohesion: 0.25
Nodes (8): Brownfield Mapping, Seven Codebase Documents, Testing Gate Matrix, Graceful Search Degradation, Structural Code Search, Test Integrity, Evidence-Backed Codebase Concerns, Living Risk Documentation

### Community 5 - "Execution and Validation"
Cohesion: 0.29
Nodes (7): Execute, TDD Gate and Atomic Commit, Requirement Traceability, Specify, Atomic Tasks, Test Coverage Matrix, Validate and Verify

### Community 6 - "Research and Feature Design"
Cohesion: 0.40
Nodes (6): Code Reuse Analysis, Feature Design, Gray Area Discussion, Locked Context Decisions, Feature Scope Guardrail, Knowledge Verification Chain

### Community 7 - "Graph Query and Memory"
Cohesion: 0.50
Nodes (4): Constrained Query Expansion, Graph Traversal, Shortest Concept Path, Graph Work Memory

### Community 8 - "Security Foundation and State"
Cohesion: 0.50
Nodes (4): Mandatory Graphify Update Rule, Security-Blocking Prototypes, M0 Security Foundation, Project State

### Community 9 - "Project Stack and Initialization"
Cohesion: 0.67
Nodes (3): Project Initialization, Secrets Storage Project, Tauri 2 Rust Vite Tailwind Stack

### Community 10 - "Quick Tasks and Handoff"
Cohesion: 0.67
Nodes (3): Quick Mode, Session Handoff, State Management

## Knowledge Gaps
- **31 isolated node(s):** `URL Ingestion`, `Graph Export Formats`, `MCP Graph Server`, `Token Reduction Benchmark`, `Semantic Similarity Edges` (+26 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **1 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Product Roadmap` connect `Secure Vault Requirements` to `Security Foundation and State`?**
  _High betweenness centrality (0.023) - this node is a cross-community bridge._
- **What connects `URL Ingestion`, `Graph Export Formats`, `MCP Graph Server` to the rest of the system?**
  _31 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Graphify Pipeline` be split into smaller, more focused modules?**
  _Cohesion score 0.1038961038961039 - nodes in this community are weakly interconnected._
- **Should `Secure Vault Requirements` be split into smaller, more focused modules?**
  _Cohesion score 0.12280701754385964 - nodes in this community are weakly interconnected._