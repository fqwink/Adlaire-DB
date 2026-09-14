# Document Index

## Status

- `docs/PROJECT_CHARTER.md` is the project charter source of truth and records the overview, direction, policies, specification entry point, and specification-driven development direction.
- `docs/spec/compatibility-scope.md`, `docs/spec/runtime.md`, `docs/spec/auth.md`, `docs/spec/api.md`, `docs/spec/operations.md`, `docs/spec/implementation-control.md`, `docs/spec/phase-contracts.md`, `docs/spec/implementation-details.md`, `docs/spec/internalization.md`, `docs/spec/testing.md`, and `docs/spec/phase-*.md` are the specification source-of-truth set.
- Policies are recorded in `docs/PROJECT_CHARTER.md` and are not specification source of truth.
- Phase 1 through Phase 19 are split under `docs/spec/`.
- Files outside the specification source-of-truth set are supporting documents.
- If this index conflicts with the specification source-of-truth set, the specification source-of-truth set takes precedence.

## Purpose

This index records the current documentation layout and the authority of each
document. It also provides a stable place to manage future split candidates and
reference rules without changing the specification source-of-truth set by
accident.

## Current Documents

| Path | Role | Authority |
|------|------|-----------|
| `docs/PROJECT_CHARTER.md` | Project charter, overview, direction, policies, and specification entry point | Project charter source of truth |
| `docs/spec/compatibility-scope.md` | Compatibility, boundary, and feature-scope specification | Source of truth |
| `docs/spec/runtime.md` | Runtime architecture, data layout, CLI, config, startup, shutdown, and recovery specification | Source of truth |
| `docs/spec/auth.md` | Authentication and authorization specification | Source of truth |
| `docs/spec/api.md` | API contract specification | Source of truth |
| `docs/spec/operations.md` | Data protection, WAL, security, logging, and operations specification | Source of truth |
| `docs/spec/implementation-control.md` | Cross-phase implementation control, done gates, evidence, and review specification | Source of truth |
| `docs/spec/phase-contracts.md` | Phase gates, endpoint, persistence, error, test, security, and phase-entry contracts | Source of truth |
| `docs/spec/implementation-details.md` | Shared module layout, type definitions, and implementation details | Source of truth |
| `docs/spec/internalization.md` | Internalization implementation specification | Source of truth |
| `docs/spec/testing.md` | Testing specification | Source of truth |
| `docs/spec/phase-01.md` - `docs/spec/phase-19.md` | Phase-specific implementation contracts | Source of truth |
| `AGENTS.md` | Repository work rules for agents | Work rule |
| `docs/DOCUMENT_INDEX.md` | Documentation index and split-management reference | Supporting document |

## Split Management Rules

1. Phase files under `docs/spec/` are normative specification files.
2. Do not move additional normative requirements unless the source-of-truth rule is updated first.
3. Prefer stable section IDs, Contract IDs, Phase IDs, API paths, and artifact paths over line-number references.
4. Any future split must preserve implementation clarity by phase, API surface, persistence contract, error contract, and verification gate.
5. A future split must include a migration map from the original section to the new document path.

## Current Responsibility Map

| Area | Current Source | Notes |
|----------------|----------------|-------|
| Product direction responsibilities | `docs/PROJECT_CHARTER.md` sections 0-1 and 11 | Keep project authority clear |
| Compatibility and scope responsibilities | `docs/spec/compatibility-scope.md` | Keep boundary decisions small and easy to inspect |
| Runtime responsibilities | `docs/spec/runtime.md`, `docs/spec/auth.md`, and `docs/spec/api.md` | Keep runtime contracts linked |
| Data protection and operations responsibilities | `docs/spec/operations.md` | Preserve operational safety |
| Implementation control responsibilities | `docs/spec/implementation-control.md` | Keep common gates and review rules separate from runtime details |
| Phase responsibility tables and phase entry | `docs/spec/phase-contracts.md` and `docs/spec/phase-*.md` | Phase detail split is active |
| Shared implementation details | `docs/spec/implementation-details.md` | Keep type/module reference material out of core |
| Internalization and test responsibilities | `docs/spec/internalization.md` and `docs/spec/testing.md` | Keep internal replacement contracts and evidence gates clear |

## Non-Goals

- This file does not redefine the specification.
- This file does not replace phase completion criteria.
- This file does not create implementation requirements outside the specification source-of-truth set.
