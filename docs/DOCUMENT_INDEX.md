# Document Index

## Status

- `docs/adlaire-db-spec.md` remains the single source of truth.
- The specification is not split at this stage.
- Files under `docs/` are supporting documents only.
- If this index conflicts with `docs/adlaire-db-spec.md`, `docs/adlaire-db-spec.md` takes precedence.

## Purpose

This index prepares for future documentation organization without changing the
current specification authority. It provides a stable place to collect document
roles, future split candidates, and reference rules before any actual split is
performed.

## Current Documents

| Path | Role | Authority |
|------|------|-----------|
| `docs/adlaire-db-spec.md` | Main specification and implementation contract | Source of truth |
| `AGENTS.md` | Repository work rules for agents | Work rule |
| `docs/DOCUMENT_INDEX.md` | Documentation index and future split preparation | Supporting document |

## Future Split Preparation Rules

1. Do not split `docs/adlaire-db-spec.md` until the split plan is explicitly approved.
2. Do not move normative requirements into `docs/` unless the source-of-truth rule is updated first.
3. Prefer stable section IDs, Contract IDs, Phase IDs, API paths, and artifact paths over line-number references.
4. Any future split must preserve implementation clarity by phase, API surface, persistence contract, error contract, and verification gate.
5. A future split must include a migration map from the original section to the new document path.

## Future Split Candidate Areas

| Candidate Area | Current Source | Notes |
|----------------|----------------|-------|
| Overview and scope | `docs/adlaire-db-spec.md` sections 0-2 | Keep project authority clear |
| Architecture and configuration | `docs/adlaire-db-spec.md` sections 3-5 | Keep runtime contracts linked |
| API and error contracts | `docs/adlaire-db-spec.md` sections 6-7 | Preserve request/response exactness |
| Lifecycle and phase contracts | `docs/adlaire-db-spec.md` sections 8-9 | Highest risk area; split last |
| Security, deployment, logs, WAL | `docs/adlaire-db-spec.md` sections 10-13 | Operational reference candidate |
| Glossary and future policy | Appendices and Phase 19 policy | Supporting reference candidate |

## Non-Goals

- This file does not redefine the specification.
- This file does not approve splitting the specification.
- This file does not replace phase completion criteria.
- This file does not create implementation requirements outside `docs/adlaire-db-spec.md`.
