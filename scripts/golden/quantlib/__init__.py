"""Native QuantLib fixture generators.

Two families share this package:

- **pricing** — ``finstack_quant.golden/1`` fixtures under
  ``valuations/tests/golden/data/pricing/quantlib/``
  (``mise run goldens-quantlib-generate`` / ``goldens-quantlib-check``)
- **attribution** — T0/T1 + expected attribution JSON under
  ``valuations/tests/data/quantlib_parity/``
  (``mise run goldens-quantlib-attribution-generate`` /
  ``goldens-quantlib-attribution-check``)

Schemas and scenarios differ on purpose; do not merge the fixture sets.
"""
