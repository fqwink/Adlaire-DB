# Document Index

## Status

- `docs/spec/adlaire-db-spec.md` and `docs/spec/phase-*.md` are the specification source-of-truth set.
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
| `docs/spec/adlaire-db-spec.md` | Parent specification and implementation contract | Source of truth |
| `docs/spec/phase-01.md` - `docs/spec/phase-19.md` | Phase-specific implementation contracts | Source of truth |
| `AGENTS.md` | Repository work rules for agents | Work rule |
| `docs/DOCUMENT_INDEX.md` | Documentation index and split-management reference | Supporting document |

## Split Management Rules

1. Phase files under `docs/spec/` are normative specification files.
2. Do not move additional normative requirements unless the source-of-truth rule is updated first.
3. Prefer stable section IDs, Contract IDs, Phase IDs, API paths, and artifact paths over line-number references.
4. Any future split must preserve implementation clarity by phase, API surface, persistence contract, error contract, and verification gate.
5. A future split must include a migration map from the original section to the new document path.

## Future Split Candidate Areas

| Candidate Area | Current Source | Notes |
|----------------|----------------|-------|
| Overview and scope | `docs/spec/adlaire-db-spec.md` sections 0-2 | Keep project authority clear |
| Architecture and configuration | `docs/spec/adlaire-db-spec.md` sections 3-5 | Keep runtime contracts linked |
| API and error contracts | `docs/spec/adlaire-db-spec.md` sections 6-7 | Preserve request/response exactness |
| Lifecycle and phase contracts | `docs/spec/adlaire-db-spec.md` section 8 and `docs/spec/phase-*.md` | Phase detail split is active |
| Security, deployment, logs, WAL | `docs/spec/adlaire-db-spec.md` sections 10-13 | Operational reference candidate |
| Glossary and future policy | Appendices and Phase 19 policy | Supporting reference candidate |

## Non-Goals

- This file does not redefine the specification.
- This file does not replace phase completion criteria.
- This file does not create implementation requirements outside the specification source-of-truth set.
