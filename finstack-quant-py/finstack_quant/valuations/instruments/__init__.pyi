"""
Python bindings for the corresponding finstack-quant Rust API.

Every typed instrument ``from_json`` classmethod accepts either canonical bare
``{"type": ..., "spec": ...}`` JSON or a versioned
``{"schema": "finstack_quant.instrument/1", "instrument": ...}`` envelope.
Inputs larger than 16 MiB raise ``ValueError`` before parsing.

Examples
--------
>>> from finstack_quant.valuations.instruments import list_models, list_models_grouped
>>> ("discounting" in list_models(), "bond" in list_models_grouped())
(True, True)

"""

from __future__ import annotations

import datetime
from typing import Literal

import pandas as pd

from finstack_quant.core.currency import Currency
from finstack_quant.core.dates import DayCount, Tenor
from finstack_quant.core.market_data import MarketContext
from finstack_quant.core.money import Money
from finstack_quant.core.types import Bps, Rate
from finstack_quant.valuations import ValuationResult
from finstack_quant.valuations.models.credit import (
    DynamicRecoverySpec,
    EndogenousHazardSpec,
    MertonModel,
    ToggleExerciseModel,
)

__all__ = [
    "AssetPool",
    "BarrierCrossing",
    "Bond",
    "CDSIndex",
    "CDSIndexBuilder",
    "CDSTranche",
    "CDSTrancheBuilder",
    "CapFloor",
    "CapFloorBuilder",
    "ConvertibleBond",
    "ConvertibleBondBuilder",
    "CreditDefaultSwap",
    "CreditDefaultSwapBuilder",
    "EquityOption",
    "EquityOptionBuilder",
    "FixedLegSpec",
    "FloatLegSpec",
    "FxForward",
    "FxForwardBuilder",
    "FxOption",
    "FxOptionBuilder",
    "InterestRateSwap",
    "InterestRateSwapBuilder",
    "MertonMcConfig",
    "MertonMcResult",
    "OasResult",
    "PathStatistics",
    "PikMode",
    "PikSchedule",
    "PremiumLegSpec",
    "ProtectionLegSpec",
    "RepLine",
    "ScenarioTable",
    "StructuredCredit",
    "StructuredCreditBuilder",
    "Swaption",
    "SwaptionBuilder",
    "TermLoan",
    "Tranche",
    "TrancheBuilder",
    "TrancheMetrics",
    "TrancheStructure",
    "bond_from_cashflows_json",
    "instrument_cashflows_json",
    "list_models",
    "list_models_grouped",
    "list_standard_metrics",
    "list_standard_metrics_grouped",
    "price_instrument",
    "price_instrument_with_metrics",
    "pretty_instrument_json",
    "structured_credit_tranche_breakeven_cdr",
    "structured_credit_tranche_discount_margin",
    "structured_credit_tranche_metrics",
    "structured_credit_tranche_oas",
    "structured_credit_tranche_scenario_table",
    "validate_instrument_json",
    "validate_typed_instrument_json",
]

class Bond:
    """
    Typed wrapper for the canonical Rust ``Bond`` instrument.

    Construct via :meth:`Bond.fixed`, :meth:`Bond.floating`, or
    :meth:`Bond.from_json`; serialize with :meth:`Bond.to_json`. Instances
    are accepted directly by :func:`price_instrument`,
    :func:`price_instrument_with_metrics`, and
    :func:`instrument_cashflows_json`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import Bond
    >>> bond = Bond.fixed(
    ...     "BOND-1",
    ...     Money(1_000_000.0, Currency("USD")),
    ...     Rate(0.05),
    ...     datetime.date(2024, 1, 1),
    ...     datetime.date(2034, 1, 1),
    ...     "USD-OIS",
    ... )
    >>> bond.id
    'BOND-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def fixed(
        id: str,
        notional: Money,
        coupon_rate: Rate,
        issue: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> Bond:
        """
        Create a standard fixed-rate bond (semi-annual, 30/360, T+2).

        Mirrors Rust ``Bond::fixed``.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        notional : Money
            Principal amount of the bond.
        coupon_rate : Rate
            Annual coupon rate.
        issue : datetime.date
            Issue date.
        maturity : datetime.date
            Maturity date.
        discount_curve_id : str
            Discount curve identifier used for pricing.

        Returns
        -------
        Bond
            A validated fixed-rate bond.

        Raises
        ------
        ValueError
            If validation fails (e.g. maturity not after issue).

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Rate
        >>> from finstack_quant.valuations.instruments import Bond
        >>> bond = Bond.fixed(
        ...     "B",
        ...     Money(1000.0, Currency("USD")),
        ...     Rate(0.05),
        ...     datetime.date(2024, 1, 1),
        ...     datetime.date(2029, 1, 1),
        ...     "USD-OIS",
        ... )
        >>> bond.id
        'B'

        """
        ...

    @staticmethod
    def floating(
        id: str,
        notional: Money,
        index_id: str,
        margin_bp: Bps,
        issue: datetime.date,
        maturity: datetime.date,
        frequency: Tenor,
        day_count: DayCount,
        discount_curve_id: str,
    ) -> Bond:
        """
        Create a floating-rate bond (FRN) linked to a forward index.

        Mirrors Rust ``Bond::floating``.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        notional : Money
            Principal amount of the bond.
        index_id : str
            Forward curve identifier (e.g. ``"USD-SOFR-3M"``).
        margin_bp : Bps
            Spread over the index in basis points.
        issue : datetime.date
            Issue date.
        maturity : datetime.date
            Maturity date.
        frequency : Tenor
            Payment frequency (e.g. ``Tenor.quarterly()``).
        day_count : DayCount
            Day count convention (e.g. ``DayCount.act360()``).
        discount_curve_id : str
            Discount curve identifier used for pricing.

        Returns
        -------
        Bond
            A validated floating-rate note.

        Raises
        ------
        ValueError
            If validation fails.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import DayCount, Tenor
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Bps
        >>> from finstack_quant.valuations.instruments import Bond
        >>> bond = Bond.floating(
        ...     "FRN",
        ...     Money(1000.0, Currency("USD")),
        ...     "USD-SOFR-3M",
        ...     Bps(125.0),
        ...     datetime.date(2024, 1, 1),
        ...     datetime.date(2029, 1, 1),
        ...     Tenor.quarterly(),
        ...     DayCount.ACT_360,
        ...     "USD-OIS",
        ... )
        >>> bond.id
        'FRN'

        """
        ...

    @classmethod
    def from_json(cls, json: str) -> Bond:
        """
        Deserialize a validated bond from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"bond"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        Bond
            The validated bond.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails bond validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Rate
        >>> from finstack_quant.valuations.instruments import Bond
        >>> bond = Bond.fixed(
        ...     "B",
        ...     Money(1000.0, Currency("USD")),
        ...     Rate(0.05),
        ...     datetime.date(2024, 1, 1),
        ...     datetime.date(2029, 1, 1),
        ...     "USD-OIS",
        ... )
        >>> Bond.from_json(bond.to_json()).id
        'B'

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`Bond.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def price_merton_mc(
        self,
        config: MertonMcConfig,
        discount_rate: float,
        as_of: datetime.date | str,
    ) -> MertonMcResult:
        """
        Price this bond with the Merton Monte Carlo structural credit engine.

        Uses geometric Brownian motion asset dynamics only. Floating-rate and
        amortizing cashflow specs raise ``ValueError``. When the config's PIK
        schedule is the default uniform cash mode, the bond's ``CouponType``
        overrides the schedule; otherwise the config schedule takes precedence.

        Parameters
        ----------
        config : MertonMcConfig
            Merton MC simulation configuration including the structural model.
        discount_rate : float
            Flat continuously compounded risk-free rate as a decimal used to
            discount simulated cashflows.
        as_of : datetime.date or str
            Valuation date.

        Returns
        -------
        MertonMcResult
            Monte Carlo pricing result with clean/dirty prices and path stats.

        Raises
        ------
        ValueError
            If ``as_of`` is invalid, the bond has floating or amortizing
            cashflows, or simulation parameters fail validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.core.types import Rate
        >>> from finstack_quant.valuations.instruments import (
        ...     Bond,
        ...     MertonMcConfig,
        ...     PikMode,
        ...     PikSchedule,
        ... )
        >>> from finstack_quant.valuations.models.credit import MertonModel
        >>> bond = Bond.fixed(
        ...     "BOND-MC",
        ...     Money(1_000_000.0, Currency("USD")),
        ...     Rate(0.08),
        ...     datetime.date(2024, 1, 1),
        ...     datetime.date(2029, 1, 1),
        ...     "USD-OIS",
        ... )
        >>> merton = MertonModel(100.0, 0.25, 80.0, 0.04)
        >>> config = MertonMcConfig(merton).pik_schedule(PikSchedule.uniform(PikMode.pik())).num_paths(256).seed(42)
        >>> result = bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 1))
        >>> result.num_paths
        256

        """
        ...

class BarrierCrossing:
    """
    Barrier-crossing detection policy for first-passage default simulation.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import BarrierCrossing
    >>> BarrierCrossing.brownian_bridge().to_json()
    '"brownian_bridge"'
    """

    @staticmethod
    def discrete() -> BarrierCrossing:
        """
        Discrete monitoring at simulation grid points.

        Returns
        -------
        BarrierCrossing
            Discrete policy: default is declared only when an asset value
            sampled on the simulation grid sits below the barrier. Fast, but it
            misses excursions between steps and so understates default risk on
            coarse grids. The default for terminal-barrier models.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import BarrierCrossing
        >>> BarrierCrossing.discrete().to_json()
        '"discrete"'
        """
        ...

    @staticmethod
    def brownian_bridge() -> BarrierCrossing:
        """
        Brownian-bridge correction for continuous monitoring.

        Returns
        -------
        BarrierCrossing
            Brownian-bridge policy: between two surviving grid points it draws
            against the analytic crossing probability, which removes the
            discretisation bias. The default for first-passage models and the
            more expensive of the two.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import BarrierCrossing
        >>> BarrierCrossing.brownian_bridge().to_json()
        '"brownian_bridge"'
        """
        ...

    @staticmethod
    def from_json(json: str) -> BarrierCrossing:
        """
        Deserialize from canonical JSON.

        Parameters
        ----------
        json : str
            JSON-encoded barrier-crossing policy.

        Returns
        -------
        BarrierCrossing
            The decoded policy.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for a barrier-crossing policy.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import BarrierCrossing
        >>> BarrierCrossing.from_json('"discrete"').to_json()
        '"discrete"'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            JSON-encoded barrier-crossing policy.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class MertonMcConfig:
    """
    Configuration for Merton Monte Carlo PIK bond pricing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import MertonMcConfig, PikMode, PikSchedule
    >>> from finstack_quant.valuations.models.credit import MertonModel
    >>> config = MertonMcConfig(MertonModel(100.0, 0.25, 80.0, 0.04)).num_paths(1000)
    >>> isinstance(config.seed(1).pik_schedule(PikSchedule.uniform(PikMode.cash())), MertonMcConfig)
    True
    """

    def __init__(self, merton: MertonModel) -> None:
        """
        Create a configuration with registry-sourced simulation defaults.

        Parameters
        ----------
        merton : MertonModel
            Structural credit model driving asset dynamics and default.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def pik_schedule(self, s: PikSchedule) -> MertonMcConfig:
        """
        Set the payment-in-kind schedule for the Merton MC config.

        Parameters
        ----------
        s : PikSchedule
            PIK schedule applied across coupon dates.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def num_paths(self, n: int) -> MertonMcConfig:
        """
        Set the number of Monte Carlo paths.

        Parameters
        ----------
        n : int
            Total paths to retain. With antithetic variates on, the mirrors
            count toward ``n`` rather than doubling it, so the engine draws
            only ``ceil(n / 2)`` independent normals.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def seed(self, s: int) -> MertonMcConfig:
        """
        Set the Monte Carlo RNG seed for reproducible paths.

        Parameters
        ----------
        s : int
            Unsigned 64-bit seed.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def antithetic(self, a: bool) -> MertonMcConfig:
        """
        Enable or disable antithetic variates.

        Parameters
        ----------
        a : bool
            When ``True``, pair each path with its antithetic counterpart.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def time_steps_per_year(self, n: int) -> MertonMcConfig:
        """
        Set simulation grid density.

        Parameters
        ----------
        n : int
            Time steps per year.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def barrier_crossing(self, p: BarrierCrossing) -> MertonMcConfig:
        """
        Set barrier-crossing policy for first-passage monitoring.

        Parameters
        ----------
        p : BarrierCrossing
            Discrete or Brownian-bridge policy.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def default_recovery_rate(self, r: float) -> MertonMcConfig:
        """
        Set flat recovery when no dynamic recovery model is configured.

        Parameters
        ----------
        r : float
            Recovery rate as a decimal in ``[0, 1]``.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def endogenous_hazard(self, h: EndogenousHazardSpec) -> MertonMcConfig:
        """
        Set an endogenous hazard model.

        Parameters
        ----------
        h : EndogenousHazardSpec
            Endogenous hazard specification.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def dynamic_recovery(self, r: DynamicRecoverySpec) -> MertonMcConfig:
        """
        Set a dynamic recovery model.

        Parameters
        ----------
        r : DynamicRecoverySpec
            Dynamic recovery specification.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def toggle_model(self, t: ToggleExerciseModel) -> MertonMcConfig:
        """
        Set toggle exercise model for PIK/cash decisions.

        Parameters
        ----------
        t : ToggleExerciseModel
            Toggle exercise model.

        Returns
        -------
        MertonMcConfig
            Updated configuration (fluent).

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    @staticmethod
    def from_json(json: str) -> MertonMcConfig:
        """
        Deserialize from canonical JSON.

        Parameters
        ----------
        json : str
            JSON-encoded configuration.

        Returns
        -------
        MertonMcConfig
            The decoded configuration.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for a Merton MC configuration.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import MertonMcConfig
        >>> from finstack_quant.valuations.models.credit import MertonModel
        >>> config = MertonMcConfig(MertonModel(100.0, 0.25, 80.0, 0.04)).num_paths(256).seed(42)
        >>> MertonMcConfig.from_json(config.to_json()).to_json() == config.to_json()
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            JSON-encoded configuration.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class MertonMcResult:
    """
    Result from Merton Monte Carlo PIK bond pricing.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import (
    ...     BarrierCrossing,
    ...     Bond,
    ...     MertonMcConfig,
    ...     PikMode,
    ...     PikSchedule,
    ... )
    >>> from finstack_quant.valuations.models.credit import MertonModel
    >>> config = (
    ...     MertonMcConfig(MertonModel(100.0, 0.25, 60.0, 0.04))
    ...     .num_paths(64)
    ...     .seed(7)
    ...     .pik_schedule(PikSchedule.uniform(PikMode.pik()))
    ...     .barrier_crossing(BarrierCrossing.discrete())
    ... )
    >>> bond = Bond.fixed(
    ...     "PIK-1",
    ...     Money(100.0, Currency("USD")),
    ...     Rate(0.08),
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ...     "USD-OIS",
    ... )
    >>> bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 15)).clean_price_pct > 0.0
    True
    """

    @property
    def clean_price_pct(self) -> float:
        """
        Clean price as a percentage of par.

        Returns
        -------
        float
            Mean discounted path value divided by notional, times 100, so
            ``98.7`` means 98.7% of par. Quoted on the same discount basis the
            configuration supplied.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def dirty_price_pct(self) -> float:
        """
        Dirty price as a percentage of par.

        Returns
        -------
        float
            Always equal to :attr:`clean_price_pct`: the Monte Carlo engine
            works in continuous time and never separates accrued interest. Use
            the pricer's metrics pipeline for a genuine clean/dirty split.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def expected_loss(self) -> float:
        """
        Expected loss as a fraction of PIK-aware risk-free PV.

        Returns
        -------
        float
            ``1 - mean_mc_pv / risk_free_pv``, so ``0.03`` is a 3% credit
            haircut. The benchmark PV accretes notional under the configured
            PIK schedule, and the value turns negative when the simulated PV
            exceeds that benchmark.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def unexpected_loss(self) -> float:
        """
        Unexpected loss (std dev of path PVs / notional).

        Returns
        -------
        float
            Dispersion of the loss distribution as a fraction of par, not a
            percentage: ``0.05`` here is comparable to 5 points on
            :attr:`clean_price_pct`.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def expected_shortfall_95(self) -> float:
        """
        Expected shortfall at the 95% confidence level.

        Returns
        -------
        float
            Mean of the worst 5% of path PVs, expressed as a percentage of par
            like :attr:`clean_price_pct`. It is a price level rather than a
            loss, so lower is worse and it never exceeds the clean price.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def average_pik_fraction(self) -> float:
        """
        Average PIK fraction across coupon dates and paths.

        Returns
        -------
        float
            PIK elections divided by simulated coupon periods, in ``[0, 1]``.
            Counts whole elections, so a 50/50 split coupon still registers as
            one election. Identical to
            ``path_statistics.pik_exercise_rate``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def effective_spread_bp(self) -> float:
        """
        Effective spread in basis points versus risk-free PV.

        Returns
        -------
        float
            Constant continuous spread ``s`` solving ``risk_free_pv`` with each
            discount factor scaled by ``exp(-s * t)`` equal to the mean
            simulated PV. Solved on whichever discount basis priced the bond,
            so curve shape stays out of the spread.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def path_statistics(self) -> PathStatistics:
        """
        Path-level simulation statistics.

        Returns
        -------
        PathStatistics
            Default frequency, timing, recovery, and PIK-election diagnostics
            for the same run that produced these prices.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def num_paths(self) -> int:
        """
        Number of Monte Carlo paths used.

        Returns
        -------
        int
            Paths actually retained, matching the configured
            ``MertonMcConfig.num_paths``. Antithetic mirrors count toward this
            total instead of doubling it.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def standard_error(self) -> float:
        """
        Standard error of the clean price (percentage of par).

        Returns
        -------
        float
            Sampling error of :attr:`clean_price_pct` in the same
            percent-of-par units, so 1.96 of these brackets a 95% interval
            either side of the price. With antithetic variates the estimate
            comes from pair averages, which keeps the negatively correlated
            legs from understating it.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class PathStatistics:
    """
    Path-level statistics from a Merton Monte Carlo simulation.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import (
    ...     BarrierCrossing,
    ...     Bond,
    ...     MertonMcConfig,
    ...     PikMode,
    ...     PikSchedule,
    ... )
    >>> from finstack_quant.valuations.models.credit import MertonModel
    >>> config = (
    ...     MertonMcConfig(MertonModel(100.0, 0.25, 60.0, 0.04))
    ...     .num_paths(64)
    ...     .seed(7)
    ...     .pik_schedule(PikSchedule.uniform(PikMode.pik()))
    ...     .barrier_crossing(BarrierCrossing.discrete())
    ... )
    >>> bond = Bond.fixed(
    ...     "PIK-1",
    ...     Money(100.0, Currency("USD")),
    ...     Rate(0.08),
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ...     "USD-OIS",
    ... )
    >>> 0.0 <= bond.price_merton_mc(config, 0.04, datetime.date(2024, 1, 15)).path_statistics.default_rate <= 1.0
    True
    """

    @property
    def default_rate(self) -> float:
        """
        Fraction of paths that defaulted.

        Returns
        -------
        float
            Defaulted paths divided by simulated paths, in ``[0, 1]``. This is
            a cumulative default probability to maturity, not an annualised
            hazard rate.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def avg_default_time(self) -> float:
        """
        Average default time in years among defaulted paths.

        Returns
        -------
        float
            Mean time from the valuation date to the barrier crossing,
            averaged over defaulted paths only. Exactly ``0.0`` when no path
            defaulted, so check :attr:`default_rate` before reading it.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def avg_terminal_notional(self) -> float:
        """
        Average terminal notional reflecting PIK accretion.

        Returns
        -------
        float
            Currency-unit notional at maturity averaged over surviving paths,
            so it exceeds the issued notional whenever coupons were PIKed.
            Falls back to the issued notional when every path defaulted.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def avg_recovery_pct(self) -> float:
        """
        Average recovery percentage among defaulted paths.

        Returns
        -------
        float
            Decimal fraction despite the ``_pct`` name: ``0.40`` means 40%
            recovery on the notional accreted up to the default time,
            averaged over defaulted paths. Exactly ``0.0`` when no path
            defaulted.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def pik_exercise_rate(self) -> float:
        """
        Fraction of coupon dates where PIK was elected.

        Returns
        -------
        float
            Same quantity as ``MertonMcResult.average_pik_fraction``, in
            ``[0, 1]``: a uniform cash schedule pins it to ``0.0`` and a
            uniform PIK schedule to ``1.0``, so only toggle and split
            schedules put it in between.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class PikMode:
    """
    Per-coupon PIK behavior for the Merton Monte Carlo engine.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import PikMode
    >>> PikMode.cash().to_json()
    '"cash"'
    """

    @staticmethod
    def cash() -> PikMode:
        """
        Coupon paid entirely in cash.

        Returns
        -------
        PikMode
            Cash mode, the default a schedule falls back to before its first
            step and whenever a toggle model is missing.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode
        >>> PikMode.cash().to_json()
        '"cash"'
        """
        ...

    @staticmethod
    def pik() -> PikMode:
        """
        Coupon accreted to notional.

        Returns
        -------
        PikMode
            Payment-in-kind mode: the coupon pays no cash and instead raises
            the notional that later coupons and recovery are computed on.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode
        >>> PikMode.pik().to_json()
        '"pik"'
        """
        ...

    @staticmethod
    def split(cash_fraction: float, pik_fraction: float) -> PikMode:
        """
        Coupon split between cash and PIK.

        Parameters
        ----------
        cash_fraction : float
            Fraction paid in cash as a decimal.
        pik_fraction : float
            Fraction accreted to notional as a decimal.

        Returns
        -------
        PikMode
            Split PIK mode. The two fractions must be non-negative and sum to
            one; the engine rejects anything else when the bond is priced, not
            when this mode is built.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode
        >>> PikMode.split(0.5, 0.5).to_json()
        '{"split":{"cash_fraction":0.5,"pik_fraction":0.5}}'
        """
        ...

    @staticmethod
    def toggle() -> PikMode:
        """
        Defer to the toggle exercise model on the config.

        Returns
        -------
        PikMode
            Toggle mode, which decides cash versus PIK per path from
            ``MertonMcConfig.toggle_model``. Without that model set the coupon
            silently falls back to cash.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode
        >>> PikMode.toggle().to_json()
        '"toggle"'
        """
        ...

    @staticmethod
    def from_json(json: str) -> PikMode:
        """
        Deserialize from canonical JSON.

        Parameters
        ----------
        json : str
            JSON-encoded PIK mode.

        Returns
        -------
        PikMode
            The decoded mode.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for a PIK mode.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode
        >>> PikMode.from_json('"pik"').to_json()
        '"pik"'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            JSON-encoded PIK mode.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class PikSchedule:
    """
    Time-varying PIK schedule for the Merton Monte Carlo engine.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import PikMode, PikSchedule
    >>> PikSchedule.uniform(PikMode.pik()).mode_at(1.0).to_json()
    '"pik"'
    """

    @staticmethod
    def uniform(mode: PikMode) -> PikSchedule:
        """
        Apply the same PIK mode at every coupon date.

        Parameters
        ----------
        mode : PikMode
            PIK mode applied uniformly.

        Returns
        -------
        PikSchedule
            Uniform schedule; every coupon date resolves to ``mode`` for the
            whole life of the bond.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode, PikSchedule
        >>> PikSchedule.uniform(PikMode.pik()).mode_at(1.0).to_json()
        '"pik"'
        """
        ...

    @staticmethod
    def stepped(steps: list[tuple[float, PikMode]]) -> PikSchedule:
        """
        Step-function PIK schedule keyed by year fraction.

        Parameters
        ----------
        steps : list[tuple[float, PikMode]]
            ``(year_fraction, mode)`` pairs sorted by time ascending.

        Returns
        -------
        PikSchedule
            Stepped schedule in which each entry stays in force from its year
            fraction until the next one. Coupons before the first step fall
            back to cash, so start the list at ``0.0`` to control them.

        Raises
        ------
        ValueError
            If ``steps`` cannot be parsed or fails validation at pricing time.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode, PikSchedule
        >>> schedule = PikSchedule.stepped([(0.0, PikMode.pik()), (2.0, PikMode.cash())])
        >>> schedule.mode_at(2.5).to_json()
        '"cash"'
        """
        ...

    def mode_at(self, t: float) -> PikMode:
        """
        Look up the active PIK mode at time ``t``.

        Parameters
        ----------
        t : float
            Time in years from the valuation date.

        Returns
        -------
        PikMode
            Active mode at ``t``.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    @staticmethod
    def from_json(json: str) -> PikSchedule:
        """
        Deserialize from canonical JSON.

        Parameters
        ----------
        json : str
            JSON-encoded PIK schedule.

        Returns
        -------
        PikSchedule
            The decoded schedule.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for a PIK schedule.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import PikMode, PikSchedule
        >>> PikSchedule.from_json('{"stepped":[[0.0,"pik"],[2.0,"cash"]]}').mode_at(0.5).to_json()
        '"pik"'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to compact JSON.

        Returns
        -------
        str
            JSON-encoded PIK schedule.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class TermLoan:
    """
    Typed wrapper for the canonical Rust ``TermLoan`` instrument.

    Rust has no ``fixed``/``floating`` convenience constructors for term
    loans; construct via :meth:`TermLoan.from_json` with a canonical
    ``finstack_quant.instrument/1`` envelope or start from
    :meth:`TermLoan.example`. Instances are accepted directly by
    :func:`price_instrument`, :func:`price_instrument_with_metrics`, and
    :func:`instrument_cashflows_json`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import TermLoan
    >>> loan = TermLoan.example()
    >>> loan.id
    'TERM-LOAN-USD-5Y'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> TermLoan:
        """
        Deserialize a validated term loan from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"term_loan"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        TermLoan
            The validated term loan.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails term-loan validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import TermLoan
        >>> TermLoan.from_json(TermLoan.example().to_json()).id
        'TERM-LOAN-USD-5Y'

        """
        ...

    @staticmethod
    def example() -> TermLoan:
        """
        Canonical example term loan (mirrors Rust ``TermLoan::example``).

        Returns
        -------
        TermLoan
            A 5-year USD fixed-rate loan (6%, quarterly, Act/360, 2.5%
            per-period amortization).

        Raises
        ------
        ValueError
            If construction fails (should not occur).

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import TermLoan
        >>> TermLoan.example().id
        'TERM-LOAN-USD-5Y'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`TermLoan.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class FixedLegSpec:
    """
    Fixed leg of an interest-rate swap.

    Thin typed wrapper for the canonical Rust ``FixedLegSpec``. Used to build
    the fixed leg of an :class:`InterestRateSwap` via
    :meth:`InterestRateSwapBuilder.fixed`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import FixedLegSpec
    >>> leg = FixedLegSpec(
    ...     "USD-OIS",
    ...     0.04,
    ...     Tenor.semi_annual(),
    ...     DayCount.THIRTY_360,
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ...     compounding_simple=False,
    ... )
    >>> "0.04" in repr(leg)
    True
    """

    def __init__(
        self,
        discount_curve_id: str,
        rate: float,
        frequency: Tenor,
        day_count: DayCount,
        start: datetime.date,
        end: datetime.date,
        *,
        compounding_simple: bool,
        business_day_convention: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
        stub: Literal["none", "short_front", "short_back", "long_front", "long_back"] = "short_front",
        payment_lag_days: int = 0,
        end_of_month: bool = False,
    ) -> None:
        """
        Construct a fixed leg specification.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        rate : float
            Fixed rate as a decimal (0.04 = 4%).
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        start : datetime.date
            Start date of the fixed leg.
        end : datetime.date
            End date of the fixed leg.
        compounding_simple : bool
            If true, use simple interest on the accrual fraction. Required:
            the canonical Rust ``FixedLegSpec`` field has no default.
        business_day_convention : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.
        stub : {"none", "short_front", "short_back", "long_front", "long_back"}, default "short_front"
            Stub period handling rule.
        payment_lag_days : int, default 0
            Payment lag in business days after period end.
        end_of_month : bool, default False
            End-of-month roll convention.

        Raises
        ------
        ValueError
            If an enum value is invalid or the accrual period is malformed
            (``start >= end``).

        """
        ...

class FloatLegSpec:
    """
    Floating leg of an interest-rate swap.

    Thin typed wrapper for the canonical Rust ``FloatLegSpec``. Used to build
    the floating leg of an :class:`InterestRateSwap` via
    :meth:`InterestRateSwapBuilder.float`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import FloatLegSpec
    >>> leg = FloatLegSpec(
    ...     "USD-OIS",
    ...     "USD-SOFR-3M",
    ...     0.0,
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     datetime.date(2024, 1, 15),
    ...     datetime.date(2029, 1, 15),
    ... )
    >>> "spread_bp=0" in repr(leg)
    True
    """

    def __init__(
        self,
        discount_curve_id: str,
        forward_curve_id: str,
        spread_bp: float,
        frequency: Tenor,
        day_count: DayCount,
        start: datetime.date,
        end: datetime.date,
        *,
        business_day_convention: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
        stub: Literal["none", "short_front", "short_back", "long_front", "long_back"] = "short_front",
        reset_lag_days: int = -1,
        fixing_calendar_id: str | None = None,
        payment_lag_days: int = 0,
        end_of_month: bool = False,
    ) -> None:
        """
        Construct a floating leg specification.

        Parameters
        ----------
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        forward_curve_id : str
            Forward curve identifier for rate projections.
        spread_bp : float
            Spread over the index in basis points.
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        start : datetime.date
            Start date of the floating leg.
        end : datetime.date
            End date of the floating leg.
        business_day_convention : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.
        stub : {"none", "short_front", "short_back", "long_front", "long_back"}, default "short_front"
            Stub period handling rule.
        reset_lag_days : int, default -1
            Reset lag in business days for the floating rate fixing.
            ``-1`` is a sentinel meaning "use the convention default".
        fixing_calendar_id : str, optional
            Calendar used for rate fixing (reset lag).
        payment_lag_days : int, default 0
            Payment lag in business days after period end.
        end_of_month : bool, default False
            End-of-month roll convention.

        Raises
        ------
        ValueError
            If an enum value is invalid or the accrual period is malformed
            (``start >= end``).

        """
        ...

class PremiumLegSpec:
    """
    Premium (fixed coupon) leg of a CDS or CDS index.

    Thin typed wrapper for the canonical Rust ``PremiumLegSpec``. Used to
    build the premium leg of a :class:`CreditDefaultSwap` or :class:`CDSIndex`
    via :meth:`CreditDefaultSwapBuilder.premium` or
    :meth:`CDSIndexBuilder.premium`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.valuations.instruments import PremiumLegSpec
    >>> leg = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 6, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     100.0,
    ...     "USD-OIS",
    ... )
    >>> "spread_bp=100" in repr(leg)
    True
    """

    def __init__(
        self,
        start: datetime.date,
        end: datetime.date,
        frequency: Tenor,
        day_count: DayCount,
        spread_bp: float,
        discount_curve_id: str,
        *,
        stub: Literal["none", "short_front", "short_back", "long_front", "long_back"] = "short_front",
        business_day_convention: Literal[
            "unadjusted", "following", "modified_following", "preceding", "modified_preceding"
        ] = "modified_following",
        calendar_id: str | None = None,
    ) -> None:
        """
        Construct a premium leg specification.

        Parameters
        ----------
        start : datetime.date
            Start date of protection / premium accrual.
        end : datetime.date
            End date of protection / premium accrual.
        frequency : Tenor
            Payment frequency.
        day_count : DayCount
            Day count convention for accrual.
        spread_bp : float
            Fixed running spread in basis points (e.g. 100.0 = 100bp = 1%).
        discount_curve_id : str
            Discount curve identifier for pricing this leg.
        stub : {"none", "short_front", "short_back", "long_front", "long_back"}, default "short_front"
            Stub period handling rule.
        business_day_convention : {"unadjusted", "following", "modified_following", "preceding", "modified_preceding"}, default "modified_following"
            Business day convention for payment dates.
        calendar_id : str, optional
            Calendar used for business day adjustments.

        Raises
        ------
        ValueError
            If an enum value is invalid.

        """
        ...

class ProtectionLegSpec:
    """
    Protection (default-contingent) leg of a CDS or CDS index.

    Thin typed wrapper for the canonical Rust ``ProtectionLegSpec``. Used to
    build the protection leg of a :class:`CreditDefaultSwap` or
    :class:`CDSIndex` via :meth:`CreditDefaultSwapBuilder.protection` or
    :meth:`CDSIndexBuilder.protection`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import ProtectionLegSpec
    >>> leg = ProtectionLegSpec("ACME-CDS", 0.4, 3)
    >>> "recovery_rate=0.4" in repr(leg)
    True
    """

    def __init__(
        self,
        credit_curve_id: str,
        recovery_rate: float,
        settlement_delay: int = 3,
    ) -> None:
        """
        Construct a protection leg specification.

        Parameters
        ----------
        credit_curve_id : str
            Hazard/credit curve identifier for default probabilities.
        recovery_rate : float
            Recovery rate in ``[0.0, 1.0]`` (e.g. 0.4 = 40%).
        settlement_delay : int, default 3
            Settlement delay in business days.

        Raises
        ------
        ValueError
            If ``recovery_rate`` is outside ``[0.0, 1.0]``.

        """
        ...

class InterestRateSwap:
    """
    Typed wrapper for the canonical Rust ``InterestRateSwap``.

    Build with :meth:`InterestRateSwap.builder`; instances are accepted
    directly by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     FixedLegSpec,
    ...     FloatLegSpec,
    ...     InterestRateSwap,
    ... )
    >>> start = datetime.date(2024, 1, 15)
    >>> end = datetime.date(2029, 1, 15)
    >>> swap = (
    ...     InterestRateSwap
    ...     .builder()
    ...     .id("IRS-1")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .side("pay")
    ...     .fixed(
    ...         FixedLegSpec(
    ...             "USD-OIS", 0.04, Tenor.semi_annual(), DayCount.THIRTY_360, start, end, compounding_simple=False
    ...         )
    ...     )
    ...     .float(FloatLegSpec("USD-OIS", "USD-SOFR-3M", 0.0, Tenor.quarterly(), DayCount.ACT_360, start, end))
    ...     .build()
    ... )
    >>> swap.id
    'IRS-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> InterestRateSwapBuilder:
        """
        Create a fluent builder (mirrors Rust ``InterestRateSwap::builder()``).

        Returns
        -------
        InterestRateSwapBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> builder = InterestRateSwap.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> InterestRateSwap:
        """
        Deserialize a validated swap from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"interest_rate_swap"`` payload. The UTF-8 input must not exceed
            16 MiB. Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        InterestRateSwap
            The validated swap.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails swap validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import InterestRateSwap
        >>> try:
        ...     InterestRateSwap.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`InterestRateSwap.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class InterestRateSwapBuilder:
    """
    Fluent builder returned by :meth:`InterestRateSwap.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import InterestRateSwap
    >>> isinstance(InterestRateSwap.builder(), InterestRateSwap.builder().__class__)
    True
    """

    def id(self, value: str) -> InterestRateSwapBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the swap.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> InterestRateSwapBuilder:
        """
        Set the notional (both legs).

        Parameters
        ----------
        value : Money
            Notional amount shared by both legs.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> InterestRateSwapBuilder:
        """
        Set the swap direction: ``"pay"`` or ``"receive"`` (fixed leg).

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to pay fixed/receive floating, ``"receive"`` for the
            opposite.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        """
        ...

    def fixed(self, value: FixedLegSpec) -> InterestRateSwapBuilder:
        """
        Set the fixed leg specification.

        Parameters
        ----------
        value : FixedLegSpec
            Fixed leg specification.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        """
        ...

    def float(self, value: FloatLegSpec) -> InterestRateSwapBuilder:
        """
        Set the floating leg specification.

        Parameters
        ----------
        value : FloatLegSpec
            Floating leg specification.

        Returns
        -------
        InterestRateSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`InterestRateSwapBuilder.build`.

        """
        ...

    def build(self) -> InterestRateSwap:
        """
        Build the validated swap.

        Returns
        -------
        InterestRateSwap
            The validated swap.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class Swaption:
    """
    Typed wrapper for the canonical Rust ``Swaption`` instrument.

    Build with :meth:`Swaption.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     FixedLegSpec,
    ...     FloatLegSpec,
    ...     Swaption,
    ... )
    >>> start = datetime.date(2025, 1, 15)
    >>> end = datetime.date(2030, 1, 15)
    >>> fixed = FixedLegSpec(
    ...     "USD-OIS",
    ...     0.04,
    ...     Tenor.semi_annual(),
    ...     DayCount.THIRTY_360,
    ...     start,
    ...     end,
    ...     compounding_simple=False,
    ... )
    >>> floating = FloatLegSpec(
    ...     "USD-OIS",
    ...     "USD-SOFR-3M",
    ...     0.0,
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     start,
    ...     end,
    ... )
    >>> swaption = (
    ...     Swaption
    ...     .builder()
    ...     .id("SWPT-1")
    ...     .option_type("call")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .expiry(datetime.date(2025, 1, 13))
    ...     .exercise_style("european")
    ...     .settlement("cash")
    ...     .cash_settlement_method("par_yield")
    ...     .vol_model("normal")
    ...     .vol_surface_id("USD-SWPT-VOL")
    ...     .underlying_fixed_leg(fixed)
    ...     .underlying_float_leg(floating)
    ...     .build()
    ... )
    >>> swaption.id
    'SWPT-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> SwaptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``Swaption::builder()``).

        Returns
        -------
        SwaptionBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> builder = Swaption.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> Swaption:
        """
        Deserialize a validated swaption from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"swaption"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        Swaption
            The validated swaption.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails swaption validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Swaption
        >>> try:
        ...     Swaption.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`Swaption.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class SwaptionBuilder:
    """
    Fluent builder returned by :meth:`Swaption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import Swaption
    >>> isinstance(Swaption.builder(), Swaption.builder().__class__)
    True
    """

    def id(self, value: str) -> SwaptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> SwaptionBuilder:
        """
        Set the option type: ``"call"`` (payer) or ``"put"`` (receiver).

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        """
        ...

    def notional(self, value: Money) -> SwaptionBuilder:
        """
        Set the notional amount of the underlying swap.

        Parameters
        ----------
        value : Money
            Notional amount of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def expiry(self, value: datetime.date) -> SwaptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def exercise_style(self, value: Literal["european", "bermudan", "american"]) -> SwaptionBuilder:
        """
        Set whether exercise is European, American, or Bermudan.

        Parameters
        ----------
        value : {"european", "bermudan", "american"}
            Exercise style of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        """
        ...

    def settlement(self, value: Literal["physical", "cash"]) -> SwaptionBuilder:
        """
        Set the settlement method.

        Parameters
        ----------
        value : {"physical", "cash"}
            Settlement method of the swaption.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized settlement method.

        """
        ...

    def cash_settlement_method(self, value: Literal["par_yield", "isda_par_par", "zero_coupon"]) -> SwaptionBuilder:
        """
        Set the cash settlement annuity method.

        Only affects pricing when ``settlement`` is ``"cash"``.

        Parameters
        ----------
        value : {"par_yield", "isda_par_par", "zero_coupon"}
            Cash settlement annuity method.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized cash settlement method.

        """
        ...

    def vol_model(self, value: Literal["black", "normal"]) -> SwaptionBuilder:
        """
        Set the volatility model.

        Parameters
        ----------
        value : {"black", "normal"}
            Volatility model used for pricing.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized volatility model.

        """
        ...

    def vol_surface_id(self, value: str) -> SwaptionBuilder:
        """
        Set the volatility surface identifier.

        Parameters
        ----------
        value : str
            Volatility surface identifier for option pricing.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def underlying_fixed_leg(self, value: FixedLegSpec) -> SwaptionBuilder:
        """
        Set the complete fixed leg of the underlying swap.

        Parameters
        ----------
        value : FixedLegSpec
            Fixed leg of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def underlying_float_leg(self, value: FloatLegSpec) -> SwaptionBuilder:
        """
        Set the complete floating leg of the underlying swap.

        Parameters
        ----------
        value : FloatLegSpec
            Floating leg of the underlying swap.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`SwaptionBuilder.build`.

        """
        ...

    def sabr_params_json(self, value: str) -> SwaptionBuilder:
        """
        Set the SABR volatility model parameters from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded SABR parameters object with fields ``alpha``,
            ``beta``, ``nu``, ``rho`` and optional ``shift``.

        Returns
        -------
        SwaptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the SABR parameters shape.

        """
        ...

    def build(self) -> Swaption:
        """
        Build the validated swaption.

        Returns
        -------
        Swaption
            The validated swaption.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class CapFloor:
    """
    Typed wrapper for the canonical Rust ``CapFloor`` instrument.

    Build with :meth:`CapFloor.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import CapFloor
    >>> cap = (
    ...     CapFloor
    ...     .builder()
    ...     .id("CAP-1")
    ...     .rate_option_type("cap")
    ...     .notional(Money(5_000_000.0, Currency("USD")))
    ...     .strike(0.05)
    ...     .start_date(datetime.date(2024, 1, 15))
    ...     .maturity(datetime.date(2027, 1, 15))
    ...     .frequency(Tenor.quarterly())
    ...     .day_count(DayCount.ACT_360)
    ...     .discount_curve_id("USD-OIS")
    ...     .forward_curve_id("USD-SOFR-3M")
    ...     .vol_surface_id("USD-CAP-VOL")
    ...     .vol_type("normal")
    ...     .build()
    ... )
    >>> cap.id
    'CAP-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> CapFloorBuilder:
        """
        Create a fluent builder (mirrors Rust ``CapFloor::builder()``).

        Returns
        -------
        CapFloorBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> builder = CapFloor.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CapFloor:
        """
        Deserialize a validated cap/floor from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"cap_floor"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        CapFloor
            The validated cap/floor.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails cap/floor validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CapFloor
        >>> try:
        ...     CapFloor.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`CapFloor.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class CapFloorBuilder:
    """
    Fluent builder returned by :meth:`CapFloor.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CapFloor
    >>> isinstance(CapFloor.builder(), CapFloor.builder().__class__)
    True
    """

    def id(self, value: str) -> CapFloorBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the cap/floor.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def rate_option_type(self, value: Literal["cap", "floor", "caplet", "floorlet"]) -> CapFloorBuilder:
        """
        Set whether this contract is a call, put, cap, or floor.

        Parameters
        ----------
        value : {"cap", "floor", "caplet", "floorlet"}
            Option type of the instrument: ``"cap"``/``"floor"`` for a
            series of caplets/floorlets, or ``"caplet"``/``"floorlet"`` for
            a single period.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        """
        ...

    def notional(self, value: Money) -> CapFloorBuilder:
        """
        Set the notional amount of the cap or floor.

        Parameters
        ----------
        value : Money
            Notional amount.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def strike(self, value: float) -> CapFloorBuilder:
        """
        Set the strike rate of the cap or floor.

        Parameters
        ----------
        value : float
            Strike as a decimal (0.05 = 5%).

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not finite.

        """
        ...

    def spread(self, value: float) -> CapFloorBuilder:
        """
        Set the contractual spread added to the referenced rate.

        Parameters
        ----------
        value : float
            Spread in decimal rate units, added after projecting the index.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not finite.

        """
        ...

    def start_date(self, value: datetime.date) -> CapFloorBuilder:
        """
        Set the start date of the underlying period.

        Parameters
        ----------
        value : datetime.date
            Start date of the underlying period.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def maturity(self, value: datetime.date) -> CapFloorBuilder:
        """
        Set the end date of the underlying period.

        Parameters
        ----------
        value : datetime.date
            End date of the underlying period.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def frequency(self, value: Tenor) -> CapFloorBuilder:
        """
        Set the payment frequency.

        Parameters
        ----------
        value : Tenor
            Payment frequency for caps/floors.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def day_count(self, value: DayCount) -> CapFloorBuilder:
        """
        Set the day count convention.

        Parameters
        ----------
        value : DayCount
            Day count convention.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def calendar_id(self, value: str) -> CapFloorBuilder:
        """
        Set the holiday calendar identifier for schedule and roll conventions.

        Parameters
        ----------
        value : str
            Holiday calendar identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def discount_curve_id(self, value: str) -> CapFloorBuilder:
        """
        Set the discount curve identifier.

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def forward_curve_id(self, value: str) -> CapFloorBuilder:
        """
        Set the forward curve identifier.

        Parameters
        ----------
        value : str
            Forward curve identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def vol_surface_id(self, value: str) -> CapFloorBuilder:
        """
        Set the volatility surface identifier.

        Parameters
        ----------
        value : str
            Volatility surface identifier.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def vol_type(self, value: Literal["lognormal", "shifted_lognormal", "normal", "auto"]) -> CapFloorBuilder:
        """
        Set the volatility type convention.

        Parameters
        ----------
        value : {"lognormal", "shifted_lognormal", "normal", "auto"}
            Volatility convention. Must match the convention of the
            configured volatility surface. ``"auto"`` resolves to
            ``"lognormal"``, pricing each caplet with Black-76 where
            well-defined and falling back to an equivalent Bachelier price
            otherwise (e.g. a cap whose schedule crosses a zero forward
            rate).

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized volatility type.

        """
        ...

    def vol_shift(self, value: float) -> CapFloorBuilder:
        """
        Set the displacement shift used for shifted-lognormal pricing.

        Parameters
        ----------
        value : float
            Displacement added to forward and strike. Must be non-negative.

        Returns
        -------
        CapFloorBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CapFloorBuilder.build`.

        """
        ...

    def build(self) -> CapFloor:
        """
        Build the validated cap/floor.

        Returns
        -------
        CapFloor
            The validated cap/floor.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class CreditDefaultSwap:
    """
    Typed wrapper for the canonical Rust ``CreditDefaultSwap`` instrument.

    Build with :meth:`CreditDefaultSwap.builder`; instances are accepted
    directly by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     CreditDefaultSwap,
    ...     PremiumLegSpec,
    ...     ProtectionLegSpec,
    ... )
    >>> premium = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 6, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     100.0,
    ...     "USD-OIS",
    ... )
    >>> protection = ProtectionLegSpec("ACME-CDS", 0.4, 3)
    >>> cds = (
    ...     CreditDefaultSwap
    ...     .builder()
    ...     .id("CDS-1")
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .side("pay")
    ...     .convention("isda_na")
    ...     .premium(premium)
    ...     .protection(protection)
    ...     .build()
    ... )
    >>> cds.id
    'CDS-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> CreditDefaultSwapBuilder:
        """
        Create a fluent builder (mirrors Rust ``CreditDefaultSwap::builder()``).

        Returns
        -------
        CreditDefaultSwapBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> builder = CreditDefaultSwap.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CreditDefaultSwap:
        """
        Deserialize a validated CDS from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"credit_default_swap"`` payload. The UTF-8 input must not exceed
            16 MiB. Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        CreditDefaultSwap
            The validated CDS.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails CDS validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
        >>> try:
        ...     CreditDefaultSwap.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`CreditDefaultSwap.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class CreditDefaultSwapBuilder:
    """
    Fluent builder returned by :meth:`CreditDefaultSwap.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CreditDefaultSwap
    >>> isinstance(CreditDefaultSwap.builder(), CreditDefaultSwap.builder().__class__)
    True
    """

    def id(self, value: str) -> CreditDefaultSwapBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the CDS.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> CreditDefaultSwapBuilder:
        """
        Set the notional amount.

        Parameters
        ----------
        value : Money
            Notional amount of protection.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> CreditDefaultSwapBuilder:
        """
        Set the protection buyer/seller perspective.

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to buy protection (pay premium), ``"receive"`` to sell
            protection (receive premium).

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        """
        ...

    def convention(self, value: Literal["isda_na", "isda_eu", "isda_as", "custom"]) -> CreditDefaultSwapBuilder:
        """
        Set the ISDA regional convention.

        Parameters
        ----------
        value : {"isda_na", "isda_eu", "isda_as", "custom"}
            ISDA CDS convention (North American, European, or Asian), or
            ``"custom"`` for a manually configured convention.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized convention.

        """
        ...

    def premium(self, value: PremiumLegSpec) -> CreditDefaultSwapBuilder:
        """
        Set the premium leg specification.

        Parameters
        ----------
        value : PremiumLegSpec
            Premium leg specification.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        """
        ...

    def protection(self, value: ProtectionLegSpec) -> CreditDefaultSwapBuilder:
        """
        Set the protection leg specification.

        Parameters
        ----------
        value : ProtectionLegSpec
            Protection leg specification.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        """
        ...

    def doc_clause(
        self,
        value: Literal["cr14", "mr14", "mm14", "xr14", "isda_na", "isda_eu", "isda_as", "isda_au", "isda_nz", "custom"],
    ) -> CreditDefaultSwapBuilder:
        """
        Set the ISDA documentation clause for restructuring credit events.

        Parameters
        ----------
        value : {"cr14", "mr14", "mm14", "xr14", "isda_na", "isda_eu", "isda_as", "isda_au", "isda_nz", "custom"}
            Restructuring documentation clause: one of the four 2014 ISDA
            restructuring elections (``"cr14"``/``"mr14"``/``"mm14"``/
            ``"xr14"``), a regional ISDA corporate default (``"isda_na"``/
            ``"isda_eu"``/``"isda_as"``/``"isda_au"``/``"isda_nz"``), or
            ``"custom"``. If never set, the effective clause is derived from
            the CDS convention.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized documentation clause.

        """
        ...

    def protection_effective_date(self, value: datetime.date) -> CreditDefaultSwapBuilder:
        """
        Set the protection effective date for a forward-starting CDS.

        Parameters
        ----------
        value : datetime.date
            Date on which credit protection begins. Must satisfy
            ``premium.start <= value <= premium.end``. When never set,
            protection starts on the premium leg start date.

        Returns
        -------
        CreditDefaultSwapBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CreditDefaultSwapBuilder.build`.

        """
        ...

    def build(self) -> CreditDefaultSwap:
        """
        Build the validated CDS.

        Returns
        -------
        CreditDefaultSwap
            The validated CDS.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class CDSIndex:
    """
    Typed wrapper for the canonical Rust ``CDSIndex`` instrument.

    Build with :meth:`CDSIndex.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import (
    ...     CDSIndex,
    ...     PremiumLegSpec,
    ...     ProtectionLegSpec,
    ... )
    >>> premium = PremiumLegSpec(
    ...     datetime.date(2024, 3, 20),
    ...     datetime.date(2029, 12, 20),
    ...     Tenor.quarterly(),
    ...     DayCount.ACT_360,
    ...     60.0,
    ...     "USD-OIS",
    ... )
    >>> protection = ProtectionLegSpec("CDX.NA.IG.HAZARD", 0.4, 3)
    >>> index = (
    ...     CDSIndex
    ...     .builder()
    ...     .id("CDX-IG-42")
    ...     .index_name("CDX.NA.IG")
    ...     .series(42)
    ...     .version(1)
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .index_factor(1.0)
    ...     .side("pay")
    ...     .convention("isda_na")
    ...     .premium(premium)
    ...     .protection(protection)
    ...     .pricing("single_curve")
    ...     .num_constituents(125)
    ...     .build()
    ... )
    >>> index.id
    'CDX-IG-42'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> CDSIndexBuilder:
        """
        Create a fluent builder (mirrors Rust ``CDSIndex::builder()``).

        The builder pre-seeds an empty ``constituents`` list (the Rust field
        has no default) so ``build()`` succeeds without calling
        :meth:`CDSIndexBuilder.constituents_json` when the index is priced in
        ``"single_curve"`` mode.

        Returns
        -------
        CDSIndexBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> builder = CDSIndex.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CDSIndex:
        """
        Deserialize a validated CDS index from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"cds_index"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        CDSIndex
            The validated CDS index.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails CDS-index validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSIndex
        >>> try:
        ...     CDSIndex.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`CDSIndex.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class CDSIndexBuilder:
    """
    Fluent builder returned by :meth:`CDSIndex.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CDSIndex
    >>> isinstance(CDSIndex.builder(), CDSIndex.builder().__class__)
    True
    """

    def id(self, value: str) -> CDSIndexBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the index trade.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def index_name(self, value: str) -> CDSIndexBuilder:
        """
        Set the CDS index name (for example ``CDX.NA.IG``).

        Parameters
        ----------
        value : str
            Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx Europe"``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def series(self, value: int) -> CDSIndexBuilder:
        """
        Set the CDS index series number (for example ``40``).

        Parameters
        ----------
        value : int
            Series number, e.g. ``42``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def version(self, value: int) -> CDSIndexBuilder:
        """
        Set the version number within the series.

        Parameters
        ----------
        value : int
            Version number, e.g. ``1``.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> CDSIndexBuilder:
        """
        Set the notional amount of the index.

        Parameters
        ----------
        value : Money
            Notional amount of the index.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def index_factor(self, value: float) -> CDSIndexBuilder:
        """
        Set the index factor (fraction of surviving notional).

        Parameters
        ----------
        value : float
            Index factor in ``[0.0, 1.0]``. ``1.0`` means no constituent has
            defaulted since series inception.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def side(self, value: Literal["pay", "receive"]) -> CDSIndexBuilder:
        """
        Set the protection buyer/seller perspective.

        Parameters
        ----------
        value : {"pay", "receive"}
            ``"pay"`` to buy protection (pay premium), ``"receive"`` to sell
            protection (receive premium).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        """
        ...

    def convention(self, value: Literal["isda_na", "isda_eu", "isda_as", "custom"]) -> CDSIndexBuilder:
        """
        Set the ISDA regional convention.

        Parameters
        ----------
        value : {"isda_na", "isda_eu", "isda_as", "custom"}
            ISDA CDS convention (North American, European, or Asian), or
            ``"custom"`` for a manually configured convention.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized convention.

        """
        ...

    def premium(self, value: PremiumLegSpec) -> CDSIndexBuilder:
        """
        Set the premium leg specification.

        Parameters
        ----------
        value : PremiumLegSpec
            Premium leg specification (coupon schedule and discounting).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def protection(self, value: ProtectionLegSpec) -> CDSIndexBuilder:
        """
        Set the protection leg specification.

        Parameters
        ----------
        value : ProtectionLegSpec
            Protection leg specification (credit curve and settlement).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def pricing(self, value: Literal["single_curve", "constituents"]) -> CDSIndexBuilder:
        """
        Set the pricing aggregation mode.

        Parameters
        ----------
        value : {"single_curve", "constituents"}
            ``"single_curve"`` prices the index against a single index hazard
            curve (synthetic CDS). ``"constituents"`` prices each issuer
            separately and aggregates by weight; requires
            :meth:`CDSIndexBuilder.constituents_json` to be set.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized pricing mode.

        """
        ...

    def constituents_json(self, value: str) -> CDSIndexBuilder:
        """
        Set the index constituents from a JSON array.

        Parameters
        ----------
        value : str
            JSON array of ``CDSIndexConstituent`` objects (``credit``,
            ``weight``, and optional ``defaulted``).

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the constituent-list shape.

        """
        ...

    def num_constituents(self, value: int) -> CDSIndexBuilder:
        """
        Set the number of reference entities in the index pool.

        Parameters
        ----------
        value : int
            Number of names in the index pool, e.g. ``125`` for CDX.NA.IG.
            Required for portfolio-level analytics (e.g. jump-to-default)
            when ``constituents`` is empty.

        Returns
        -------
        CDSIndexBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSIndexBuilder.build`.

        """
        ...

    def build(self) -> CDSIndex:
        """
        Build the validated CDS index.

        Returns
        -------
        CDSIndex
            The validated CDS index.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class CDSTranche:
    """
    Typed wrapper for the canonical Rust ``CDSTranche`` instrument.

    Build with :meth:`CDSTranche.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount, Tenor
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import CDSTranche
    >>> tranche = (
    ...     CDSTranche
    ...     .builder()
    ...     .id("CDX-IG-42-3-7")
    ...     .index_name("CDX.NA.IG")
    ...     .series(42)
    ...     .attach_pct(3.0)
    ...     .detach_pct(7.0)
    ...     .notional(Money(10_000_000.0, Currency("USD")))
    ...     .maturity(datetime.date(2029, 6, 20))
    ...     .running_coupon_bp(100.0)
    ...     .frequency(Tenor.quarterly())
    ...     .day_count(DayCount.ACT_360)
    ...     .discount_curve_id("USD-OIS")
    ...     .credit_index_id("CDX-IG-42-CURVE")
    ...     .side("buy_protection")
    ...     .build()
    ... )
    >>> tranche.id
    'CDX-IG-42-3-7'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> CDSTrancheBuilder:
        """
        Create a fluent builder (mirrors Rust ``CDSTranche::builder()``).

        The builder pre-seeds ``accumulated_loss(0.0)`` and
        ``standard_imm_dates(True)`` (the Rust fields have no defaults),
        which :meth:`CDSTrancheBuilder.accumulated_loss` and
        :meth:`CDSTrancheBuilder.standard_imm_dates` can override.

        Returns
        -------
        CDSTrancheBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> builder = CDSTranche.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> CDSTranche:
        """
        Deserialize a validated CDS tranche from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"cds_tranche"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        CDSTranche
            The validated CDS tranche.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails CDS-tranche validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import CDSTranche
        >>> try:
        ...     CDSTranche.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`CDSTranche.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class CDSTrancheBuilder:
    """
    Fluent builder returned by :meth:`CDSTranche.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import CDSTranche
    >>> isinstance(CDSTranche.builder(), CDSTranche.builder().__class__)
    True
    """

    def id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the tranche trade.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def index_name(self, value: str) -> CDSTrancheBuilder:
        """
        Set the underlying index name.

        Parameters
        ----------
        value : str
            Index name, e.g. ``"CDX.NA.IG"``, ``"CDX.NA.HY"``, ``"iTraxx EUR"``.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def series(self, value: int) -> CDSTrancheBuilder:
        """
        Set the CDS index series number (for example ``40``).

        Parameters
        ----------
        value : int
            Series number, e.g. ``42``.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def attach_pct(self, value: float) -> CDSTrancheBuilder:
        """
        Set the attachment point.

        Parameters
        ----------
        value : float
            Attachment point quoted in percent (e.g. ``0.0`` for equity;
            ``3.0`` for a tranche attaching at 3%).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def detach_pct(self, value: float) -> CDSTrancheBuilder:
        """
        Set the detachment point.

        Parameters
        ----------
        value : float
            Detachment point quoted in percent (e.g. ``3.0`` for a 0-3%
            tranche).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> CDSTrancheBuilder:
        """
        Set the notional amount of the tranche.

        Parameters
        ----------
        value : Money
            Notional amount of the tranche.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def maturity(self, value: datetime.date) -> CDSTrancheBuilder:
        """
        Set the maturity date of the tranche.

        Parameters
        ----------
        value : datetime.date
            Maturity date of the tranche.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def running_coupon_bp(self, value: float) -> CDSTrancheBuilder:
        """
        Set the CDS tranche running coupon in basis points.

        Parameters
        ----------
        value : float
            Running coupon in basis points (e.g. ``100.0`` = 1.00%).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def frequency(self, value: Tenor) -> CDSTrancheBuilder:
        """
        Set the payment frequency.

        Parameters
        ----------
        value : Tenor
            Payment frequency (typically quarterly).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def day_count(self, value: DayCount) -> CDSTrancheBuilder:
        """
        Set the day count convention.

        Parameters
        ----------
        value : DayCount
            Day count convention (typically Act/360).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def calendar_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the holiday calendar identifier.

        Parameters
        ----------
        value : str
            Holiday calendar identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def discount_curve_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the discount curve identifier (by quote currency).

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def credit_index_id(self, value: str) -> CDSTrancheBuilder:
        """
        Set the credit index identifier for survival/loss modeling.

        Parameters
        ----------
        value : str
            Credit index identifier.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def side(self, value: Literal["buy_protection", "sell_protection"]) -> CDSTrancheBuilder:
        """
        Set the tranche side (buy/sell protection).

        Parameters
        ----------
        value : {"buy_protection", "sell_protection"}
            Tranche side.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized side.

        """
        ...

    def effective_date(self, value: datetime.date) -> CDSTrancheBuilder:
        """
        Set the effective date for schedule anchoring.

        Parameters
        ----------
        value : datetime.date
            Effective date. If never set, uses the as-of date (or standard
            IMM-date rolling, if ``standard_imm_dates`` is true).

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def accumulated_loss(self, value: float) -> CDSTrancheBuilder:
        """
        Set the accumulated realized loss.

        Parameters
        ----------
        value : float
            Accumulated realized loss as a fraction of the original
            portfolio notional. Defaults to ``0.0`` when never set
            explicitly.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def standard_imm_dates(self, value: bool) -> CDSTrancheBuilder:
        """
        Set whether to enforce standard IMM dates.

        Parameters
        ----------
        value : bool
            Whether to enforce standard IMM dates (20th of Mar, Jun, Sep,
            Dec). Defaults to ``True`` when never set explicitly.

        Returns
        -------
        CDSTrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`CDSTrancheBuilder.build`.

        """
        ...

    def build(self) -> CDSTranche:
        """
        Build the validated CDS tranche.

        Returns
        -------
        CDSTranche
            The validated CDS tranche.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class ConvertibleBond:
    """
    Typed wrapper for the canonical Rust ``ConvertibleBond`` instrument.

    Build with :meth:`ConvertibleBond.builder`; nested conversion/call/coupon
    terms are set via ``*_json`` setters (JSON sub-fields, per the
    nested-spec rule). Instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> import json
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import ConvertibleBond
    >>> conversion = json.dumps({
    ...     "ratio": 20.0,
    ...     "price": None,
    ...     "policy": "voluntary",
    ...     "anti_dilution": "full_ratchet",
    ...     "dividend_adjustment": "none",
    ...     "dilution_events": [],
    ... })
    >>> bond = (
    ...     ConvertibleBond
    ...     .builder()
    ...     .id("CONV-1")
    ...     .notional(Money(1_000.0, Currency("USD")))
    ...     .issue_date(datetime.date(2024, 1, 15))
    ...     .maturity(datetime.date(2029, 1, 15))
    ...     .discount_curve_id("USD-OIS")
    ...     .conversion_json(conversion)
    ...     .underlying_equity_id("ACME")
    ...     .build()
    ... )
    >>> bond.id
    'CONV-1'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> ConvertibleBondBuilder:
        """
        Create a fluent builder (mirrors Rust ``ConvertibleBond::builder()``).

        Returns
        -------
        ConvertibleBondBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> builder = ConvertibleBond.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> ConvertibleBond:
        """
        Deserialize a validated convertible bond from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"convertible_bond"`` payload. The UTF-8 input must not exceed
            16 MiB. Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        ConvertibleBond
            The validated convertible bond.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails convertible-bond
            validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import ConvertibleBond
        >>> try:
        ...     ConvertibleBond.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`ConvertibleBond.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class ConvertibleBondBuilder:
    """
    Fluent builder returned by :meth:`ConvertibleBond.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import ConvertibleBond
    >>> isinstance(ConvertibleBond.builder(), ConvertibleBond.builder().__class__)
    True
    """

    def id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the convertible bond.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> ConvertibleBondBuilder:
        """
        Set the principal amount.

        Parameters
        ----------
        value : Money
            Principal amount.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def issue_date(self, value: datetime.date) -> ConvertibleBondBuilder:
        """
        Set the convertible bond's issue/dated date.

        Parameters
        ----------
        value : datetime.date
            Issue date.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def maturity(self, value: datetime.date) -> ConvertibleBondBuilder:
        """
        Set the convertible bond's maturity date.

        Parameters
        ----------
        value : datetime.date
            Maturity date.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def discount_curve_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the discount curve identifier for the debt component.

        Parameters
        ----------
        value : str
            Discount curve identifier for the debt component (risk-free or
            funding).

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def credit_curve_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the credit curve identifier for risky discounting (bond floor).

        Parameters
        ----------
        value : str
            Credit curve identifier. If not provided, falls back to
            ``discount_curve_id`` (implies no credit spread). Must represent
            zero-recovery (pure hazard) risky discounting.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def conversion_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the conversion terms from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``ConversionSpec`` object with fields ``ratio``,
            ``price``, ``policy``, ``anti_dilution``, ``dividend_adjustment``
            and ``dilution_events``. At least one of ``ratio`` / ``price``
            must be set. Enum values use canonical snake_case, including
            ``"voluntary"``, ``"full_ratchet"``, and ``"none"``.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``ConversionSpec`` shape.

        """
        ...

    def underlying_equity_id(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the underlying equity identifier.

        Parameters
        ----------
        value : str
            Underlying equity identifier (ticker or instrument id).

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def call_put_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the call/put schedule from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``CallPutSchedule`` object with ``calls`` and
            ``puts`` arrays of call/put windows.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``CallPutSchedule`` shape.

        """
        ...

    def soft_call_trigger_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the soft-call trigger condition from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``SoftCallTrigger`` object with fields
            ``threshold_pct``, ``observation_days`` and
            ``required_days_above``.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``SoftCallTrigger`` shape.

        """
        ...

    def settlement_days(self, value: int) -> ConvertibleBondBuilder:
        """
        Set the convertible bond's settlement lag in business days.

        Parameters
        ----------
        value : int
            Number of business days from trade date to settlement date
            (e.g. ``2`` for US corporate convertibles). If never set,
            settlement is assumed same-day.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def recovery_rate(self, value: float) -> ConvertibleBondBuilder:
        """
        Set the assumed recovery rate on default.

        Parameters
        ----------
        value : float
            Recovery rate as a fraction (e.g. ``0.40`` = 40%). Used in the
            Tsiveriotis-Zhang credit model; only relevant when
            ``credit_curve_id`` is set.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`ConvertibleBondBuilder.build`.

        """
        ...

    def fixed_coupon_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the fixed coupon specification from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``FixedCouponSpec`` object.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``FixedCouponSpec`` shape.

        """
        ...

    def floating_coupon_json(self, value: str) -> ConvertibleBondBuilder:
        """
        Set the floating coupon specification from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``FloatingCouponSpec`` object.

        Returns
        -------
        ConvertibleBondBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``FloatingCouponSpec``
            shape.

        """
        ...

    def build(self) -> ConvertibleBond:
        """
        Build the validated convertible bond.

        Returns
        -------
        ConvertibleBond
            The validated convertible bond.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            neither ``ratio`` nor ``price`` set on the conversion terms).

        """
        ...

class FxForward:
    """
    Typed wrapper for the canonical Rust ``FxForward``.

    Build with :meth:`FxForward.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import FxForward
    >>> forward = (
    ...     FxForward
    ...     .builder()
    ...     .id("EURUSD-FWD-6M")
    ...     .base_currency(Currency("EUR"))
    ...     .quote_currency(Currency("USD"))
    ...     .maturity(datetime.date(2025, 6, 15))
    ...     .notional(Money(1_000_000.0, Currency("EUR")))
    ...     .contract_rate(1.10)
    ...     .domestic_discount_curve_id("USD-OIS")
    ...     .foreign_discount_curve_id("EUR-OIS")
    ...     .build()
    ... )
    >>> forward.id
    'EURUSD-FWD-6M'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> FxForwardBuilder:
        """
        Create a fluent builder (mirrors Rust ``FxForward::builder()``).

        Returns
        -------
        FxForwardBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> builder = FxForward.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> FxForward:
        """
        Deserialize a validated FX forward from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"fx_forward"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        FxForward
            The validated FX forward.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails FX-forward validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxForward
        >>> try:
        ...     FxForward.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`FxForward.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class FxForwardBuilder:
    """
    Fluent builder returned by :meth:`FxForward.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import FxForward
    >>> isinstance(FxForward.builder(), FxForward.builder().__class__)
    True
    """

    def id(self, value: str) -> FxForwardBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the FX forward.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def base_currency(self, value: Currency) -> FxForwardBuilder:
        """
        Set the base currency (foreign currency, numerator of the pair).

        Parameters
        ----------
        value : Currency
            Base (foreign) currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def quote_currency(self, value: Currency) -> FxForwardBuilder:
        """
        Set the quote currency (domestic currency, denominator of the pair).

        Parameters
        ----------
        value : Currency
            Quote (domestic) currency; also the PV currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def maturity(self, value: datetime.date) -> FxForwardBuilder:
        """
        Set the maturity/settlement date.

        Parameters
        ----------
        value : datetime.date
            Maturity/settlement date.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> FxForwardBuilder:
        """
        Set the notional amount in base currency.

        Parameters
        ----------
        value : Money
            Notional amount, denominated in the base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def contract_rate(self, value: float) -> FxForwardBuilder:
        """
        Set the contract forward rate (quote per base).

        If not set, the forward is valued at-market (zero PV at inception).

        Parameters
        ----------
        value : float
            Contract forward rate, quote currency per unit of base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def domestic_discount_curve_id(self, value: str) -> FxForwardBuilder:
        """
        Set the domestic (quote currency) discount curve identifier.

        Parameters
        ----------
        value : str
            Domestic (quote currency) discount curve identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def foreign_discount_curve_id(self, value: str) -> FxForwardBuilder:
        """
        Set the foreign (base currency) discount curve identifier.

        Parameters
        ----------
        value : str
            Foreign (base currency) discount curve identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def spot_rate_override(self, value: float) -> FxForwardBuilder:
        """
        Set an explicit spot rate override (quote per base).

        If not set, the spot rate is sourced from the market's FX matrix.

        Parameters
        ----------
        value : float
            Spot FX rate, quote currency per unit of base currency.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def base_calendar_id(self, value: str) -> FxForwardBuilder:
        """
        Set the base currency calendar identifier for business day adjustment.

        Parameters
        ----------
        value : str
            Base currency holiday calendar identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def quote_calendar_id(self, value: str) -> FxForwardBuilder:
        """
        Set the quote currency calendar identifier for business day adjustment.

        Parameters
        ----------
        value : str
            Quote currency holiday calendar identifier.

        Returns
        -------
        FxForwardBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxForwardBuilder.build`.

        """
        ...

    def build(self) -> FxForward:
        """
        Build the validated FX forward.

        Returns
        -------
        FxForward
            The validated FX forward.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            ``base_currency`` equals ``quote_currency``).

        """
        ...

class FxOption:
    """
    Typed wrapper for the canonical Rust ``FxOption``.

    Build with :meth:`FxOption.builder`; instances are accepted directly by
    :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import FxOption
    >>> option = (
    ...     FxOption
    ...     .builder()
    ...     .id("EURUSD-CALL-1Y")
    ...     .base_currency(Currency("EUR"))
    ...     .quote_currency(Currency("USD"))
    ...     .strike(1.12)
    ...     .option_type("call")
    ...     .expiry(datetime.date(2025, 12, 15))
    ...     .notional(Money(1_000_000.0, Currency("EUR")))
    ...     .domestic_discount_curve_id("USD-OIS")
    ...     .foreign_discount_curve_id("EUR-OIS")
    ...     .vol_surface_id("EURUSD-VOL")
    ...     .build()
    ... )
    >>> option.id
    'EURUSD-CALL-1Y'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> FxOptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``FxOption::builder()``).

        Returns
        -------
        FxOptionBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> builder = FxOption.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> FxOption:
        """
        Deserialize a validated FX option from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"fx_option"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        FxOption
            The validated FX option.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails FX-option validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import FxOption
        >>> try:
        ...     FxOption.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`FxOption.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class FxOptionBuilder:
    """
    Fluent builder returned by :meth:`FxOption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import FxOption
    >>> isinstance(FxOption.builder(), FxOption.builder().__class__)
    True
    """

    def id(self, value: str) -> FxOptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the FX option.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def base_currency(self, value: Currency) -> FxOptionBuilder:
        """
        Set the base currency (foreign currency).

        Parameters
        ----------
        value : Currency
            Base (foreign) currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def quote_currency(self, value: Currency) -> FxOptionBuilder:
        """
        Set the quote currency (domestic currency).

        Parameters
        ----------
        value : Currency
            Quote (domestic) currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def strike(self, value: float) -> FxOptionBuilder:
        """
        Set the strike exchange rate (quote per base).

        Parameters
        ----------
        value : float
            Strike exchange rate, quote currency per unit of base currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> FxOptionBuilder:
        """
        Set the option type: ``"call"`` or ``"put"`` on base currency.

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the FX option.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        """
        ...

    def exercise_style(self, value: Literal["european", "american", "bermudan"]) -> FxOptionBuilder:
        """
        Set whether exercise is European, American, or Bermudan.

        Parameters
        ----------
        value : {"european", "american", "bermudan"}
            Exercise style of the FX option. Only ``"european"`` is
            currently priceable; ``"american"`` and ``"bermudan"`` are
            accepted here but rejected with a ``ValueError`` at pricing
            time (specialized pricers are not yet implemented).

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        """
        ...

    def expiry(self, value: datetime.date) -> FxOptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> FxOptionBuilder:
        """
        Set the notional amount in base currency.

        Parameters
        ----------
        value : Money
            Notional amount, denominated in the base currency.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def domestic_discount_curve_id(self, value: str) -> FxOptionBuilder:
        """
        Set the domestic currency discount curve identifier.

        Parameters
        ----------
        value : str
            Domestic currency discount curve identifier.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def foreign_discount_curve_id(self, value: str) -> FxOptionBuilder:
        """
        Set the foreign currency discount curve identifier.

        Parameters
        ----------
        value : str
            Foreign currency discount curve identifier.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def vol_surface_id(self, value: str) -> FxOptionBuilder:
        """
        Set the FX volatility surface identifier.

        Parameters
        ----------
        value : str
            FX volatility surface identifier for option pricing.

        Returns
        -------
        FxOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`FxOptionBuilder.build`.

        """
        ...

    def build(self) -> FxOption:
        """
        Build the validated FX option.

        Returns
        -------
        FxOption
            The validated FX option.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails (e.g.
            ``base_currency`` equals ``quote_currency``).

        """
        ...

class EquityOption:
    """
    Typed wrapper for the canonical Rust ``EquityOption`` instrument.

    Build with :meth:`EquityOption.builder`; instances are accepted directly
    by :func:`price_instrument`.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import EquityOption
    >>> option = (
    ...     EquityOption
    ...     .builder()
    ...     .id("AAPL-C-200")
    ...     .underlying_ticker("AAPL")
    ...     .strike(200.0)
    ...     .option_type("call")
    ...     .expiry(datetime.date(2025, 6, 20))
    ...     .notional(Money(100.0, Currency("USD")))
    ...     .discount_curve_id("USD-OIS")
    ...     .spot_id("AAPL")
    ...     .vol_surface_id("AAPL-VOL")
    ...     .build()
    ... )
    >>> option.id
    'AAPL-C-200'
    """

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @staticmethod
    def builder() -> EquityOptionBuilder:
        """
        Create a fluent builder (mirrors Rust ``EquityOption::builder()``).

        Returns
        -------
        EquityOptionBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> builder = EquityOption.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @classmethod
    def from_json(cls, json: str) -> EquityOption:
        """
        Deserialize a validated equity option from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"equity_option"`` payload. The UTF-8 input must not exceed 16 MiB.
            Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        EquityOption
            The validated equity option.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails equity-option validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import EquityOption
        >>> try:
        ...     EquityOption.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`EquityOption.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

class EquityOptionBuilder:
    """
    Fluent builder returned by :meth:`EquityOption.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import EquityOption
    >>> isinstance(EquityOption.builder(), EquityOption.builder().__class__)
    True
    """

    def id(self, value: str) -> EquityOptionBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the equity option.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def underlying_ticker(self, value: str) -> EquityOptionBuilder:
        """
        Set the underlying equity ticker symbol.

        Parameters
        ----------
        value : str
            Underlying equity ticker symbol.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def strike(self, value: float) -> EquityOptionBuilder:
        """
        Set the option strike in the underlying's price units.

        Parameters
        ----------
        value : float
            Strike price. Must be finite and positive.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def option_type(self, value: Literal["call", "put"]) -> EquityOptionBuilder:
        """
        Set whether this contract is a call, put, cap, or floor.

        Parameters
        ----------
        value : {"call", "put"}
            Option type of the equity option.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized option type.

        """
        ...

    def exercise_style(self, value: Literal["european", "american", "bermudan"]) -> EquityOptionBuilder:
        """
        Set whether exercise is European, American, or Bermudan.

        Parameters
        ----------
        value : {"european", "american", "bermudan"}
            Exercise style of the equity option. Defaults to ``"european"``
            when never set.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized exercise style.

        """
        ...

    def expiry(self, value: datetime.date) -> EquityOptionBuilder:
        """
        Set the option expiry date.

        Parameters
        ----------
        value : datetime.date
            Option expiry date.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def notional(self, value: Money) -> EquityOptionBuilder:
        """
        Set the notional amount for valuation scaling.

        Parameters
        ----------
        value : Money
            Notional amount for valuation scaling.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def discount_curve_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the discount curve identifier for present value calculations.

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def spot_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the equity spot price identifier.

        Parameters
        ----------
        value : str
            Equity spot price identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def vol_surface_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the equity volatility surface identifier.

        Parameters
        ----------
        value : str
            Equity volatility surface identifier.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def div_yield_id(self, value: str) -> EquityOptionBuilder:
        """
        Set the continuous dividend yield identifier.

        Parameters
        ----------
        value : str
            Continuous dividend yield identifier. If never set, the pricer
            treats the underlying as having zero continuous dividend yield.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def discrete_dividends(self, value: list[tuple[datetime.date, float]]) -> EquityOptionBuilder:
        """
        Set the discrete dividend schedule.

        Parameters
        ----------
        value : list[tuple[datetime.date, float]]
            Discrete dividend schedule as ``(ex_date, dividend_amount)``
            pairs. When provided, the escrowed dividend model is used for
            pricing.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def exercise_schedule(self, value: list[datetime.date]) -> EquityOptionBuilder:
        """
        Set the exercise schedule for Bermudan options.

        Parameters
        ----------
        value : list[datetime.date]
            Dates on which early exercise is permitted. Required when
            ``exercise_style`` is ``"bermudan"``.

        Returns
        -------
        EquityOptionBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`EquityOptionBuilder.build`.

        """
        ...

    def build(self) -> EquityOption:
        """
        Build the validated equity option.

        Returns
        -------
        EquityOption
            The validated equity option.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.

        """
        ...

class RepLine:
    """
    Aggregated representative line for pool modeling.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.dates import DayCount
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import RepLine
    >>> line = RepLine(
    ...     "LINE-1",
    ...     Money(80_000_000.0, Currency("USD")),
    ...     0.07,
    ...     datetime.date(2031, 1, 15),
    ...     12,
    ...     DayCount.ACT_360,
    ...     cpr=0.10,
    ...     cdr=0.02,
    ...     recovery_rate=0.45,
    ... )
    >>> "LINE-1" in repr(line)
    True
    """

    def __init__(
        self,
        id: str,
        balance: Money,
        rate: float,
        maturity: datetime.date,
        seasoning_months: int,
        day_count: DayCount,
        *,
        spread_bp: float | None = None,
        index_id: str | None = None,
        cpr: float | None = None,
        cdr: float | None = None,
        recovery_rate: float | None = None,
    ) -> None:
        """
        Aggregated representative line for pool modeling.

        Parameters
        ----------
        id : str
            Unique identifier for the rep line.
        balance : Money
            Aggregated balance.
        rate : float
            Weighted average coupon as an annual decimal rate (e.g. ``0.07``
            = 7%).
        maturity : datetime.date
            Weighted average maturity date.
        seasoning_months : int
            Weighted average seasoning in months.
        day_count : DayCount
            Day count convention.
        spread_bp : float, optional
            Weighted average spread over the reference index, in basis
            points (e.g. ``150.0`` = 150bp), for floating-rate lines.
        index_id : str, optional
            Reference index identifier, if floating.
        cpr : float, optional
            Constant prepayment rate override, as an annual decimal (e.g.
            ``0.10`` = 10% CPR).
        cdr : float, optional
            Constant default rate override, as an annual decimal (e.g.
            ``0.02`` = 2% CDR).
        recovery_rate : float, optional
            Recovery rate override, as a decimal fraction (e.g. ``0.45`` =
            45%).

        Returns
        -------
        RepLine
            The rep line.

        Raises
        ------
        TypeError
            If ``maturity`` is not a date-like object (``datetime.date``,
            ``datetime.datetime``, or ``pandas.Timestamp``).

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import DayCount
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import RepLine
        >>> line = RepLine(
        ...     "LINE-1",
        ...     Money(80_000_000.0, Currency("USD")),
        ...     0.07,
        ...     datetime.date(2031, 1, 15),
        ...     12,
        ...     DayCount.ACT_360,
        ...     cpr=0.10,
        ...     cdr=0.02,
        ...     recovery_rate=0.45,
        ... )
        >>> "LINE-1" in repr(line)
        True
        """
        ...

class AssetPool:
    """
    Structured-credit collateral pool.

    Examples
    --------
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.valuations.instruments import AssetPool
    >>> pool = AssetPool("POOL-1", "abs", Currency("USD"))
    >>> "POOL-1" in repr(pool)
    True
    """

    def __init__(
        self,
        id: str,
        deal_type: Literal["clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"],
        base_currency: Currency,
    ) -> None:
        """
        Structured-credit collateral pool.

        Parameters
        ----------
        id : str
            Pool identifier.
        deal_type : {"clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"}
            Deal classification for pool-level assumptions.
        base_currency : Currency
            Base currency for every asset and pool-level account.

        Returns
        -------
        AssetPool
            A new, empty asset pool. Use :meth:`with_rep_lines` and/or
            :meth:`assets_json` to attach collateral.

        Raises
        ------
        ValueError
            If ``deal_type`` is not a recognized deal type.

        Examples
        --------
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.valuations.instruments import AssetPool
        >>> pool = AssetPool("POOL-1", "abs", Currency("USD"))
        >>> "POOL-1" in repr(pool)
        True
        """
        ...

    def with_rep_lines(self, rep_lines: list[RepLine]) -> AssetPool:
        """
        Attach representative pool lines, returning a new pool.

        Parameters
        ----------
        rep_lines : list[RepLine]
            Aggregated representative lines the pricing engine will use
            instead of individual assets.

        Returns
        -------
        AssetPool
            A new pool with ``rep_lines`` set (the original is unchanged).

        Raises
        ------
        TypeError
            If an element of ``rep_lines`` is not a ``RepLine``.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import DayCount
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import AssetPool, RepLine
        >>> pool = AssetPool("POOL-1", "abs", Currency("USD")).with_rep_lines([
        ...     RepLine(
        ...         "LINE-1",
        ...         Money(80_000_000.0, Currency("USD")),
        ...         0.07,
        ...         datetime.date(2031, 1, 15),
        ...         12,
        ...         DayCount.ACT_360,
        ...     )
        ... ])
        >>> "POOL-1" in repr(pool)
        True
        """
        ...

    def assets_json(self, value: str) -> AssetPool:
        """
        Attach loan-level assets from a JSON array, returning a new pool.

        Loan-level ``PoolAsset`` records carry ~30 fields and stay JSON per
        the nested-spec rule; use :meth:`with_rep_lines` for the typed,
        aggregated path.

        Parameters
        ----------
        value : str
            JSON array of ``PoolAsset`` objects.

        Returns
        -------
        AssetPool
            A new pool with ``assets`` set (the original is unchanged).

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``PoolAsset`` list shape.
        """
        ...

class Tranche:
    """
    Structured-credit tranche with attachment/detachment points.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import Tranche
    >>> tranche = (
    ...     Tranche
    ...     .builder()
    ...     .id("A")
    ...     .attachment_point(0.0)
    ...     .detachment_point(100.0)
    ...     .seniority("senior")
    ...     .original_balance(Money(100.0, Currency("USD")))
    ...     .coupon_fixed(0.05)
    ...     .maturity(datetime.date(2029, 1, 1))
    ...     .build()
    ... )
    >>> repr(tranche)
    'Tranche(id="A", seniority=Senior, attachment_point=0, detachment_point=100)'

    """

    @staticmethod
    def builder() -> TrancheBuilder:
        """
        Create a fluent builder (mirrors Rust ``Tranche::builder()``).

        Returns
        -------
        TrancheBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import Tranche
        >>> builder = Tranche.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

class TrancheBuilder:
    """
    Fluent builder returned by :meth:`Tranche.builder`.

    ``attachment_point`` and ``detachment_point`` are tracked separately from
    the wrapped Rust builder (which only exposes a combined
    ``attachment_detachment(a, d)`` setter) and applied together on
    :meth:`build`, so either call order works.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import Tranche
    >>> isinstance(Tranche.builder(), Tranche.builder().__class__)
    True
    """

    def id(self, value: str) -> TrancheBuilder:
        """
        Set the tranche identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the tranche.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def attachment_point(self, value: float) -> TrancheBuilder:
        """
        Set the attachment point.

        Parameters
        ----------
        value : float
            Attachment point quoted in percent on a 0-100 scale (e.g. ``0.0``
            for equity, ``10.0`` for a tranche attaching at 10%).

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def detachment_point(self, value: float) -> TrancheBuilder:
        """
        Set the detachment point.

        Parameters
        ----------
        value : float
            Detachment point quoted in percent on a 0-100 scale (e.g.
            ``100.0`` for the most senior tranche).

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def seniority(self, value: Literal["senior", "mezzanine", "subordinated", "equity"]) -> TrancheBuilder:
        """
        Set the tranche seniority.

        Parameters
        ----------
        value : {"senior", "mezzanine", "subordinated", "equity"}
            Structural seniority of the tranche.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized seniority.
        """
        ...

    def original_balance(self, value: Money) -> TrancheBuilder:
        """
        Set the original tranche balance.

        Maps to the Rust ``TrancheBuilder::balance`` setter; named
        ``original_balance`` here to match the ``Tranche.original_balance``
        field it populates.

        Parameters
        ----------
        value : Money
            Original tranche balance. Must be positive.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def coupon_fixed(self, rate: float) -> TrancheBuilder:
        """
        Set a fixed-rate coupon.

        Parameters
        ----------
        rate : float
            Fixed interest rate as an annual decimal (e.g. ``0.05`` = 5%).

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def coupon_floating_json(self, value: str) -> TrancheBuilder:
        """
        Set a floating-rate coupon from a JSON ``TrancheCoupon::Floating`` payload.

        The floating-rate spec (``FloatingRateSpec``: index, spread, gearing,
        floors/caps, reset conventions) stays JSON per the nested-spec rule —
        the typed cashflows plan owns that shape.

        Parameters
        ----------
        value : str
            JSON-encoded, externally-tagged ``TrancheCoupon`` value, e.g.
            ``{"floating": {...FloatingRateSpec fields...}}``.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``TrancheCoupon`` shape.
        """
        ...

    def maturity(self, value: datetime.date) -> TrancheBuilder:
        """
        Set the legal final maturity date.

        Parameters
        ----------
        value : datetime.date
            Legal final maturity date.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def frequency(self, value: Tenor) -> TrancheBuilder:
        """
        Set the payment frequency.

        Parameters
        ----------
        value : Tenor
            Payment frequency. Defaults to quarterly when never set.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def day_count(self, value: DayCount) -> TrancheBuilder:
        """
        Set the day count convention for interest accrual.

        Parameters
        ----------
        value : DayCount
            Day count convention. Defaults to Act/360 when never set.

        Returns
        -------
        TrancheBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`TrancheBuilder.build`.
        """
        ...

    def build(self) -> Tranche:
        """
        Build the validated tranche.

        Returns
        -------
        Tranche
            The validated tranche.

        Raises
        ------
        ValueError
            If a required field is missing, or attachment/detachment points
            are invalid (negative, out of the ``[0, 100]`` range, or
            detachment not strictly above attachment).
        """
        ...

class TrancheStructure:
    """
    Capital structure formed from a list of tranches.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.valuations.instruments import Tranche
    >>> tranche = (
    ...     Tranche
    ...     .builder()
    ...     .id("A")
    ...     .attachment_point(0.0)
    ...     .detachment_point(100.0)
    ...     .seniority("senior")
    ...     .original_balance(Money(100.0, Currency("USD")))
    ...     .coupon_fixed(0.05)
    ...     .maturity(datetime.date(2029, 1, 1))
    ...     .build()
    ... )
    >>> from finstack_quant.valuations.instruments import TrancheStructure
    >>> "tranches=1" in repr(TrancheStructure([tranche]))
    True

    """

    def __init__(self, tranches: list[Tranche]) -> None:
        """
        Capital structure formed from a list of tranches.

        Validates that attachment/detachment points tile ``[0, 100]`` without
        gaps or overlaps, that every tranche shares one currency, and assigns
        each tranche a distinct, strictly-increasing ``payment_priority``
        ranked by seniority.

        Parameters
        ----------
        tranches : list[Tranche]
            Tranches forming the capital structure.

        Returns
        -------
        TrancheStructure
            The validated tranche structure.

        Raises
        ------
        ValueError
            If ``tranches`` is empty, has non-finite attachment/detachment
            points, leaves a gap/overlap, doesn't tile to 100%, or mixes
            currencies.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import Tranche, TrancheStructure
        >>> senior = (
        ...     Tranche
        ...     .builder()
        ...     .id("A")
        ...     .attachment_point(10.0)
        ...     .detachment_point(100.0)
        ...     .seniority("senior")
        ...     .original_balance(Money(72_000_000.0, Currency("USD")))
        ...     .coupon_fixed(0.05)
        ...     .maturity(datetime.date(2031, 1, 15))
        ...     .build()
        ... )
        >>> equity = (
        ...     Tranche
        ...     .builder()
        ...     .id("E")
        ...     .attachment_point(0.0)
        ...     .detachment_point(10.0)
        ...     .seniority("equity")
        ...     .original_balance(Money(8_000_000.0, Currency("USD")))
        ...     .coupon_fixed(0.0)
        ...     .maturity(datetime.date(2031, 1, 15))
        ...     .build()
        ... )
        >>> structure = TrancheStructure([senior, equity])
        >>> "tranches=2" in repr(structure)
        True
        """
        ...

class StructuredCredit:
    """
    Structured-credit deal (ABS/CLO/CMBS/RMBS) with pool, tranches, and waterfall.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import StructuredCredit
    >>> builder = StructuredCredit.builder()
    >>> builder.id("EXAMPLE") is builder
    True

    """

    @staticmethod
    def builder() -> StructuredCreditBuilder:
        """
        Create a fluent builder (mirrors Rust ``StructuredCredit::builder()``).

        The builder pre-seeds ``market_conditions``, ``credit_factors``,
        ``deal_metadata``, ``behavior_overrides``, ``default_assumptions``,
        and ``hedge_swaps`` with their Rust ``Default`` values (the Rust
        builder fields have no default), which the corresponding ``*_json``
        setters can override. Prefer :meth:`new_abs` / :meth:`new_clo` /
        :meth:`new_cmbs` / :meth:`new_rmbs` for registry-calibrated deal-type
        defaults; use this builder for full manual control.

        Returns
        -------
        StructuredCreditBuilder
            A builder with fluent, consuming setter methods.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import StructuredCredit
        >>> builder = StructuredCredit.builder()
        >>> builder.id("EXAMPLE") is builder
        True
        """
        ...

    @staticmethod
    def new_abs(
        id: str,
        pool: AssetPool,
        tranches: TrancheStructure,
        closing_date: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> StructuredCredit:
        """
        Create a new ABS deal with registry-calibrated defaults.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        pool : AssetPool
            Asset pool definition.
        tranches : TrancheStructure
            Tranche capital structure.
        closing_date : datetime.date
            Deal closing date (issuance).
        maturity : datetime.date
            Legal final maturity date.
        discount_curve_id : str
            Discount curve identifier for valuation.

        Returns
        -------
        StructuredCredit
            The validated ABS deal.

        Raises
        ------
        ValueError
            If the deal fails pricing validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.dates import DayCount
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import (
        ...     AssetPool,
        ...     RepLine,
        ...     StructuredCredit,
        ...     Tranche,
        ...     TrancheStructure,
        ... )
        >>> pool = AssetPool("POOL-1", "abs", Currency("USD")).with_rep_lines([
        ...     RepLine(
        ...         "LINE-1",
        ...         Money(80_000_000.0, Currency("USD")),
        ...         0.07,
        ...         datetime.date(2031, 1, 15),
        ...         12,
        ...         DayCount.ACT_360,
        ...     )
        ... ])
        >>> senior = (
        ...     Tranche
        ...     .builder()
        ...     .id("A")
        ...     .attachment_point(10.0)
        ...     .detachment_point(100.0)
        ...     .seniority("senior")
        ...     .original_balance(Money(72_000_000.0, Currency("USD")))
        ...     .coupon_fixed(0.05)
        ...     .maturity(datetime.date(2031, 1, 15))
        ...     .build()
        ... )
        >>> equity = (
        ...     Tranche
        ...     .builder()
        ...     .id("E")
        ...     .attachment_point(0.0)
        ...     .detachment_point(10.0)
        ...     .seniority("equity")
        ...     .original_balance(Money(8_000_000.0, Currency("USD")))
        ...     .coupon_fixed(0.0)
        ...     .maturity(datetime.date(2031, 1, 15))
        ...     .build()
        ... )
        >>> deal = StructuredCredit.new_abs(
        ...     "ABS-1",
        ...     pool,
        ...     TrancheStructure([senior, equity]),
        ...     datetime.date(2024, 1, 15),
        ...     datetime.date(2031, 1, 15),
        ...     "USD-SOFR-DISC",
        ... )
        >>> "ABS-1" in repr(deal)
        True
        """
        ...

    @staticmethod
    def new_clo(
        id: str,
        pool: AssetPool,
        tranches: TrancheStructure,
        closing_date: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> StructuredCredit:
        """
        Create a new CLO deal with registry-calibrated defaults.

        Same signature as :meth:`new_abs`; only the deal-type calibration
        (prepayment/default/recovery specs, frequency, fees) differs.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        pool : AssetPool
            Asset pool definition.
        tranches : TrancheStructure
            Tranche capital structure.
        closing_date : datetime.date
            Deal closing date (issuance).
        maturity : datetime.date
            Legal final maturity date.
        discount_curve_id : str
            Discount curve identifier for valuation.

        Returns
        -------
        StructuredCredit
            The validated CLO deal.

        Raises
        ------
        ValueError
            If the deal fails pricing validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import Tranche
        >>> tranche = (
        ...     Tranche
        ...     .builder()
        ...     .id("A")
        ...     .attachment_point(0.0)
        ...     .detachment_point(100.0)
        ...     .seniority("senior")
        ...     .original_balance(Money(100.0, Currency("USD")))
        ...     .coupon_fixed(0.05)
        ...     .maturity(datetime.date(2029, 1, 1))
        ...     .build()
        ... )
        >>> from finstack_quant.valuations.instruments import AssetPool, StructuredCredit, TrancheStructure
        >>> pool = AssetPool("P", "clo", Currency("USD"))
        >>> deal = StructuredCredit.new_clo(
        ...     "D", pool, TrancheStructure([tranche]), datetime.date(2024, 1, 1), datetime.date(2029, 1, 1), "USD-OIS"
        ... )
        >>> (deal.id, "Clo" in repr(deal))
        ('D', True)

        """
        ...

    @staticmethod
    def new_cmbs(
        id: str,
        pool: AssetPool,
        tranches: TrancheStructure,
        closing_date: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> StructuredCredit:
        """
        Create a new CMBS deal with registry-calibrated defaults.

        Same signature as :meth:`new_abs`; only the deal-type calibration
        (prepayment/default/recovery specs, frequency, fees) differs.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        pool : AssetPool
            Asset pool definition.
        tranches : TrancheStructure
            Tranche capital structure.
        closing_date : datetime.date
            Deal closing date (issuance).
        maturity : datetime.date
            Legal final maturity date.
        discount_curve_id : str
            Discount curve identifier for valuation.

        Returns
        -------
        StructuredCredit
            The validated CMBS deal.

        Raises
        ------
        ValueError
            If the deal fails pricing validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import Tranche
        >>> tranche = (
        ...     Tranche
        ...     .builder()
        ...     .id("A")
        ...     .attachment_point(0.0)
        ...     .detachment_point(100.0)
        ...     .seniority("senior")
        ...     .original_balance(Money(100.0, Currency("USD")))
        ...     .coupon_fixed(0.05)
        ...     .maturity(datetime.date(2029, 1, 1))
        ...     .build()
        ... )
        >>> from finstack_quant.valuations.instruments import AssetPool, StructuredCredit, TrancheStructure
        >>> pool = AssetPool("P", "cmbs", Currency("USD"))
        >>> deal = StructuredCredit.new_cmbs(
        ...     "D", pool, TrancheStructure([tranche]), datetime.date(2024, 1, 1), datetime.date(2029, 1, 1), "USD-OIS"
        ... )
        >>> (deal.id, "Cmbs" in repr(deal))
        ('D', True)

        """
        ...

    @staticmethod
    def new_rmbs(
        id: str,
        pool: AssetPool,
        tranches: TrancheStructure,
        closing_date: datetime.date,
        maturity: datetime.date,
        discount_curve_id: str,
    ) -> StructuredCredit:
        """
        Create a new RMBS deal with registry-calibrated defaults.

        Same signature as :meth:`new_abs`; only the deal-type calibration
        (prepayment/default/recovery specs, frequency, fees) differs.

        Parameters
        ----------
        id : str
            Unique instrument identifier.
        pool : AssetPool
            Asset pool definition.
        tranches : TrancheStructure
            Tranche capital structure.
        closing_date : datetime.date
            Deal closing date (issuance).
        maturity : datetime.date
            Legal final maturity date.
        discount_curve_id : str
            Discount curve identifier for valuation.

        Returns
        -------
        StructuredCredit
            The validated RMBS deal.

        Raises
        ------
        ValueError
            If the deal fails pricing validation.

        Examples
        --------
        >>> import datetime
        >>> from finstack_quant.core.currency import Currency
        >>> from finstack_quant.core.money import Money
        >>> from finstack_quant.valuations.instruments import Tranche
        >>> tranche = (
        ...     Tranche
        ...     .builder()
        ...     .id("A")
        ...     .attachment_point(0.0)
        ...     .detachment_point(100.0)
        ...     .seniority("senior")
        ...     .original_balance(Money(100.0, Currency("USD")))
        ...     .coupon_fixed(0.05)
        ...     .maturity(datetime.date(2029, 1, 1))
        ...     .build()
        ... )
        >>> from finstack_quant.valuations.instruments import AssetPool, StructuredCredit, TrancheStructure
        >>> pool = AssetPool("P", "rmbs", Currency("USD"))
        >>> deal = StructuredCredit.new_rmbs(
        ...     "D", pool, TrancheStructure([tranche]), datetime.date(2024, 1, 1), datetime.date(2029, 1, 1), "USD-OIS"
        ... )
        >>> (deal.id, "Rmbs" in repr(deal))
        ('D', True)

        """
        ...

    @classmethod
    def from_json(cls, json: str) -> StructuredCredit:
        """
        Deserialize a validated deal from its canonical v1 envelope.

        Parameters
        ----------
        json : str
            A ``finstack_quant.instrument/1`` envelope containing an exact
            ``"structured_credit"`` payload. The UTF-8 input must not exceed
            16 MiB. Bare payloads and cross-type coercion are rejected.

        Returns
        -------
        StructuredCredit
            The validated deal.

        Raises
        ------
        ValueError
            If input exceeds 16 MiB, is malformed, uses an unsupported
            envelope schema, carries another type, or fails structured-credit
            validation.

        Examples
        --------
        >>> from finstack_quant.valuations.instruments import StructuredCredit
        >>> try:
        ...     StructuredCredit.from_json("{}")
        ... except ValueError as exc:
        ...     print("schema" in str(exc))
        True

        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a canonical ``finstack_quant.instrument/1`` envelope.

        Returns
        -------
        str
            Canonical instrument envelope accepted by :func:`price_instrument`
            and :meth:`StructuredCredit.from_json`.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def id(self) -> str:
        """
        Stable instrument identifier used in market lookup and results.

        Returns
        -------
        str
            The unique instrument identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class StructuredCreditBuilder:
    """
    Fluent builder returned by :meth:`StructuredCredit.builder`.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import StructuredCredit
    >>> isinstance(StructuredCredit.builder(), StructuredCredit.builder().__class__)
    True
    """

    def id(self, value: str) -> StructuredCreditBuilder:
        """
        Set the instrument identifier.

        Parameters
        ----------
        value : str
            Unique identifier for the deal.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def deal_type(self, value: Literal["clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"]) -> StructuredCreditBuilder:
        """
        Set the deal-type classification.

        Parameters
        ----------
        value : {"clo", "cbo", "abs", "rmbs", "cmbs", "auto", "card"}
            Deal classification.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized deal type.
        """
        ...

    def pool(self, value: AssetPool) -> StructuredCreditBuilder:
        """
        Set the structured-credit asset pool backing the deal.

        Parameters
        ----------
        value : AssetPool
            Asset pool definition.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def tranches(self, value: TrancheStructure) -> StructuredCreditBuilder:
        """
        Set the tranche capital structure.

        Parameters
        ----------
        value : TrancheStructure
            Tranche capital structure.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def closing_date(self, value: datetime.date) -> StructuredCreditBuilder:
        """
        Set the deal closing (issuance) date.

        Parameters
        ----------
        value : datetime.date
            Deal closing date.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def first_payment_date(self, value: datetime.date) -> StructuredCreditBuilder:
        """
        Set the first payment date to tranches.

        Parameters
        ----------
        value : datetime.date
            First payment date.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def reinvestment_end_date(self, value: datetime.date) -> StructuredCreditBuilder:
        """
        Set the end of the reinvestment period.

        Parameters
        ----------
        value : datetime.date
            End date of the reinvestment period. Optional; when never set,
            the deal has no reinvestment period.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def maturity(self, value: datetime.date) -> StructuredCreditBuilder:
        """
        Set the legal final maturity date.

        Parameters
        ----------
        value : datetime.date
            Legal final maturity date.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def frequency(self, value: Tenor) -> StructuredCreditBuilder:
        """
        Set the payment frequency for the structure.

        Parameters
        ----------
        value : Tenor
            Payment frequency.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def payment_calendar_id(self, value: str) -> StructuredCreditBuilder:
        """
        Set the payment calendar identifier for schedule adjustments.

        Parameters
        ----------
        value : str
            Holiday calendar identifier (e.g. ``"nyse"``). Required for
            accurate schedule generation.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def payment_business_day_convention(self, value: str) -> StructuredCreditBuilder:
        """
        Set the business day convention for tranche payments.

        Parameters
        ----------
        value : str
            Business day convention (e.g. ``"following"``,
            ``"modified_following"``). Defaults to ``"following"`` when
            never set.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not a recognized business day convention.
        """
        ...

    def discount_curve_id(self, value: str) -> StructuredCreditBuilder:
        """
        Set the discount curve identifier for valuation.

        Parameters
        ----------
        value : str
            Discount curve identifier.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If this builder was already consumed by a prior call to
            :meth:`StructuredCreditBuilder.build`.
        """
        ...

    def market_conditions_json(self, value: str) -> StructuredCreditBuilder:
        """
        Set market conditions from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``MarketConditions`` object (refinancing rate, home
            price appreciation, unemployment, seasonal factor, custom
            factors). :meth:`StructuredCredit.builder` pre-seeds the registry
            default, which this overrides.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``MarketConditions`` shape.
        """
        ...

    def credit_factors_json(self, value: str) -> StructuredCreditBuilder:
        """
        Set credit factors from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``CreditFactors`` object (credit score, DTI, LTV,
            delinquency, unemployment, CMBS NOI/debt-service, custom
            factors). :meth:`StructuredCredit.builder` pre-seeds
            ``CreditFactors``'s default, which this overrides.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``CreditFactors`` shape.
        """
        ...

    def waterfall_rules_json(self, value: str) -> StructuredCreditBuilder:
        """
        Set declarative waterfall rules from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``WaterfallRules`` object (available-funds caps,
            step-down, shifting interest, controlled accumulation), layered
            onto the base waterfall.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``WaterfallRules`` shape.
        """
        ...

    def fees_json(self, value: str) -> StructuredCreditBuilder:
        """
        Set senior transaction fees from a JSON object.

        Parameters
        ----------
        value : str
            JSON-encoded ``DealFees`` object (trustee, senior management,
            servicing, and optional master/special servicer fees), paid
            ahead of every note. Skipped (``None``) by default.

        Returns
        -------
        StructuredCreditBuilder
            ``self``, for chaining.

        Raises
        ------
        ValueError
            If ``value`` is not valid JSON for the ``DealFees`` shape.
        """
        ...

    def build(self) -> StructuredCredit:
        """
        Build the validated structured-credit deal.

        Returns
        -------
        StructuredCredit
            The validated deal.

        Raises
        ------
        ValueError
            If a required field is missing or Rust validation fails.
        """
        ...

def bond_from_cashflows_json(
    instrument_id: str,
    schedule_json: str,
    discount_curve_id: str,
    quoted_clean: float | None = None,
) -> str:
    """
    Construct tagged bond instrument JSON from a cashflow schedule.

    Parameters
    ----------
    instrument_id : str
        Identifier for the bond instrument.
    schedule_json : str
        JSON-encoded ``CashFlowSchedule``.
    discount_curve_id : str
        Discount curve ID required for pricing.
    quoted_clean : float, optional
        Clean quoted price as a percent of par.

    Returns
    -------
    str
        Canonical ``finstack_quant.instrument/1`` envelope containing the bond.

    Raises
    ------
    ValueError
        If the schedule is invalid or bond construction fails.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import bond_from_cashflows_json
    >>> try:
    ...     bond_from_cashflows_json("B", "{}", "USD-OIS")
    ... except ValueError as exc:
    ...     print("flows" in str(exc))
    True

    """
    ...

def validate_instrument_json(json: str) -> str:
    """
    Validate a canonical instrument envelope and return canonical JSON.

    Parameters
    ----------
    json : str
        A ``finstack_quant.instrument/1`` envelope. Bare instrument payloads
        are rejected.

    Returns
    -------
    str
        Canonical pretty-printed instrument JSON after Rust serde validation.

    Raises
    ------
    ValueError
        If the JSON is malformed, has an unknown instrument tag, or
        fails instrument-specific validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.valuations.instruments import TermLoan, validate_instrument_json
    >>> validated = json.loads(validate_instrument_json(TermLoan.example().to_json()))
    >>> validated["instrument"]["type"]
    'term_loan'

    """
    ...

def validate_typed_instrument_json(type_tag: str, json: str) -> str:
    """
    Validate a payload as one exact instrument type and return the envelope.

    Parameters
    ----------
    type_tag : str
        Canonical instrument discriminator, such as ``"term_loan"`` or
        ``"fx_forward"``.
    json : str
        A ``finstack_quant.instrument/1`` envelope whose instrument type must
        match *type_tag*.

    Returns
    -------
    str
        Canonical instrument envelope for the validated instrument.

    Raises
    ------
    ValueError
        If ``json`` is malformed, carries a different instrument type, or
        fails instrument validation.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.valuations.instruments import TermLoan, validate_typed_instrument_json
    >>> envelope = TermLoan.example().to_json()
    >>> json.loads(validate_typed_instrument_json("term_loan", envelope))["instrument"]["type"]
    'term_loan'

    """
    ...

def pretty_instrument_json(json: str) -> str:
    """
    Re-render a canonical instrument envelope as pretty-printed JSON.

    Parameters
    ----------
    json : str
        A canonical ``finstack_quant.instrument/1`` envelope.

    Returns
    -------
    str
        The same envelope, pretty-printed.

    Raises
    ------
    ValueError
        If ``json`` is malformed or cannot be rendered.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import TermLoan, pretty_instrument_json
    >>> "term_loan" in pretty_instrument_json(TermLoan.example().to_json())
    True

    """
    ...

def price_instrument(
    instrument_json: str
    | Bond
    | TermLoan
    | InterestRateSwap
    | Swaption
    | CapFloor
    | CreditDefaultSwap
    | CDSIndex
    | FxForward
    | FxOption
    | CDSTranche
    | ConvertibleBond
    | EquityOption
    | StructuredCredit,
    market: MarketContext | str,
    as_of: datetime.date | str,
    model: str = "default",
) -> ValuationResult:
    """
    Price one instrument and return a typed :class:`ValuationResult`.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex or FxForward or FxOption or CDSTranche or ConvertibleBond or EquityOption or StructuredCredit
        Canonical ``finstack_quant.instrument/1`` envelope accepted by
        :func:`validate_instrument_json`, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` / :class:`FxForward` / :class:`FxOption` /
        :class:`CDSTranche` / :class:`ConvertibleBond` /
        :class:`EquityOption` / :class:`StructuredCredit` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    model : str, default "default"
        Pricing model selector. Common values include ``"default"``,
        ``"discounting"``, ``"hazard_rate"``, and option-model keys such
        as ``"black76"`` where supported by the instrument.

    Returns
    -------
    ValuationResult
        Typed valuation envelope containing value, currency, metrics, and
        covenant flags when applicable.

    Raises
    ------
    ValueError
        If any input JSON is malformed, required market data is
        missing, or the selected model is unsupported for the instrument.

    Notes
    -----
    The wire payload is still one call away: ``result.to_json()`` returns the
    JSON that :meth:`ValuationResult.from_json` accepts, for pipelines that
    serialize results.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
    >>> from finstack_quant.valuations.instruments import TermLoan
    >>> loan = TermLoan.example()
    >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", datetime.date(2024, 1, 1), 0.04))
    >>> from finstack_quant.valuations.instruments import price_instrument
    >>> result = price_instrument(loan, market, "2024-01-01")
    >>> (result.instrument_id, round(result.price, 2), result.currency)
    ('TERM-LOAN-USD-5Y', 10727162.26, 'USD')

    """
    ...

def price_instrument_with_metrics(
    instrument_json: str
    | Bond
    | TermLoan
    | InterestRateSwap
    | Swaption
    | CapFloor
    | CreditDefaultSwap
    | CDSIndex
    | FxForward
    | FxOption
    | CDSTranche
    | ConvertibleBond
    | EquityOption
    | StructuredCredit,
    market: MarketContext | str,
    as_of: datetime.date | str,
    model: str = "default",
    metrics: list[str] = [],
    pricing_options: str | None = None,
    market_history: str | None = None,
) -> ValuationResult:
    """
    Price one instrument and compute explicit risk metric requests.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex or FxForward or FxOption or CDSTranche or ConvertibleBond or EquityOption or StructuredCredit
        Canonical ``finstack_quant.instrument/1`` envelope, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` / :class:`FxForward` / :class:`FxOption` /
        :class:`CDSTranche` / :class:`ConvertibleBond` /
        :class:`EquityOption` / :class:`StructuredCredit` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    model : str, default "default"
        Pricing model selector.
    metrics : list[str], default []
        Metric IDs to compute, such as ``"ytm"``, ``"dv01"``,
        ``"modified_duration"``, ``"hvar"``, or ``"expected_shortfall"``
        when supported by the instrument.
    pricing_options : str, optional
        Optional JSON string for metric pricing overrides.
    market_history : str, optional
        Optional JSON market-history payload required by
        historical risk metrics.

    Returns
    -------
    ValuationResult
        Typed valuation envelope including the requested metric values.

    Raises
    ------
    ValueError
        If a metric is unknown, not applicable, or cannot be
        calculated from the supplied market and history inputs.

    Notes
    -----
    The wire payload is still one call away: ``result.to_json()`` returns the
    JSON that :meth:`ValuationResult.from_json` accepts, for pipelines that
    serialize results.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
    >>> from finstack_quant.valuations.instruments import TermLoan
    >>> loan = TermLoan.example()
    >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", datetime.date(2024, 1, 1), 0.04))
    >>> from finstack_quant.valuations.instruments import price_instrument_with_metrics
    >>> result = price_instrument_with_metrics(loan, market, "2024-01-01", metrics=["all_in_rate"])
    >>> (result.metric_keys(), round(result.get_metric("all_in_rate"), 4))
    (['all_in_rate'], 0.06)

    """
    ...

def instrument_cashflows_json(
    instrument_json: str
    | Bond
    | TermLoan
    | InterestRateSwap
    | Swaption
    | CapFloor
    | CreditDefaultSwap
    | CDSIndex
    | FxForward
    | FxOption
    | CDSTranche
    | ConvertibleBond
    | EquityOption
    | StructuredCredit,
    market: MarketContext | str,
    as_of: datetime.date | str,
    model: str,
) -> str:
    """
    Per-flow cashflow envelope for a discountable instrument.

    Parameters
    ----------
    instrument_json : str or Bond or TermLoan or InterestRateSwap or Swaption or CapFloor or CreditDefaultSwap or CDSIndex or FxForward or FxOption or CDSTranche or ConvertibleBond or EquityOption or StructuredCredit
        Canonical ``finstack_quant.instrument/1`` envelope, or a typed :class:`Bond` /
        :class:`TermLoan` / :class:`InterestRateSwap` / :class:`Swaption` /
        :class:`CapFloor` / :class:`CreditDefaultSwap` /
        :class:`CDSIndex` / :class:`FxForward` / :class:`FxOption` /
        :class:`CDSTranche` / :class:`ConvertibleBond` /
        :class:`EquityOption` / :class:`StructuredCredit` instance.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    model : str
        Must be ``"discounting"`` or ``"hazard_rate"``. ``"default"`` is not
        accepted on cashflow export.

    Returns
    -------
    str
        JSON-serialized ``InstrumentCashflowEnvelope``.

    Raises
    ------
    ValueError
        If the model is unsupported, the instrument is unsupported
        for cashflow export, or required market data is missing.

    Examples
    --------
    >>> import datetime
    >>> from finstack_quant.core.currency import Currency
    >>> from finstack_quant.core.market_data import DiscountCurve, MarketContext
    >>> from finstack_quant.core.money import Money
    >>> from finstack_quant.core.types import Rate
    >>> from finstack_quant.valuations.instruments import Bond
    >>> as_of = datetime.date(2024, 1, 1)
    >>> bond = Bond.fixed("B", Money(1000.0, Currency("USD")), Rate(0.05), as_of, datetime.date(2026, 1, 1), "USD-OIS")
    >>> market = MarketContext().insert(DiscountCurve.flat("USD-OIS", as_of, 0.04))
    >>> import json
    >>> from finstack_quant.valuations.instruments import instrument_cashflows_json
    >>> payload = json.loads(instrument_cashflows_json(bond, market, "2024-01-01", "discounting"))
    >>> (payload["instrument_id"], len(payload["flows"]))
    ('B', 6)

    """
    ...

def list_models() -> list[str]:
    """
    Return every pricing model key registered in the standard pricer registry.

    The list is registry-derived rather than enum-derived, so it reflects real
    dispatch coverage: a model with no registered pricer is omitted. The names
    are the canonical keys accepted by the ``model`` argument of
    :func:`price_instrument`.

    Returns
    -------
    list[str]
        Canonical model keys such as ``"discounting"`` or ``"black76"``,
        deduplicated and sorted.

    Notes
    -----
    This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_models
    >>> models = list_models()
    >>> (len(models), "discounting" in models, "black76" in models)
    (29, True, True)
    """
    ...

def list_models_grouped() -> dict[str, list[str]]:
    """
    Return the standard registry's pricing models grouped by instrument type.

    Only instrument types with at least one registered pricer appear as keys,
    and each entry lists only the models that can price that instrument.

    Returns
    -------
    dict[str, list[str]]
        Mapping from canonical instrument-type name to its sorted model keys.

    Notes
    -----
    This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_models_grouped
    >>> grouped = list_models_grouped()
    >>> ("bond" in grouped, "discounting" in grouped["bond"])
    (True, True)
    """
    ...

def list_standard_metrics() -> list[str]:
    """
    Return all standard metric IDs registered by the Rust valuation engine.

    Returns
    -------
    list[str]
        Sorted list of fully qualified metric keys.

    Notes
    -----
    This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_standard_metrics
    >>> metrics = list_standard_metrics()
    >>> (len(metrics), "dirty_price" in metrics, "dv01" in metrics)
    (214, True, True)
    """
    ...

def list_standard_metrics_grouped() -> dict[str, list[str]]:
    """
    Return standard metric IDs grouped by human-readable category.

    Returns
    -------
    dict[str, list[str]]
        Mapping from group label to sorted metric ID lists.

    Notes
    -----
    This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import list_standard_metrics_grouped
    >>> grouped = list_standard_metrics_grouped()
    >>> ("Credit" in grouped, "Rates" in grouped)
    (True, True)
    """
    ...

class OasResult:
    """
    Result of an option-adjusted-spread calculation for a structured-credit
    tranche (``structured_credit_tranche_oas``'s return value).

    Examples
    --------
    >>> import json
    >>> from finstack_quant.valuations.instruments import OasResult
    >>> payload = json.dumps({
    ...     "oas": 0.0125,
    ...     "model_price": 99.5,
    ...     "market_price": 98.75,
    ...     "num_paths": 256,
    ...     "price_std_error": 0.05,
    ... })
    >>> result = OasResult.from_json(payload)
    >>> result.oas
    0.0125
    """

    @staticmethod
    def from_json(json: str) -> OasResult:
        """
        Deserialize from the JSON produced by ``to_json``.

        Parameters
        ----------
        json : str
            JSON-encoded ``OasResult``.

        Returns
        -------
        OasResult
            The decoded result.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for the ``OasResult`` shape.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.valuations.instruments import OasResult
        >>> result = OasResult.from_json(
        ...     json.dumps({
        ...         "oas": 0.0125,
        ...         "model_price": 99.5,
        ...         "market_price": 98.75,
        ...         "num_paths": 256,
        ...         "price_std_error": 0.05,
        ...     })
        ... )
        >>> (result.oas, result.model_price, result.num_paths)
        (0.0125, 99.5, 256)

        """
        ...

    def to_json(self) -> str:
        """
        Serialize back to the same JSON shape ``from_json`` accepts.

        Returns
        -------
        str
            JSON-encoded ``OasResult``.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def oas(self) -> float:
        """
        Option-adjusted spread, as an annual decimal (``0.01`` = 100 bp).

        Returns
        -------
        float
            The option-adjusted spread.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def model_price(self) -> float:
        """
        Model price at the solved OAS, as a percentage of original balance.

        Returns
        -------
        float
            The model price.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def market_price(self) -> float:
        """
        Target market price, as a percentage of original balance.

        Returns
        -------
        float
            The target market price.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def num_paths(self) -> int:
        """
        Number of Monte-Carlo scenarios used.

        Returns
        -------
        int
            The scenario count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def price_std_error(self) -> float:
        """
        Monte-Carlo standard error of the mean price, as a percentage of
        original balance.

        Returns
        -------
        float
            The standard error.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a single-row pandas DataFrame.

        Columns: ``oas`` (annual decimal), ``model_price`` and
        ``market_price`` (percentage of original balance), ``num_paths``,
        ``price_std_error`` (percentage of original balance).

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame of the OAS solve, so a book of tranches
            stacks with ``pd.concat``.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class TrancheMetrics:
    """
    Summary risk/pricing metrics for a structured-credit tranche
    (``structured_credit_tranche_metrics``'s return value).

    Examples
    --------
    >>> import json
    >>> from finstack_quant.valuations.instruments import TrancheMetrics
    >>> payload = json.dumps({
    ...     "tranche_id": "A",
    ...     "currency": "USD",
    ...     "pv": 1000.0,
    ...     "price_pct": 100.0,
    ...     "wal": 3.0,
    ...     "z_spread_bp": 0.0,
    ...     "cs01": -1.0,
    ...     "spread_duration": 3.0,
    ...     "modified_duration": 3.0,
    ...     "convexity": 12.0,
    ...     "target_price_pct": 100.0,
    ... })
    >>> metrics = TrancheMetrics.from_json(payload)
    >>> metrics.tranche_id
    'A'
    """

    @staticmethod
    def from_json(json: str) -> TrancheMetrics:
        """
        Deserialize from the JSON produced by ``to_json``.

        Parameters
        ----------
        json : str
            JSON-encoded ``TrancheMetrics``.

        Returns
        -------
        TrancheMetrics
            The decoded metrics bundle.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for the ``TrancheMetrics`` shape.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.valuations.instruments import TrancheMetrics
        >>> payload = {
        ...     "tranche_id": "A",
        ...     "currency": "USD",
        ...     "pv": 1000.0,
        ...     "price_pct": 100.0,
        ...     "wal": 3.0,
        ...     "z_spread_bp": 0.0,
        ...     "cs01": -1.0,
        ...     "spread_duration": 3.0,
        ...     "modified_duration": 3.0,
        ...     "convexity": 12.0,
        ...     "target_price_pct": 100.0,
        ... }
        >>> metrics = TrancheMetrics.from_json(json.dumps(payload))
        >>> (metrics.tranche_id, metrics.currency, metrics.pv)
        ('A', 'USD', 1000.0)

        """
        ...

    def to_json(self) -> str:
        """
        Serialize back to the same JSON shape ``from_json`` accepts.

        Returns
        -------
        str
            JSON-encoded ``TrancheMetrics``.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def tranche_id(self) -> str:
        """
        Identifier of the tranche.

        Returns
        -------
        str
            The tranche identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def currency(self) -> str:
        """
        ISO-4217 code of the currency ``pv`` and ``cs01`` are denominated in.
        Empty when decoded from a legacy payload that predates this field.

        Returns
        -------
        str
            The ISO-4217 currency code, or an empty string for legacy payloads.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def pv(self) -> float:
        """
        Present value of the tranche, in ``currency`` units.

        Returns
        -------
        float
            The present value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def price_pct(self) -> float:
        """
        Model price, as a percentage of original balance.

        Returns
        -------
        float
            The model price.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def wal(self) -> float:
        """
        Weighted-average life, in years.

        Returns
        -------
        float
            The weighted-average life.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def z_spread_bp(self) -> float:
        """
        Z-spread to ``target_price_pct``, in basis points.

        Returns
        -------
        float
            The z-spread in basis points.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def cs01(self) -> float:
        """
        Credit-spread DV01 -- currency change for a +1 bp z-spread shock, in
        ``currency`` units. Negative for a long tranche.

        Returns
        -------
        float
            The credit-spread DV01.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def spread_duration(self) -> float:
        """
        Spread duration, in years (``-cs01 / (pv * 1bp)``).

        Returns
        -------
        float
            The spread duration.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def modified_duration(self) -> float:
        """
        Modified (rate) duration of the projected cashflows, in years.

        Returns
        -------
        float
            The modified duration.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def convexity(self) -> float:
        """
        Modified convexity of the projected cashflows, in years squared.

        Returns
        -------
        float
            The modified convexity.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def target_price_pct(self) -> float:
        """
        Price the z-spread/CS01 were solved against, as a percentage of
        original balance.

        Returns
        -------
        float
            The target price.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a single-row pandas DataFrame.

        Columns: ``tranche_id``, ``currency``, ``pv``, ``price_pct``, ``wal``,
        ``z_spread_bp``, ``cs01``, ``spread_duration``, ``modified_duration``,
        ``convexity``, ``target_price_pct`` -- the same fields and units as
        the properties of the same name.

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame, so a capital structure stacks with
            ``pd.concat``. ``pv`` and ``cs01`` are in ``currency`` units and
            are only additive across tranches sharing one currency.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class ScenarioTable:
    """
    Scenario/yield table for a single structured-credit tranche
    (``structured_credit_tranche_scenario_table``'s return value).

    Examples
    --------
    >>> import json
    >>> from finstack_quant.valuations.instruments import ScenarioTable
    >>> payload = json.dumps({
    ...     "tranche_id": "A",
    ...     "cells": [{"cpr": 0.06, "cdr": 0.02, "severity": 0.6, "price": 98.2, "wal": 4.1, "writedown": 0.0}],
    ... })
    >>> table = ScenarioTable.from_json(payload)
    >>> table.tranche_id
    'A'
    """

    @staticmethod
    def from_json(json: str) -> ScenarioTable:
        """
        Deserialize from the JSON produced by ``to_json``.

        Parameters
        ----------
        json : str
            JSON-encoded ``ScenarioTable``.

        Returns
        -------
        ScenarioTable
            The decoded scenario table.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for the ``ScenarioTable`` shape.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.valuations.instruments import ScenarioTable
        >>> payload = {
        ...     "tranche_id": "A",
        ...     "cells": [{"cpr": 0.06, "cdr": 0.02, "severity": 0.6, "price": 98.2, "wal": 4.1, "writedown": 0.0}],
        ... }
        >>> table = ScenarioTable.from_json(json.dumps(payload))
        >>> (table.tranche_id, table.cells()[0]["price"])
        ('A', 98.2)

        """
        ...

    def to_json(self) -> str:
        """
        Serialize back to the same JSON shape ``from_json`` accepts.

        Returns
        -------
        str
            JSON-encoded ``ScenarioTable``.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def tranche_id(self) -> str:
        """
        Identifier of the tranche evaluated.

        Returns
        -------
        str
            The tranche identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def cells(self) -> list[dict[str, float]]:
        """
        Evaluated cells, in CPR-major, then CDR, then severity order.

        Each cell is a dict with keys ``cpr`` (annual decimal), ``cdr``
        (annual decimal), ``severity`` (decimal), ``price`` (percentage of
        original balance), ``wal`` (years), and ``writedown`` (currency
        units).

        Returns
        -------
        list[dict[str, float]]
            One dict per evaluated scenario cell.

        Notes
        -----
        This method does not raise; undefined results use ``None``, ``NaN``, or ``inf`` rather than an exception.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the evaluated cells as a pandas DataFrame.

        Columns: ``tranche_id``, ``cpr``, ``cdr``, ``severity``, ``price``
        (percentage of original balance), ``wal`` (years), ``writedown``
        (currency units). One row per cell, in CPR-major then CDR then
        severity order -- the same cells and order as ``cells``.

        Returns
        -------
        pd.DataFrame
            One row per scenario cell. A grid that evaluated no cells yields a
            zero-row frame that still carries the columns above.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

def structured_credit_tranche_discount_margin(
    instrument_json: str | StructuredCredit,
    tranche_id: str,
    market: MarketContext | str,
    as_of: datetime.date | str,
    target_pv: float,
) -> float:
    """Solve a z-spread-equivalent discount margin for a floating-rate tranche.

    Contractual cashflows are projected without changing coupon projection,
    then a constant additive spread is applied to the discount curve. The
    result is zero at model PV, negative for a richer (higher) target PV, and
    positive for a cheaper (lower) target PV; it is not the contractual quoted
    margin.

    Parameters
    ----------
    instrument_json : str or StructuredCredit
        Tagged JSON for a ``StructuredCredit`` deal, or a typed
        ``StructuredCredit`` instance.
    tranche_id : str
        Identifier of the floating-rate tranche whose contractual cashflows
        are spread-discounted.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        the discount curve and any forward curves or historical fixings
        required for cashflow projection.
    as_of : datetime.date | str
        Valuation date used for projection and discounting, either a date-like
        object or an ISO 8601 string.
    target_pv : float
        Target present value in the tranche's currency. Values above model PV
        produce a negative result; values below model PV produce a positive
        result.

    Returns
    -------
    float
        Z-spread-equivalent discount margin in decimal (``0.015`` = 150 bp).

    Raises
    ------
    ValueError
        If the JSON or date is malformed, the deal fails validation, the
        tranche is missing or fixed-rate, ``target_pv`` is not finite, required
        market data is unavailable, or the spread solve fails or exceeds
        ±5000 bp.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_discount_margin
    >>> try:
    ...     structured_credit_tranche_discount_margin("{}", "A", "{}", "2026-01-01", 100.0)
    ... except ValueError as exc:
    ...     print("schema" in str(exc))
    True

    """
    ...

def structured_credit_tranche_breakeven_cdr(
    instrument_json: str | StructuredCredit,
    tranche_id: str,
    market: MarketContext | str,
    as_of: datetime.date | str,
) -> float:
    """Solve the constant default rate at which a tranche first takes a writedown.

    Parameters
    ----------
    instrument_json : str or StructuredCredit
        Tagged JSON for a ``StructuredCredit`` deal, or a typed
        ``StructuredCredit`` instance.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.

    Returns
    -------
    float
        Break-even annual CDR in decimal.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_breakeven_cdr
    >>> try:
    ...     structured_credit_tranche_breakeven_cdr("{}", "A", "{}", "2026-01-01")
    ... except ValueError as exc:
    ...     print("schema" in str(exc))
    True

    """
    ...

def structured_credit_tranche_oas(
    instrument_json: str | StructuredCredit,
    tranche_id: str,
    market_price_pct: float,
    market: MarketContext | str,
    as_of: datetime.date | str,
    config_json: str | None = None,
) -> OasResult:
    """Compute option-adjusted spread for a tranche. Returns an ``OasResult``.

    Parameters
    ----------
    instrument_json : str or StructuredCredit
        Tagged JSON for a ``StructuredCredit`` deal, or a typed
        ``StructuredCredit`` instance.
    tranche_id : str
        Identifier of the tranche within the deal.
    market_price_pct : float
        Market price as a percentage of original balance (100.0 = par).
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    config_json : str or None, optional
        Serialized ``OasConfig``. All fields are required when supplied.

    Returns
    -------
    OasResult
        Typed OAS result. Call :meth:`OasResult.to_json` on it for the wire
        payload.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_oas
    >>> try:
    ...     structured_credit_tranche_oas("{}", "A", 100.0, "{}", "2026-01-01")
    ... except ValueError as exc:
    ...     print("schema" in str(exc))
    True

    """
    ...

def structured_credit_tranche_metrics(
    instrument_json: str | StructuredCredit,
    tranche_id: str,
    market: MarketContext | str,
    as_of: datetime.date | str,
    market_price_pct: float | None = None,
) -> TrancheMetrics:
    """Summary risk/pricing metrics for a tranche. Returns a ``TrancheMetrics``.

    Parameters
    ----------
    instrument_json : str or StructuredCredit
        Tagged JSON for a ``StructuredCredit`` deal, or a typed
        ``StructuredCredit`` instance.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    market_price_pct : float or None, optional
        Market price as a percentage of original balance; the model price is
        used when omitted.

    Returns
    -------
    TrancheMetrics
        Typed metrics bundle. Call :meth:`TrancheMetrics.to_json` on it for
        the wire payload.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_metrics
    >>> try:
    ...     structured_credit_tranche_metrics("{}", "A", "{}", "2026-01-01")
    ... except ValueError as exc:
    ...     print("schema" in str(exc))
    True

    """
    ...

def structured_credit_tranche_scenario_table(
    instrument_json: str | StructuredCredit,
    tranche_id: str,
    market: MarketContext | str,
    as_of: datetime.date | str,
    grid_json: str,
) -> ScenarioTable:
    """Price a tranche across a CPR x CDR x severity grid. Returns a ``ScenarioTable``.

    Parameters
    ----------
    instrument_json : str or StructuredCredit
        Tagged JSON for a ``StructuredCredit`` deal, or a typed
        ``StructuredCredit`` instance.
    tranche_id : str
        Identifier of the tranche within the deal.
    market : MarketContext or str
        Typed ``MarketContext`` or serialized market-context JSON supplying
        curves and fixings.
    as_of : datetime.date | str
        Valuation date, either a date-like object or an ISO 8601 string.
    grid_json : str
        Serialized ``ScenarioGrid``. Capped at 10,000 cells because each cell
        reprices the entire deal.

    Returns
    -------
    ScenarioTable
        Typed scenario table. Call :meth:`ScenarioTable.to_json` on it for the
        wire payload.

    Raises
    ------
    ValueError
        If the instrument JSON is malformed, the deal fails validation, the
        tranche id is not part of the deal, or required market data is missing.

    Examples
    --------
    >>> from finstack_quant.valuations.instruments import structured_credit_tranche_scenario_table
    >>> try:
    ...     structured_credit_tranche_scenario_table("{}", "A", "{}", "2026-01-01", "{}")
    ... except ValueError as exc:
    ...     print("schema" in str(exc))
    True

    """
    ...
