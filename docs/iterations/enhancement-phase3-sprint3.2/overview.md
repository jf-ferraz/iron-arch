# Phase 3 Sprint 3.2 -- Full Declarative Convergence

- **Type**: ENHANCEMENT
- **Request**: Start Phase 3 Sprint 3.2 implementation
- **Agent Chain**: analyst -> architect -> developer -> tester -> reviewer
- **Created**: 2026-02-23

## Scope

Implement Sprint 3.2 of Phase 3 (Declarative Convergence): managed resource tracking in state, template variable rendering in the apply pipeline, file copy deployment mode, package removal actions, service disable actions, symlink/module removal actions, risk level classification on all ApplyAction variants, and the `iron apply --confirm` UX flow with granular prune flags. This sprint transforms Iron from an additive-only system to one that can fully converge -- including removing resources no longer declared.
