"""Product-independent interest-rate models.

Submodules
----------
- :mod:`finstack_quant.models.rates.dtsm` — Diebold-Li dynamic Nelson-Siegel
  and yield-curve PCA.
- :mod:`finstack_quant.models.rates.hull_white` — Hull-White one-factor
  parameters and closed-form kernels.

Examples
--------
>>> from finstack_quant.models.rates import dtsm
>>> len(dtsm.nelson_siegel_yields(0.7308, (0.03, -0.01, 0.005), [1.0, 5.0]))
2
"""

from finstack_quant.models.rates import dtsm as dtsm
from finstack_quant.models.rates import hull_white as hull_white

__all__ = ["dtsm", "hull_white"]
