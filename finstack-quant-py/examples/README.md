# finstack-quant-py examples

Runnable material for the Python bindings, in three forms: teaching notebooks,
standalone scripts, and the reports some scripts produce.

Everything here runs against the compiled extension. Build it first:

```bash
mise run python-build
```

## Layout

```
examples/
  notebooks/   layered Jupyter curriculum (01_foundations ... 09_reporting)
  scripts/     standalone .py examples, each runnable on its own
  reports/     committed Markdown output produced by a script
```

## `notebooks/`

A nine-level curriculum from core types through pricing, analytics, statement
modeling, portfolio, scenarios, advanced quant, a capstone, and reporting, plus
an example-only `_shared/` helper package.

It has its own documentation — see
[`notebooks/README.md`](notebooks/README.md) for the reading order, the
per-notebook topic tables, the `_shared` helper API, and the data-file
conventions. Not repeated here.

Batch execution:

```bash
mise run python-examples                                   # runs every notebook
uv run python finstack-quant-py/examples/notebooks/run_all_notebooks.py --help
```

`run_all_notebooks.py` executes each notebook with `nbclient` and prints a
pass/fail summary. Flags: `--directory` (a subdirectory or a single `.ipynb`),
`--timeout` (per notebook, default 300s), `--verbose`, `--fail-fast`, and
`--save-outputs` to write executed notebooks back with fresh outputs.

## `scripts/`

Standalone examples. Each is a plain module with a `main()`, runnable directly
from the repository root, and each carries its run command in its module
docstring.

| Script | What it does |
|--------|--------------|
| `reporting_instrument_tearsheet.py` | Prices a fixed-rate bond against a `MarketContext` built from a standard-tenor `DiscountCurve`, pulls its cashflows, and renders an instrument tear sheet through `finstack_quant.reporting` |
| `reporting_performance_tearsheet.py` | Builds a `Performance` from a returns `DataFrame` and renders a performance tear sheet, writing `performance_tearsheet.html` to the current directory |
| `statements_test_a.py` | Runs the Test A financial-statements readiness checks: model round-trip, schema validation of `FinancialModelSpec` and `NormalizationConfig`, Rust evaluation, and normalization. Optionally writes a signed Markdown report |
| `run_all_scripts.py` | Discovers and executes the other scripts, skipping itself and `.ipynb_checkpoints`, and prints a per-script pass/fail line |

Run one:

```bash
uv run python finstack-quant-py/examples/scripts/reporting_performance_tearsheet.py
```

Run them all:

```bash
uv run python finstack-quant-py/examples/scripts/run_all_scripts.py
```

`run_all_scripts.py` runs each script with `cwd` set to the script's own
directory and a 120-second timeout, so anything a script writes relative to the
current directory lands in `scripts/`.

### `statements_test_a.py` flags

```bash
# mechanical checks only; does not touch the committed report
uv run python finstack-quant-py/examples/scripts/statements_test_a.py

# treat the conditional readiness gate as a failure
uv run python finstack-quant-py/examples/scripts/statements_test_a.py --strict-readiness

# regenerate the named report (both --signer and --signed-on are required)
uv run python finstack-quant-py/examples/scripts/statements_test_a.py \
    --write-report --signer Jon --signed-on 2026-08-02
```

`--report-path` overrides the destination. The write is atomic: content goes to
a temp file in the destination directory, is fsynced, mode-fixed to 0644 on the
open descriptor, and only then renamed over the target — so a failed run cannot
leave a truncated report behind.

## `reports/`

Committed output, not input.

| File | Produced by |
|------|-------------|
| `statements_test_a_report.md` | `scripts/statements_test_a.py --write-report` |

Do not hand-edit it; regenerate it with the command above so the signer and
date fields stay consistent with the checks that actually ran.

## What CI enforces about this directory

The examples are tested, not merely shipped. From
[`../tests/`](../tests/README.md):

- `test_notebook_hygiene.py` — AST scan of every notebook for failure-hiding
  constructs around `finstack_quant` calls. A cell that demonstrates a failure
  on purpose must carry the `intentional-negative` tag.
- `test_run_all_notebooks.py` — unit-tests the notebook runner and executes a
  few notebooks end to end.
- `test_run_all_scripts.py` — unit-tests `run_all_scripts.py` and pins the
  symbols the scripts import from `finstack_quant.{analytics, portfolio,
  statements_analytics, valuations}`, so a binding rename breaks the test
  rather than the example.
- `test_notebook_instrument_fixtures.py` — every instrument factory in
  `notebooks/_shared/instrument_fixtures.py` must still deserialize.
- `test_statements_test_a_example.py` — drives `statements_test_a.py` as a
  subprocess and asserts the report write is atomic and that a default run
  leaves `reports/statements_test_a_report.md` untouched.

Consequence: if you rename a binding, update the examples in the same change.

## See also

- [`notebooks/README.md`](notebooks/README.md) — the notebook curriculum
- [`../README.md`](../README.md) — the Python package overview
- [`../tests/README.md`](../tests/README.md) — the test suite that guards these
  examples
