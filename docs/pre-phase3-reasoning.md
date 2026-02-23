# Project Considerations — Final Version (Phase 1)

> **Context:** This document consolidates the current direction of the project after reviewing both user perspectives (newcomer and mid-level Arch users) and refining the scope based on practical constraints and immediate development goals.
>
> **Goal of this document:** Define the product direction, scope, priorities, and user interaction model for the first phase — with a strong focus on clarity, segmentation, and a solid foundation.
>
> **Note:** This is intentionally **not** a deeply technical architecture document. It focuses on product/experience decisions and project structure at a higher level. A separate technical specification can be created next.

---

## 1. Project Direction (Phase 1)

The project will start as an **interactive CLI application** for Arch Linux system management and configuration orchestration.

### Why CLI first
- It is the most practical format for immediate testing during the current Arch machine restructuring.
- It allows fast iteration on real-world features (packages, services, config files, maintenance routines).
- It preserves the terminal-native philosophy expected by Arch users.
- It avoids the overhead of building a full TUI too early.

### What “interactive CLI” means here
The CLI should feel **guided and pleasant**, with:
- clear prompts
- confirmations with context
- readable summaries
- progress indicators
- structured output
- diff/preview before changes

It should feel “Bubble Tea-like” in usability, but without becoming a full-screen terminal interface at this stage.

---

## 2. Core Product Philosophy

This project is **not** just a wrapper around package manager commands.

It is a tool to make Arch Linux system management:

- **declarative**
- **safe**
- **transparent**
- **composable**
- **reproducible**
- **recoverable**

### The key idea
The user defines their desired system state in a **single controllable directory** (their source of truth), and the tool:

1. reads that declared state
2. compares it with the real system
3. shows a plan/diff
4. applies changes safely
5. records what happened

This keeps the user in control while reducing chaos.

---

## 3. Primary Focus Areas (Main Pillars)

To keep the project objective and well-segmented, the first phase should revolve around **three major pillars**:

---

### Pillar A — Canonical State Management (Source of Truth)

This is the foundation of everything.

The tool must support a clear, user-controlled source of truth that defines:
- what the system should look like
- what modules are enabled
- what differs per machine (desktop/laptop)
- what is shared vs host-specific

This is the backbone that enables:
- reproducibility
- drift detection
- multi-host management
- safe experimentation

---

### Pillar B — Module-Based System Management

The project should treat system customization as a set of **modules**, not a collection of random commands.

Examples of modules:
- shell config
- editor config
- window manager config
- package groups
- services
- maintenance routines

Each module should represent a clear unit of behavior/configuration and be manageable independently.

This aligns strongly with the expectations of intermediate users while still being approachable for newcomers.

---

### Pillar C — Safe Execution Workflow (Plan → Diff → Apply)

Every meaningful change should follow a safe and predictable lifecycle:

1. **Plan** (understand what will happen)
2. **Diff/Preview** (show what changes)
3. **Confirm** (with enough context)
4. **Apply**
5. **Verify / Record**

This is essential for trust and should be consistent across features like:
- package management
- service management
- config file management
- cleanup routines

---

## 4. Scope Prioritization (What Comes First)

The first phase should intentionally focus on a **small set of features** that validate the core design and can be tested empirically on a real Arch setup.

### Initial feature groups (high-value, practical)
- **System status**
- **Plan / diff / apply flow**
- **Module listing and module apply**
- **Package management workflows**
- **Service management workflows**
- **Basic config file management**
- **Drift detection (initial version)**
- **Update and cleanup routines**

These features are enough to prove the product direction while forcing the core to be well structured.

---

## 5. User Interaction Model

A major design decision for this project is how the user interacts with the application and where they place their system definition.

### User-owned source of truth
The user should control a dedicated directory that acts as the canonical definition of their system.

This directory should contain:
- main configuration entry point
- shared system definitions
- host-specific definitions
- modules
- profiles (optional, later)
- variables/overrides

### Why this matters
It creates a clean mental model:

> “I declare what I want in my config directory, and the tool shows me the plan and applies it.”

This makes the system:
- understandable
- portable
- Git-friendly
- easy to audit
- easy to evolve over time

---

## 6. Separation of Concerns (User State vs Runtime State)

The project should clearly separate:

### A) User-controlled configuration (source of truth)
This is the user’s declared system state:
- versionable
- editable
- reviewable
- portable between machines

### B) Tool runtime/state data
This includes:
- logs
- history of operations
- temporary backups
- lock files
- cached scan results (if any)

This separation prevents confusion and keeps the project aligned with standard Linux/XDG expectations.

---

## 7. Newcomer vs Intermediate User Positioning (Phase 1)

Although the CLI-first approach naturally aligns more with intermediate users, the project should still preserve the core values that matter to newcomers:

### Keep the beginner essence
- safety first
- readable output
- confirmations before risk
- no destructive surprises
- transparency (show what is happening)

### But optimize for intermediate workflows
Phase 1 should prioritize:
- modular workflows
- configuration ownership
- selective apply
- drift awareness
- multi-machine thinking
- experimentation with rollback paths

This gives the project a strong “power-user” foundation while staying accessible.

---

## 8. Product Boundaries (What We Are NOT Doing Yet)

To avoid scope explosion, Phase 1 should **not** try to solve everything.

### Not in scope yet
- Full-screen TUI
- Community module registry
- Advanced config merge UI
- Full secrets management system
- Remote host apply
- Advanced snapshot orchestration
- Deep configuration intelligence features

These can be added later, once the core model is validated.

---

## 9. What Success Looks Like in Phase 1

Phase 1 succeeds if the user can:

- define a basic canonical system state
- inspect what would change before applying
- apply changes safely and repeatedly
- manage a few modules (packages/services/configs)
- detect system drift at a basic level
- run useful maintenance commands through the same tool
- trust the tool enough to use it during real machine restructuring

### The practical success test
If the tool is already useful during a real Arch rebuild/reorganization, the foundation is correct.

---

## 10. Guiding Principles for Implementation (Non-Technical)

These principles should guide product decisions and UX decisions throughout the first phase.

### 1) Safety before convenience
No destructive operations without clarity and confirmation.

### 2) Transparency over magic
The tool should never feel like a black box.

### 3) Declarative first
The user should define state, not memorize commands.

### 4) Modular and segmented
Features should be organized by domain and remain independently useful.

### 5) Progressive adoption
Users should be able to adopt the tool gradually (not all-or-nothing).

### 6) Arch-native mindset
The tool orchestrates Arch/Linux tools; it does not replace them.

### 7) Escape hatch always available
If the user stops using the tool, their system still works and their files remain standard.

---

## 11. Next Step (Planned Follow-up Document)

The next document should be more technical and define:

- canonical config directory layout (exact structure)
- data contracts (desired state, plan, diff, drift report)
- command tree (v0.1)
- module contract/spec
- runtime directory structure
- plan/apply/rollback lifecycle details

This current document exists to ensure the project direction is aligned before locking technical details.

---

## Final Summary

This project will begin as an **interactive, declarative CLI for Arch Linux system management**, focused on:

- **canonical configuration ownership**
- **module-based organization**
- **safe and transparent change execution**
- **practical workflows for real Arch usage**

The first phase is intentionally constrained to validate the foundation with real usage, while setting up a structure that can evolve later into a more advanced system (and eventually a richer interface) without rethinking the core model.
