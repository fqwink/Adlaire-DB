# Document Index

## Status

- `docs/spec/project-charter.md`, `docs/spec/auth.md`, `docs/spec/api.md`, `docs/spec/testing.md`, and `docs/spec/phase-*.md` are the specification source-of-truth set.
- `docs/spec/project-charter.md` is the project charter and specification entry point.
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
| `docs/spec/project-charter.md` | Project charter and specification entry point | Source of truth |
| `docs/spec/auth.md` | Authentication and authorization specification | Source of truth |
| `docs/spec/api.md` | API contract specification | Source of truth |
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

## Future Split Candidate Areas

| Candidate Area | Current Source | Notes |
|----------------|----------------|-------|
| Product and compatibility responsibilities | `docs/spec/project-charter.md` sections 0-3 | Keep project authority clear |
| Runtime, auth, and API responsibilities | `docs/spec/project-charter.md` section 4, `docs/spec/auth.md`, and `docs/spec/api.md` | Keep runtime contracts linked |
| Data protection and operations responsibilities | `docs/spec/project-charter.md` sections 7-8 | Preserve operational safety |
| Implementation and phase responsibilities | `docs/spec/project-charter.md` sections 9-10 and `docs/spec/phase-*.md` | Phase detail split is active |
| Internalization and test responsibilities | `docs/spec/project-charter.md` section 11 and `docs/spec/testing.md` | Keep future testing and evidence gates clear |

## Non-Goals

- This file does not redefine the specification.
- This file does not replace phase completion criteria.
- This file does not create implementation requirements outside the specification source-of-truth set.
