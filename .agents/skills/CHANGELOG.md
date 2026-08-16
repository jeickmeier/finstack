# Skills Refactor Changelog

## Active Catalog

Active skill count: 13. See [`README.md`](README.md) for what each one is for.

Active skills:

- `finstack-binding-parity-reviewer`
- `finstack-consistency-reviewer`
- `finstack-documentation-maintainer`
- `finstack-simplify`
- `finstack-performance-reviewer`
- `finstack-production-release-prep`
- `finstack-quality-gate-triage`
- `finstack-quant-finance-review`
- `finstack-refactor`
- `finstack-rust-architecture-review`
- `finstack-rust-library-architecture-docs`
- `finstack-senior-code-review`
- `finstack-stepwise-quant-audit`

## Renamed Or Merged

- `python-binding-reviewer` -> `finstack-binding-parity-reviewer`
- `bug_hunting` -> `finstack-quality-gate-triage`
- `documentation-cleanup` + `documentation-reviewer` -> `finstack-documentation-maintainer`

## Retired Or Demoted

Moved to `.agents/retired-skills/`, then deleted outright in `b81bd85d8`. That
directory no longer exists; recover the files from git history if needed.

- `code-simplifier`
- `simplicity-auditor`
- `dead-code-removal`

Their active responsibilities now live in `finstack-refactor`, `finstack-simplify`, `finstack-quality-gate-triage`, and `finstack-production-release-prep`.

## Coverage Added

- Binding parity across Rust, PyO3, WASM, stubs, exports, parity contract, tests, and examples.
- Numerical regression and market-convention modes in `finstack-quant-finance-review`.
- Benchmark regression mode in `finstack-performance-reviewer`.
- Quality-gate and CI/pre-commit triage in `finstack-quality-gate-triage`.
- Combined API documentation and stale-doc cleanup in `finstack-documentation-maintainer`.
- Golden output examples and eval prompts for each active skill.
