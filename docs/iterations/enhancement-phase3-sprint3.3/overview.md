# Sprint 3.3 -- Execution Lifecycle Completion

- **Type**: ENHANCEMENT
- **Request**: Implement all 5 tasks from Phase 3 Sprint 3.3 (Lifecycle Completion)
- **Agent Chain**: analyst -> architect -> developer -> tester -> reviewer
- **Created**: 2026-02-23

## Scope
Sprint 3.3 completes the execution lifecycle by adding hook execution with behavior policies (Always/Once/Ask/Skip), operation history via `iron history`, automatic dotfiles directory mirroring (`dotfiles_sync`), module dependency resolution in apply ordering, and optionally the `iron config` CLI namespace. This builds on Sprint 3.2's declarative convergence (managed tracking, removal actions, risk levels) to deliver a fully operational apply pipeline with hooks, history tracking, and smart dotfile discovery.
