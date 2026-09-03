"""Product-independent interest-rate models.

Examples:
--------
>>> from finstack_quant.models.rates import dtsm
>>> len(dtsm.nelson_siegel_yields(0.7308, (0.03, -0.01, 0.005), [1.0, 5.0]))
2
"""

from finstack_quant.models.rates import dtsm as dtsm, hull_white as hull_white

__all__: list[str] = ["dtsm", "hull_white"]
