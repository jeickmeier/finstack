"""
Credit risk models: academic scoring, PD calibration, and LGD / EAD.

Bindings for ``finstack_quant_core::credit``. Each submodule mirrors the Rust
module of the same name and is registered at runtime in ``sys.modules``
so that ``from finstack_quant.core.credit import scoring`` (or ``pd``, ``lgd``,
``migration``, ``recovery_waterfall``, ``liability_management``) works
transparently.

Examples
--------
>>> from finstack_quant.core.credit import pd
>>> pd.central_tendency([0.01, 0.02, 0.03])
0.02

"""

from __future__ import annotations

__all__ = ["lgd", "liability_management", "migration", "pd", "recovery_waterfall", "scoring"]

class liability_management:
    """
    Distressed-exchange hold-versus-tender economics and issuer LME analytics.

    Examples
    --------
    >>> from finstack_quant.core.credit import liability_management
    >>> analysis = liability_management.analyze_exchange_offer(60.0, 75.0, consent_fee=2.0)
    >>> (analysis.delta_npv, analysis.tender_recommended)
    (17.0, True)

    """

    class ExchangeOfferAnalysis:
        """
        Hold-versus-tender economics of a distressed exchange offer.

        Examples
        --------
        >>> from finstack_quant.core.credit import liability_management
        >>> analysis = liability_management.analyze_exchange_offer(60.0, 75.0, consent_fee=2.0)
        >>> (analysis.delta_npv, analysis.tender_recommended)
        (17.0, True)

        """

        @property
        def exchange_type(self) -> str:
            """
            Return the canonical exchange structure for this analysis.

            Returns
            -------
            str
                One of ``par_for_par``, ``discount``, ``uptier``, ``downtier``.
            """
            ...

        @property
        def old_npv(self) -> float:
            """
            Return the hold-out present value used in the comparison.

            Returns
            -------
            float
                Present value of the existing claim if it is not tendered.
            """
            ...

        @property
        def new_npv(self) -> float:
            """
            Return the present value of the new instrument offered.

            Returns
            -------
            float
                Present value received on tendering, excluding fees.
            """
            ...

        @property
        def consent_fee(self) -> float:
            """
            Return the cash consent or early-tender fee.

            Returns
            -------
            float
                Fee paid to participating holders, in the input unit.
            """
            ...

        @property
        def equity_sweetener_value(self) -> float:
            """
            Return the value of equity or warrants attached to the offer.

            Returns
            -------
            float
                Estimated sweetener value, in the input unit.
            """
            ...

        @property
        def tender_total(self) -> float:
            """
            Return the total tender consideration.

            Returns
            -------
            float
                ``new_npv + consent_fee + equity_sweetener_value``.
            """
            ...

        @property
        def delta_npv(self) -> float:
            """
            Return the NPV pickup from tendering.

            Returns
            -------
            float
                ``tender_total - old_npv``; negative when holding out wins.
            """
            ...

        @property
        def breakeven_recovery(self) -> float:
            """
            Return the hold-out recovery that matches the tender.

            Returns
            -------
            float
                Fraction of the hold-out present value, capped at ``1.0``.
            """
            ...

        @property
        def tender_recommended(self) -> bool:
            """
            Return whether the offer clears the 2% tender hurdle.

            Returns
            -------
            bool
                True when ``tender_total > old_npv * 1.02``.
            """
            ...

    class LeverageImpact:
        """
        Gross-leverage impact of a liability management exercise.

        Examples
        --------
        >>> from finstack_quant.core.credit import liability_management
        >>> impact = liability_management.analyze_lme("open_market_repurchase", 100.0, 0.70, 0.50, 20.0).leverage_impact
        >>> (impact.pre_leverage, impact.post_leverage)
        (5.0, 2.5)

        """

        @property
        def pre_total_debt(self) -> float:
            """
            Return gross debt of the target instrument before the exercise.

            Returns
            -------
            float
                Outstanding face amount, in the input unit.
            """
            ...

        @property
        def post_total_debt(self) -> float:
            """
            Return gross debt of the target instrument after the exercise.

            Returns
            -------
            float
                Face amount remaining once retired par is removed.
            """
            ...

        @property
        def pre_leverage(self) -> float:
            """
            Return gross debt over EBITDA before the exercise.

            Returns
            -------
            float
                Leverage as a multiple, so ``8.0`` reads as 8.0x.
            """
            ...

        @property
        def post_leverage(self) -> float:
            """
            Return gross debt over EBITDA after the exercise.

            Returns
            -------
            float
                Leverage as a multiple, so ``4.8`` reads as 4.8x.
            """
            ...

        @property
        def leverage_reduction(self) -> float:
            """
            Return the turns of leverage removed by the exercise.

            Returns
            -------
            float
                ``pre_leverage - post_leverage``, in turns.
            """
            ...

    class LmeAnalysis:
        """
        Issuer-side economics of a liability management exercise.

        Examples
        --------
        >>> from finstack_quant.core.credit import liability_management
        >>> analysis = liability_management.analyze_lme("open_market_repurchase", 100.0, 0.70, 0.50)
        >>> (analysis.notional_reduction, analysis.discount_capture)
        (50.0, 15.0)

        """

        @property
        def lme_type(self) -> str:
            """
            Return the canonical LME structure for this analysis.

            Returns
            -------
            str
                One of ``open_market_repurchase``, ``tender_offer``,
                ``amend_and_extend``, ``dropdown``.
            """
            ...

        @property
        def cost(self) -> float:
            """
            Return the cash paid by the issuer.

            Returns
            -------
            float
                Repurchase consideration or consent fees, in the input unit.
            """
            ...

        @property
        def notional_reduction(self) -> float:
            """
            Return the face amount retired by the exercise.

            Returns
            -------
            float
                Par extinguished; zero for amend-and-extend and dropdowns.
            """
            ...

        @property
        def discount_capture(self) -> float:
            """
            Return the discount captured by the issuer.

            Returns
            -------
            float
                ``notional_reduction - cost``, in the input unit.
            """
            ...

        @property
        def discount_capture_pct(self) -> float:
            """
            Return the discount captured as a fraction of par retired.

            Returns
            -------
            float
                Fraction in ``[0, 1]``; zero when no par is retired.
            """
            ...

        @property
        def remaining_holder_impact_pct(self) -> float:
            """
            Return the value fraction diverted from non-participating holders.

            Returns
            -------
            float
                Nonzero only for a dropdown transaction.
            """
            ...

        @property
        def leverage_impact(self) -> liability_management.LeverageImpact | None:
            """
            Return the gross-leverage block, when EBITDA was supplied.

            Returns
            -------
            LeverageImpact or None
                None when no positive EBITDA was provided.
            """
            ...

    @staticmethod
    def analyze_exchange_offer(
        old_pv: float,
        new_pv: float,
        consent_fee: float = 0.0,
        equity_sweetener_value: float = 0.0,
        exchange_type: str = "par_for_par",
    ) -> liability_management.ExchangeOfferAnalysis:
        """
        Compare hold-versus-tender economics for a distressed exchange offer.

        Parameters
        ----------
        old_pv : float
            Present value of the existing claim if it is not tendered, in the
            caller's monetary unit. Must be finite and non-negative.
        new_pv : float
            Present value of the new instrument received on tendering,
            expressed in the same unit as ``old_pv``.
        consent_fee : float, optional
            Cash consent or early-tender fee paid to participating holders, in
            the same unit as ``old_pv``.
        equity_sweetener_value : float, optional
            Estimated value of equity or warrants attached to the new
            instrument, in the same unit as ``old_pv``.
        exchange_type : str, optional
            Offer structure: ``par_for_par`` (alias ``par``), ``discount``,
            ``uptier``, or ``downtier``. Case-insensitive; ``-`` is normalised
            to ``_``.

        Returns
        -------
        ExchangeOfferAnalysis
            Tender total, NPV pickup, breakeven recovery, and the tender
            recommendation against the 2% hurdle.

        Raises
        ------
        ValueError
            If an amount is negative or non-finite, or ``exchange_type`` is not
            a recognised structure.

        Examples
        --------
        >>> from finstack_quant.core.credit import liability_management
        >>> liability_management.analyze_exchange_offer(60.0, 75.0, consent_fee=2.0).tender_total
        77.0

        """
        ...

    @staticmethod
    def analyze_lme(
        lme_type: str,
        notional: float,
        repurchase_price_pct: float,
        opt_acceptance_pct: float = 1.0,
        ebitda: float | None = None,
    ) -> liability_management.LmeAnalysis:
        """
        Compute discount capture and leverage impact for an LME transaction.

        Parameters
        ----------
        lme_type : str
            Structure of the exercise: ``open_market`` (aliases
            ``open_market_repurchase``, ``omr``), ``tender_offer`` (alias
            ``tender``), ``amend_and_extend`` (aliases ``ae``, ``a&e``), or
            ``dropdown``. Case-insensitive; ``-`` and ``&`` normalise to ``_``.
        notional : float
            Outstanding face amount of the target instrument, in the caller's
            monetary unit. Must be finite and strictly positive.
        repurchase_price_pct : float
            Price as a fraction of par for repurchases and tenders (``(0, 1.5]``),
            the extension fee for amend-and-extend (``[0, 0.10]``), or the
            transferred-asset fraction for a dropdown (``[0, 1]``).
        opt_acceptance_pct : float, optional
            Fraction of holders participating, in ``[0, 1]``. Defaults to full
            participation.
        ebitda : float or None, optional
            EBITDA in the same unit as ``notional``. A positive value adds the
            ``leverage_impact`` block; None or a non-positive value omits it.

        Returns
        -------
        LmeAnalysis
            Cash cost, par retired, discount captured, impact on remaining
            holders, and the optional gross-leverage block.

        Raises
        ------
        ValueError
            If ``notional`` is not positive, ``opt_acceptance_pct`` is outside
            ``[0, 1]``, ``repurchase_price_pct`` is outside the range admitted
            by ``lme_type``, or ``lme_type`` is not recognised.

        Examples
        --------
        >>> from finstack_quant.core.credit import liability_management
        >>> liability_management.analyze_lme("open_market_repurchase", 100.0, 0.70, 0.50).discount_capture
        15.0

        """
        ...

class recovery_waterfall:
    """
    Absolute-priority recovery allocation with estate-inclusive collateral.

    Examples
    --------
    >>> from finstack_quant.core.credit import recovery_waterfall
    >>> claim = recovery_waterfall.RecoveryClaim("SEN", "secured", 1, 100.0)
    >>> result = recovery_waterfall.allocate_recovery(40.0, [claim])
    >>> (result.total_distributed, result.undistributed_estate, result.apr_satisfied)
    (40.0, 0.0, True)

    """

    class RecoveryClaim:
        """
        Compute recovery waterfall.RecoveryClaim.

        Examples
        --------
        >>> from finstack_quant.core.credit import recovery_waterfall
        >>> claim = recovery_waterfall.RecoveryClaim("SEN", "secured", 1, 100.0, accrued=5.0)
        >>> (claim.id, claim.total_claim)
        ('SEN', 105.0)

        """

        def __init__(
            self,
            id: str,
            seniority: str,
            priority: int,
            principal: float,
            accrued: float = 0.0,
            penalties: float = 0.0,
            collateral: tuple[float, float] | None = None,
        ) -> None:
            """
            Create a claim for absolute-priority recovery allocation.

            Parameters
            ----------
            id : str
                Stable claim identifier retained on the resulting allocation.
            seniority : str
                Human-readable seniority label used in recovery reporting.
            priority : int
                Absolute-priority rank; lower values receive estate proceeds
                before higher values.
            principal : float
                Outstanding principal claim in the estate's monetary units.
            accrued : float, default 0.0
                Unpaid accrued interest added to the claim amount.
            penalties : float, default 0.0
                Contractual penalty or default-interest claim added to the total.
            collateral : tuple[float, float] or None, default None
                Optional ``(market_value, haircut)`` collateral tuple. The
                haircut is a decimal fraction deducted before estate allocation.

            """
            ...
        @property
        def id(self) -> str:
            """
            Return the id for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            str
                The id exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def seniority(self) -> str:
            """
            Return the seniority for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            str
                The seniority exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def priority(self) -> int:
            """
            Return the priority for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            int
                The priority exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def principal(self) -> float:
            """
            Return the principal for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float
                The principal exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def accrued(self) -> float:
            """
            Return the accrued for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float
                The accrued exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def penalties(self) -> float:
            """
            Return the penalties for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float
                The penalties exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def collateral_value(self) -> float | None:
            """
            Return the collateral value for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float | None
                The collateral value exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def collateral_haircut(self) -> float:
            """
            Return the collateral haircut for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float
                The collateral haircut exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

        @property
        def total_claim(self) -> float:
            """
            Return the total claim for `recovery_waterfall.RecoveryClaim`.

            Returns
            -------
            float
                The total claim exposed by this `recovery_waterfall.RecoveryClaim`.
            """
            ...

    class RecoveryAllocation:
        """
        Compute recovery waterfall.RecoveryAllocation.

        Examples
        --------
        >>> from finstack_quant.core.credit import recovery_waterfall
        >>> claim = recovery_waterfall.RecoveryClaim("SEN", "secured", 1, 100.0)
        >>> allocation = recovery_waterfall.allocate_recovery(40.0, [claim]).allocations[0]
        >>> (allocation.id, allocation.total_recovery, allocation.recovery_rate)
        ('SEN', 40.0, 0.4)

        """

        @property
        def id(self) -> str:
            """
            Return the id for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            str
                The id exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def seniority(self) -> str:
            """
            Return the seniority for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            str
                The seniority exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def priority(self) -> int:
            """
            Return the priority for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            int
                The priority exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def total_claim(self) -> float:
            """
            Return the total claim for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The total claim exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def collateral_recovery(self) -> float:
            """
            Return the collateral recovery for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The collateral recovery exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def general_recovery(self) -> float:
            """
            Return the general recovery for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The general recovery exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def total_recovery(self) -> float:
            """
            Return the total recovery for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The total recovery exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def recovery_rate(self) -> float:
            """
            Return the recovery rate for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The recovery rate exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

        @property
        def deficiency(self) -> float:
            """
            Return the deficiency for `recovery_waterfall.RecoveryAllocation`.

            Returns
            -------
            float
                The deficiency exposed by this `recovery_waterfall.RecoveryAllocation`.
            """
            ...

    class RecoveryWaterfallResult:
        """
        Compute recovery waterfall.RecoveryWaterfallResult.

        Examples
        --------
        >>> from finstack_quant.core.credit import recovery_waterfall
        >>> claim = recovery_waterfall.RecoveryClaim("SEN", "secured", 1, 100.0)
        >>> result = recovery_waterfall.allocate_recovery(40.0, [claim])
        >>> (result.total_distributed, result.undistributed_estate, result.apr_satisfied)
        (40.0, 0.0, True)

        """

        @property
        def total_distributed(self) -> float:
            """
            Return the total distributed for `recovery_waterfall.RecoveryWaterfallResult`.

            Returns
            -------
            float
                The total distributed exposed by this `recovery_waterfall.RecoveryWaterfallResult`.
            """
            ...

        @property
        def undistributed_estate(self) -> float:
            """
            Return the undistributed estate for `recovery_waterfall.RecoveryWaterfallResult`.

            Returns
            -------
            float
                The undistributed estate exposed by this `recovery_waterfall.RecoveryWaterfallResult`.
            """
            ...

        @property
        def apr_satisfied(self) -> bool:
            """
            Return the apr satisfied for `recovery_waterfall.RecoveryWaterfallResult`.

            Returns
            -------
            bool
                The apr satisfied exposed by this `recovery_waterfall.RecoveryWaterfallResult`.
            """
            ...

        @property
        def allocations(self) -> list[recovery_waterfall.RecoveryAllocation]:
            """
            Return the allocations for `recovery_waterfall.RecoveryWaterfallResult`.

            Returns
            -------
            list[recovery_waterfall.RecoveryAllocation]
                The allocations exposed by this `recovery_waterfall.RecoveryWaterfallResult`.
            """
            ...

    @staticmethod
    def allocate_recovery(
        estate_value: float,
        claims: list[recovery_waterfall.RecoveryClaim],
    ) -> recovery_waterfall.RecoveryWaterfallResult:
        """
        Allocate an insolvent estate under absolute priority.

        Parameters
        ----------
        estate_value : float
            Cash estate available for distribution after any external costs,
            expressed in the same monetary units as each claim.
        claims : list[RecoveryClaim]
            Claims to rank by ``priority``. Collateral recovery is applied to
            each claim before general estate proceeds are distributed.

        Returns
        -------
        RecoveryWaterfallResult
            Per-claim recoveries, undistributed estate, and APR satisfaction.

        Raises
        ------
        ValueError
            If the estate or claim amounts are negative or non-finite, a claim
            identifier or seniority is blank, identifiers are duplicated,
            a haircut is outside ``[0, 1]``, a claim total overflows, or net
            collateral exceeds the estate.
        RuntimeError
            If the allocator cannot reserve its claim-index storage or a
            recovery-conservation invariant fails.


        Examples
        --------
        >>> from finstack_quant.core.credit import recovery_waterfall
        >>> claims = [recovery_waterfall.RecoveryClaim("SEN", "secured", 1, 100.0)]
        >>> recovery_waterfall.allocate_recovery(40.0, claims).allocations[0].recovery_rate
        0.4

        """
        ...

class scoring:
    """
    Academic credit scoring: Altman Z-Score family, Ohlson O-Score, Zmijewski.

    Examples
    --------
    >>> from finstack_quant.core.credit import scoring
    >>> round(scoring.altman_z_score(0.2, 0.3, 0.15, 1.5, 1.0)[0], 3)
    3.055

    """

    class AltmanPdCalibration:
        """
        Explicit versioned Altman score-to-PD heuristics.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> result = scoring.altman_z_score(0.2, 0.3, 0.15, 1.5, 1.0, scoring.AltmanPdCalibration.HEURISTIC_V1)
        >>> result[2] is not None
        True

        """

        HEURISTIC_V1: scoring.AltmanPdCalibration

    @staticmethod
    def altman_z_score(
        working_capital_to_total_assets: float,
        retained_earnings_to_total_assets: float,
        ebit_to_total_assets: float,
        market_equity_to_total_liabilities: float,
        sales_to_total_assets: float,
        pd_calibration: AltmanPdCalibration | None = None,
    ) -> tuple[float, str, float | None]:
        """
        Original Altman Z-Score (1968) for publicly traded manufacturers.

        Parameters
        ----------
        working_capital_to_total_assets : float
            Working capital / total assets (X1).
        retained_earnings_to_total_assets : float
            Retained earnings / total assets (X2).
        ebit_to_total_assets : float
            EBIT / total assets (X3).
        market_equity_to_total_liabilities : float
            Market equity / total liabilities (X4).
        sales_to_total_assets : float
            Sales / total assets (X5).
        pd_calibration : AltmanPdCalibration | None
            Explicit score-to-PD mapping. ``HEURISTIC_V1`` is an uncalibrated
            house heuristic, not an empirical Altman calibration.

        Returns
        -------
        tuple[float, str, float | None]
            ``(score, zone, implied_pd)`` where ``zone`` is one of
            ``"safe"``, ``"grey"``, or ``"distress"``. ``implied_pd`` is
            ``None`` unless ``pd_calibration`` is supplied.

        Raises
        ------
        ValueError
            If any ratio is non-finite.

        Sources
        -------
        See ``docs/REFERENCES.md#altman-1968``.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.altman_z_score(0.2, 0.3, 0.15, 1.5, 1.0)
        >>> zone
        'safe'
        """
        ...

    @staticmethod
    def altman_z_prime(
        working_capital_to_total_assets: float,
        retained_earnings_to_total_assets: float,
        ebit_to_total_assets: float,
        book_equity_to_total_liabilities: float,
        sales_to_total_assets: float,
        pd_calibration: AltmanPdCalibration | None = None,
    ) -> tuple[float, str, float | None]:
        """
        Altman Z'-Score (1983) for private firms.

        Parameters
        ----------
        working_capital_to_total_assets : float
            Working capital divided by total assets (Altman X1).
        retained_earnings_to_total_assets : float
            Cumulative retained earnings divided by total assets (X2).
        ebit_to_total_assets : float
            Earnings before interest and tax divided by total assets (X3).
        book_equity_to_total_liabilities : float
            Book value of equity divided by total liabilities, replacing the
            original public-company market-equity ratio (X4).
        sales_to_total_assets : float
            Sales divided by total assets, the private-firm turnover ratio (X5).
        pd_calibration : AltmanPdCalibration or None, default None
            Optional explicit score-to-PD heuristic. ``None`` returns no
            implied PD rather than applying an undocumented mapping.

        Returns
        -------
        tuple[float, str, float | None]
            ``(score, zone, implied_pd)`` where ``zone`` is ``"safe"``,
            ``"grey"``, or ``"distress"``. PD is absent unless an explicit
            versioned heuristic is supplied.

        Raises
        ------
        ValueError
            If any accounting ratio is non-finite.

        Sources
        -------
        - Altman (1968/1983): see docs/REFERENCES.md#altman-1968

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.altman_z_prime(0.2, 0.3, 0.15, 1.5, 1.0)
        >>> zone in ("safe", "grey", "distress")
        True

        """
        ...

    @staticmethod
    def altman_z_double_prime(
        working_capital_to_total_assets: float,
        retained_earnings_to_total_assets: float,
        ebit_to_total_assets: float,
        book_equity_to_total_liabilities: float,
        pd_calibration: AltmanPdCalibration | None = None,
    ) -> tuple[float, str, float | None]:
        """
        Altman Z''-Score for non-manufacturing firms (non-EM model, no constant).

        Parameters
        ----------
        working_capital_to_total_assets : float
            Working capital divided by total assets (Altman X1).
        retained_earnings_to_total_assets : float
            Cumulative retained earnings divided by total assets (X2).
        ebit_to_total_assets : float
            Earnings before interest and tax divided by total assets (X3).
        book_equity_to_total_liabilities : float
            Book value of equity divided by total liabilities (X4).
        pd_calibration : AltmanPdCalibration or None, default None
            Optional explicit score-to-PD heuristic; ``None`` leaves the
            implied-PD component absent.

        Returns ``(score, zone, implied_pd)``; PD is ``None`` unless an
        explicit versioned heuristic is supplied.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.altman_z_double_prime(0.2, 0.3, 0.15, 1.5)
        >>> zone in ("safe", "grey", "distress")
        True

        Returns
        -------
        tuple[float, str, float | None]
            Unitless ``(Z'' score, lowercase zone, implied_pd)``; PD is
            ``None`` without calibration and otherwise a decimal probability.

        Raises
        ------
        ValueError
            If any accounting ratio is non-finite.

        """
        ...

    @staticmethod
    def altman_em_score(
        working_capital_to_total_assets: float,
        retained_earnings_to_total_assets: float,
        ebit_to_total_assets: float,
        book_equity_to_total_liabilities: float,
    ) -> tuple[float, str, float | None]:
        """
        Compute the Altman EM-Score for emerging-market corporates.

        EM = 3.25 + Z''. Zone cutoffs: EM > 5.85 Safe, 4.35 <= EM <= 5.85 Grey,
        EM < 4.35 Distress (Altman, Hartzell & Peck 1995).

        Parameters
        ----------
        working_capital_to_total_assets : float
            Working capital divided by total assets (Altman X1).
        retained_earnings_to_total_assets : float
            Cumulative retained earnings divided by total assets (X2).
        ebit_to_total_assets : float
            Earnings before interest and tax divided by total assets (X3).
        book_equity_to_total_liabilities : float
            Book value of equity divided by total liabilities (X4).

        Returns
        -------
        tuple[float, str, float | None]
            ``(score, zone, implied_pd)``; ``implied_pd`` is always ``None``.

        Raises
        ------
        ValueError
            If any ratio is non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.altman_em_score(0.2, 0.3, 0.15, 1.2)
        >>> round(score, 3), zone
        (7.808, 'safe')
        """
        ...

    @staticmethod
    def ohlson_o_score(
        log_total_assets_adjusted: float,
        total_liabilities_to_total_assets: float,
        working_capital_to_total_assets: float,
        current_liabilities_to_current_assets: float,
        liabilities_exceed_assets: float,
        net_income_to_total_assets: float,
        funds_from_operations_to_total_liabilities: float,
        negative_net_income_two_years: float,
        net_income_change: float,
    ) -> tuple[float, str, float]:
        """
        Ohlson O-Score (1980) logistic bankruptcy model.

        Parameters
        ----------
        log_total_assets_adjusted : float
            Natural log of inflation-adjusted total assets, the Ohlson size
            variable.
        total_liabilities_to_total_assets : float
            Total liabilities divided by total assets.
        working_capital_to_total_assets : float
            Working capital divided by total assets.
        current_liabilities_to_current_assets : float
            Current liabilities divided by current assets.
        liabilities_exceed_assets : float
            Indicator equal to ``1.0`` when liabilities exceed assets and
            ``0.0`` otherwise.
        net_income_to_total_assets : float
            Net income divided by total assets.
        funds_from_operations_to_total_liabilities : float
            Funds from operations divided by total liabilities.
        negative_net_income_two_years : float
            Indicator equal to ``1.0`` when net income was negative in both
            the current and prior year, otherwise ``0.0``.
        net_income_change : float
            Ohlson CHIN variable describing the scaled change in net income
            between the current and prior year.

        Returns ``(score, zone, implied_pd)``.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.ohlson_o_score(-0.5, 0.5, 0.1, 0.8, 0.0, 0.05, 0.2, 1.0, -0.01)
        >>> zone in ("safe", "grey", "distress")
        True

        Returns
        -------
        tuple[float, str, float]
            Unitless ``(O logit, lowercase zone, implied_pd)`` with decimal PD
            ``1 / (1 + exp(-O))``.

        Raises
        ------
        ValueError
            If any accounting input is non-finite, or if either binary
            indicator is not exactly ``0.0`` or ``1.0``.

        """
        ...

    @staticmethod
    def zmijewski_score(
        net_income_to_total_assets: float,
        total_liabilities_to_total_assets: float,
        current_assets_to_current_liabilities: float,
    ) -> tuple[float, str, float]:
        """
        Zmijewski (1984) probit bankruptcy score.

        Parameters
        ----------
        net_income_to_total_assets : float
            Net income divided by total assets, the profitability predictor.
        total_liabilities_to_total_assets : float
            Total liabilities divided by total assets, the leverage predictor.
        current_assets_to_current_liabilities : float
            Current assets divided by current liabilities, the liquidity ratio.

        Returns ``(score, zone, implied_pd)``.

        Examples
        --------
        >>> from finstack_quant.core.credit import scoring
        >>> score, zone, pd = scoring.zmijewski_score(0.05, 0.4, 1.5)
        >>> zone in ("safe", "grey", "distress")
        True

        Returns
        -------
        tuple[float, str, float]
            Unitless ``(Y probit, lowercase zone, implied_pd)`` with decimal PD
            equal to ``Phi(Y)``.

        Raises
        ------
        ValueError
            If any accounting ratio is non-finite.

        """
        ...

class pd:
    """
    Probability of default: PiT/TtC conversion and central-tendency calibration.

    Examples
    --------
    >>> from finstack_quant.core.credit import pd
    >>> pd.central_tendency([0.01, 0.02, 0.03])
    0.02

    """

    @staticmethod
    def pit_to_ttc(pit_pd: float, asset_correlation: float, cycle_index: float) -> float:
        """
        Convert a Point-in-Time PD to Through-the-Cycle via Merton-Vasicek.

        ``PD_TtC = Phi( Phi^{-1}(PD_PiT) * sqrt(1 - rho) + sqrt(rho) * z )``.

        Parameters
        ----------
        pit_pd:
            Point-in-time probability of default in ``(0, 1)``.
        asset_correlation:
            Asset correlation ``rho`` in ``[0, 1)``.
        cycle_index:
            Standardized credit cycle index ``z`` (negative = downturn,
            positive = benign).

        Returns
        -------
        float
            Through-the-cycle PD in ``(0, 1)``.

        Raises
        ------
        ValueError
            If *pit_pd* or *asset_correlation* is not strictly between zero
            and one, or if *cycle_index* is non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import pd
        >>> round(pd.pit_to_ttc(0.02, 0.12, 0.0), 6)
        0.027016

        """
        ...

    @staticmethod
    def ttc_to_pit(ttc_pd: float, asset_correlation: float, cycle_index: float) -> float:
        """
        Convert a Through-the-Cycle PD to Point-in-Time via Merton-Vasicek.

        ``PD_PiT = Phi( (Phi^{-1}(PD_TtC) - sqrt(rho) * z) / sqrt(1 - rho) )``.

        Parameters
        ----------
        ttc_pd:
            Through-the-cycle probability of default in ``(0, 1)``.
        asset_correlation:
            Asset correlation ``rho`` in ``[0, 1)``.
        cycle_index:
            Standardized credit cycle index ``z`` (negative = downturn,
            positive = benign).

        Returns
        -------
        float
            Point-in-time PD in ``(0, 1)``.

        Raises
        ------
        ValueError
            If *ttc_pd* or *asset_correlation* is not strictly between zero
            and one, or if *cycle_index* is non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import pd
        >>> round(pd.ttc_to_pit(0.02, 0.12, 0.0), 6)
        0.014287

        """
        ...

    @staticmethod
    def central_tendency(annual_default_rates: list[float]) -> float:
        """
        Arithmetic-mean long-run PD from annual default rates (regulatory TtC).

        Parameters
        ----------
        annual_default_rates:
            Observed annual default rates as decimals.

        Returns
        -------
        float
            Long-run average PD.

        Raises
        ------
        ValueError
            If *annual_default_rates* is empty or contains a non-finite value
            or a rate outside ``[0, 1]``.

        Examples
        --------
        >>> from finstack_quant.core.credit import pd
        >>> pd.central_tendency([0.01, 0.02, 0.03])
        0.02

        """
        ...

class lgd:
    """
    Loss-given-default: seniority recovery, workout LGD, downturn adjustments, EAD.

    Examples
    --------
    >>> from finstack_quant.core.credit import lgd
    >>> lgd.ead_revolver(60.0, 40.0, 0.5)
    80.0

    """

    @staticmethod
    def seniority_recovery_stats(
        seniority: str,
        rating_agency: str | None = None,
    ) -> dict[str, float]:
        """
        Historical recovery moments for a seniority class.

        If ``rating_agency`` is omitted, the Rust credit-assumptions registry
        default seniority calibration is used.

        Parameters
        ----------
        seniority:
            Seniority label (e.g. ``"senior_secured"``, ``"senior_unsecured"``,
            ``"subordinated"``).
        rating_agency:
            Optional agency source (e.g. ``"Moody"``, ``"S&P"``).

        Returns
        -------
        dict[str, float]
            Dict with keys ``{"mean", "std", "alpha", "beta"}``.

        Raises
        ------
        ValueError
            If *seniority* is unknown, *rating_agency* is unsupported, or the
            selected calibration does not contain the seniority class.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> lgd.seniority_recovery_stats("senior_secured")["mean"]
        0.52

        """
        ...

    @staticmethod
    def beta_recovery_sample(
        mean: float,
        std: float,
        n_samples: int,
        seed: int,
    ) -> list[float]:
        """
        Sample ``n_samples`` recoveries from Beta(alpha, beta) via PCG64.

        Parameters
        ----------
        mean:
            Target mean recovery rate in ``(0, 1)``.
        std:
            Target standard deviation of recovery rate.
        n_samples:
            Number of samples to draw.
        seed:
            Random seed for reproducibility.

        Returns
        -------
        list[float]
            Sampled recovery rates in ``(0, 1)``.

        Raises
        ------
        ValueError
            If *mean* is not strictly between zero and one, or if *std* is not
            positive and compatible with a Beta distribution having that mean.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> [round(value, 6) for value in lgd.beta_recovery_sample(0.4, 0.2, 3, 42)]
        [0.217125, 0.329273, 0.244662]

        """
        ...

    @staticmethod
    def beta_recovery_quantile(mean: float, std: float, q: float) -> float:
        """
        Quantile ``q`` of a Beta recovery distribution parameterized by (mean, std).

        Parameters
        ----------
        mean:
            Mean recovery rate in ``(0, 1)``.
        std:
            Standard deviation of recovery rate.
        q:
            Quantile in ``[0, 1]``.

        Returns
        -------
        float
            Recovery rate at the given quantile.

        Raises
        ------
        ValueError
            If *mean* and *std* cannot parameterize a Beta distribution, or if
            *q* is non-finite or outside ``[0, 1]``.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> round(lgd.beta_recovery_quantile(0.4, 0.2, 0.5), 6)
        0.385728

        """
        ...

    @staticmethod
    def workout_lgd(
        ead: float,
        collateral: list[tuple[str, float, float]],
        direct_cost_pct: float,
        indirect_cost_pct: float,
        time_to_resolution_years: float,
        discount_rate: float,
    ) -> tuple[float, float]:
        """
        Workout LGD from collateral waterfall, costs, and discounting.

        Parameters
        ----------
        ead:
            Exposure at default.
        collateral:
            List of ``(collateral_id, recovery_value, recovery_rate)`` tuples
            in priority order.
        direct_cost_pct:
            Direct workout costs as a fraction of EAD.
        indirect_cost_pct:
            Indirect workout costs as a fraction of EAD.
        time_to_resolution_years:
            Time from default to workout resolution.
        discount_rate:
            Annual discount rate for time-value adjustment.

        Returns
        -------
        tuple[float, float]
            ``(net_recovery, lgd)`` with ``lgd`` clamped to ``[0, 1]``.

        Raises
        ------
        ValueError
            If *ead* is not finite and strictly positive; a collateral type is
            unknown; a collateral value, haircut, cost rate, workout horizon,
            or discount rate is non-finite or outside its documented range.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> recovery, loss = lgd.workout_lgd(100.0, [("cash", 40.0, 1.0)], 0.02, 0.01, 1.5, 0.05)
        >>> 0.0 <= loss <= 1.0
        True

        """
        ...

    @staticmethod
    def downturn_lgd_stressed(
        base_lgd: float,
        asset_correlation: float,
        lgd_sensitivity: float,
        stress_quantile: float,
    ) -> float:
        """
        Stressed downturn LGD adjustment, clamped to ``[0, 1]``.

        Proprietary mean-plus-multiple-of-Bernoulli-stdev approximation
        (not the Frye-Jacobs 2012 model). Typical ``lgd_sensitivity``:
        0.3-0.5.

        Parameters
        ----------
        base_lgd:
            Baseline LGD in ``[0, 1]``.
        asset_correlation:
            Asset correlation ``rho`` in ``[0, 1)``.
        lgd_sensitivity:
            Sensitivity of LGD to systematic risk (typical: 0.3-0.5).
        stress_quantile:
            Quantile of the systematic factor for stress (e.g. ``0.999``).

        Returns
        -------
        float
            Stressed LGD in ``[0, 1]``.

        Raises
        ------
        ValueError
            If *base_lgd* is outside ``[0, 1]``, *asset_correlation* or
            *stress_quantile* is not strictly between zero and one, or
            *lgd_sensitivity* is negative or non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> round(lgd.downturn_lgd_stressed(0.4, 0.12, 0.3, 0.999), 6)
        0.557329

        """
        ...

    @staticmethod
    def downturn_lgd_regulatory_floor(
        base_lgd: float,
        add_on: float,
        floor: float,
    ) -> float:
        """
        Regulatory-floor downturn LGD: ``max(base + add_on, floor)`` clamped to ``[0, 1]``.

        Parameters
        ----------
        base_lgd:
            Baseline LGD in ``[0, 1]``.
        add_on:
            Downturn add-on.
        floor:
            Regulatory floor LGD.

        Returns
        -------
        float
            Floored downturn LGD in ``[0, 1]``.

        Raises
        ------
        ValueError
            If *base_lgd* or *floor* is non-finite or outside ``[0, 1]``, or
            if *add_on* is negative or non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> lgd.downturn_lgd_regulatory_floor(0.4, 0.05, 0.45)
        0.45

        """
        ...

    @staticmethod
    def ead_term_loan(principal: float) -> float:
        """
        Exposure at default for a fully drawn term loan (equal to principal).

        Parameters
        ----------
        principal:
            Outstanding principal amount.

        Returns
        -------
        float
            EAD equal to ``principal``.

        Raises
        ------
        ValueError
            If *principal* is negative or non-finite.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> lgd.ead_term_loan(100.0)
        100.0

        """
        ...

    @staticmethod
    def ead_revolver(drawn: float, undrawn: float, ccf: float) -> float:
        """
        Exposure at default for a revolver: ``drawn + undrawn * ccf``.

        Parameters
        ----------
        drawn:
            Current funded balance in the facility's monetary units.
        undrawn:
            Undrawn commitment.
        ccf:
            Credit conversion factor in ``[0, 1]``.

        Returns
        -------
        float
            EAD for the revolver facility.

        Raises
        ------
        ValueError
            If *drawn* or *undrawn* is negative or non-finite, or if *ccf* is
            non-finite or outside ``[0, 1]``.

        Examples
        --------
        >>> from finstack_quant.core.credit import lgd
        >>> lgd.ead_revolver(60.0, 40.0, 0.5)
        80.0

        """
        ...

class migration:
    """
    Credit migration: rating scales, transition matrices, generators, and CTMC simulation.

    Examples
    --------
    >>> from finstack_quant.core.credit import migration
    >>> migration.RatingScale.custom_with_default(["A", "D"], "D").labels()
    ['A', 'D']

    """

    class RatingScale:
        """
        Ordinal rating scale for credit migration modelling.

        Provides standard agency scales (S&P/Moody's/Fitch) or custom scales
        with an optional default absorbing state.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> (scale.n_states(), scale.index_of("A"), scale.default_state(), scale.labels())
        (2, 0, 1, ['A', 'D'])
        >>> (scale.warf("A"), scale.rating_from_warf(120.0))
        (120.0, 'A')

        """

        @staticmethod
        def standard() -> migration.RatingScale:
            """
            Standard 8-state agency scale (AAA through D).

            Returns
            -------
            migration.RatingScale
                Scale with labels ``AAA, AA, A, BBB, BB, B, CCC, D``.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> migration.RatingScale.standard().n_states()
            10

            """
            ...

        @staticmethod
        def standard_with_nr() -> migration.RatingScale:
            """
            Standard scale with an explicit ``NR`` (not rated) state.

            Returns
            -------
            migration.RatingScale
                Scale with labels ``AAA, AA, A, BBB, BB, B, CCC, D, NR``.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> migration.RatingScale.standard_with_nr().n_states()
            11

            """
            ...

        @staticmethod
        def notched() -> migration.RatingScale:
            """
            Notched 18-state scale (AAA through CCC- and D).

            Returns
            -------
            migration.RatingScale
                Scale with notched sub-grades.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> migration.RatingScale.notched().n_states()
            22

            """
            ...

        @staticmethod
        def custom(labels: list[str]) -> migration.RatingScale:
            """
            Build a custom rating scale from an ordered label list.

            Parameters
            ----------
            labels:
                Rating labels from best to worst.  The last label is the
                default absorbing state.

            Returns
            -------
            migration.RatingScale
                Custom scale.

            Raises
            ------
            ValueError
                If fewer than two labels are supplied or any label is duplicated.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> migration.RatingScale.custom(["A", "B"]).labels()
            ['A', 'B']

            """
            ...

        @staticmethod
        def custom_with_default(labels: list[str], default_label: str) -> migration.RatingScale:
            """
            Build a custom rating scale with an explicit default label.

            Parameters
            ----------
            labels:
                Non-default rating labels from best to worst.
            default_label:
                Label for the default absorbing state.

            Returns
            -------
            migration.RatingScale
                Custom scale with the default state appended.

            Raises
            ------
            ValueError
                If fewer than two labels are supplied or any label is duplicated.
            KeyError
                If *default_label* is not present in *labels*.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> migration.RatingScale.custom_with_default(["A", "D"], "D").default_state()
            1

            """
            ...

        def n_states(self) -> int:
            """
            Number of states on this scale (including default if present).

            Returns
            -------
            int
                State count.

            """
            ...

        def index_of(self, label: str) -> int | None:
            """
            Return the 0-based index of ``label``, or ``None`` if not found.

            Parameters
            ----------
            label:
                Rating label to look up.

            Returns
            -------
            int or None
                State index, or ``None`` when the label is not on this scale.

            """
            ...

        def default_state(self) -> int | None:
            """
            Return the index of the default absorbing state, or ``None``.

            Returns
            -------
            int or None
                Default state index, or ``None`` if no default state exists.

            """
            ...

        def labels(self) -> list[str]:
            """
            Return all rating labels in ordinal order.

            Returns
            -------
            list[str]
                Ordered label list.

            """
            ...

        def warf(self, label: str) -> float:
            """
            Weighted average rating factor (WARF) for a rating label.

            Parameters
            ----------
            label:
                Rating label on this scale.

            Returns
            -------
            float
                WARF value (higher = riskier).

            Raises
            ------
            ValueError
                If ``label`` is not on this scale.

            """
            ...

        def rating_from_warf(self, warf: float) -> str:
            """
            Map a WARF value back to the closest rating label.

            Parameters
            ----------
            warf:
                Weighted average rating factor.

            Returns
            -------
            str
                Rating label whose WARF bucket contains the given value.

            Raises
            ------
            ValueError
                If *warf* is non-finite or the scale contains no label with a
                known Moody's WARF factor.

            """
            ...

    class TransitionMatrix:
        """
        Discrete-horizon rating transition probability matrix.

        Parameters
        ----------
        scale:
            Rating scale defining the row/column ordering.
        data:
            Row-major transition probabilities (length ``n_states * n_states``).
        horizon:
            Time horizon in years (e.g. ``1.0`` for a 1-year matrix).

        Raises
        ------
        ValueError
            If ``data`` length does not match ``scale.n_states() ** 2`` or rows
            do not sum to 1.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> matrix = migration.TransitionMatrix(scale, [0.9, 0.1, 0.0, 1.0], 1.0)
        >>> (matrix.probability("A", "D"), matrix.row("A"), matrix.horizon(), matrix.n_states())
        (0.1, [0.9, 0.1], 1.0, 2)
        >>> (matrix.to_matrix(), matrix.default_probabilities())
        ([[0.9, 0.1], [0.0, 1.0]], [0.1, 1.0])

        """

        def __init__(self, scale: migration.RatingScale, data: list[float], horizon: float) -> None:
            """
            Construct a validated discrete-horizon transition matrix.

            Parameters
            ----------
            scale : migration.RatingScale
                Ordered rating states defining the row and column labels and,
                when configured, the absorbing default state.
            data : list[float]
                Row-major ``n * n`` transition probabilities for the ``n``
                states in ``scale``. Entries must lie in ``[0, 1]``, every row
                must sum to one, and a configured default row must be absorbing.
            horizon : float
                Finite, strictly positive transition horizon in years.

            Raises
            ------
            ValueError
                If *horizon* is not finite and strictly positive; *data* is not
                an ``n x n`` row-major matrix; an entry is non-finite or outside
                ``[0, 1]``; a row does not sum to one; or the default row is not
                absorbing.

            """
            ...

        def probability(self, from_: str, to: str) -> float:
            """
            Transition probability from one rating to another.

            Parameters
            ----------
            from_:
                Origin rating label.
            to:
                Destination rating label.

            Returns
            -------
            float
                Transition probability in ``[0, 1]``.

            Raises
            ------
            KeyError
                If *from_* or *to* is not a label on the matrix's rating scale.

            """
            ...

        def row(self, from_: str) -> list[float]:
            """
            Return the full transition row for a given origin rating.

            Parameters
            ----------
            from_:
                Origin rating label.

            Returns
            -------
            list[float]
                Transition probabilities to every state on the scale.

            Raises
            ------
            KeyError
                If *from_* is not a label on the matrix's rating scale.

            """
            ...

        def to_matrix(self) -> list[list[float]]:
            """
            Return the full transition matrix as nested lists.

            Returns
            -------
            list[list[float]]
                Row-major matrix of transition probabilities.

            """
            ...

        def horizon(self) -> float:
            """
            Return the time horizon in years.

            Returns
            -------
            float
                Horizon (e.g. ``1.0``).

            """
            ...

        def n_states(self) -> int:
            """
            Return the number of states on the underlying scale.

            Returns
            -------
            int
                State count.

            """
            ...

        def default_probabilities(self) -> list[float] | None:
            """
            Return per-state default probabilities, or ``None`` if no default state.

            Returns
            -------
            list[float] or None
                Probability of default from each state, or ``None`` when the
                scale has no default absorbing state.

            """
            ...

    class GeneratorMatrix:
        """
        Continuous-time generator matrix (Q) for CTMC credit migration.

        Parameters
        ----------
        scale:
            Rating scale defining the row/column ordering.
        data:
            Row-major generator intensities (length ``n_states * n_states``).

        Raises
        ------
        ValueError
            If ``data`` length does not match ``scale.n_states() ** 2`` or rows
            do not sum to zero.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> generator = migration.GeneratorMatrix(scale, [-0.1, 0.1, 0.0, 0.0])
        >>> (generator.intensity("A", "D"), generator.exit_rate("A"), generator.n_states())
        (0.1, 0.1, 2)
        >>> (generator.to_matrix(), generator.regularization_l1, generator.round_trip_error)
        ([[-0.1, 0.1], [0.0, 0.0]], 0.0, 0.0)

        """

        def __init__(self, scale: migration.RatingScale, data: list[float]) -> None:
            """
            Construct a validated continuous-time generator matrix.

            Parameters
            ----------
            scale : migration.RatingScale
                Ordered rating states defining the row and column labels and,
                when configured, the absorbing default state.
            data : list[float]
                Row-major ``n * n`` generator entries for the ``n`` states in
                ``scale``. Off-diagonals must be non-negative, every row must
                sum to zero, and a configured default row must be absorbing.

            Raises
            ------
            ValueError
                If *data* is not an ``n x n`` row-major matrix; an off-diagonal
                entry is negative or non-finite; a row does not sum to zero; or
                the default row is not absorbing.

            """
            ...

        @staticmethod
        def from_transition_matrix(p: migration.TransitionMatrix) -> migration.GeneratorMatrix:
            """
            Estimate a generator matrix from a discrete transition matrix.

            Uses the eigendecomposition method (Israel, Rosenthal, Wei 2001).

            Parameters
            ----------
            p:
                A :class:`migration.TransitionMatrix` to invert.

            Returns
            -------
            migration.GeneratorMatrix
                Estimated generator matrix.

            Raises
            ------
            RuntimeError
                If the transition matrix has complex or non-positive
                eigenvalues, its matrix logarithm is singular, or the
                reconstructed transition matrix exceeds the round-trip
                tolerance.

            Examples
            --------
            >>> from finstack_quant.core.credit import migration
            >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
            >>> matrix = migration.TransitionMatrix(scale, [0.9, 0.1, 0.0, 1.0], 1.0)
            >>> migration.GeneratorMatrix.from_transition_matrix(matrix).round_trip_error < 1e-12
            True

            """
            ...

        def intensity(self, from_: str, to: str) -> float:
            """
            Generator intensity (migration rate) from one state to another.

            Parameters
            ----------
            from_:
                Origin rating label.
            to:
                Destination rating label.

            Returns
            -------
            float
                Generator intensity.  Diagonal entries are negative (exit rates).

            Raises
            ------
            KeyError
                If *from_* or *to* is not a label on the generator's rating scale.

            """
            ...

        def exit_rate(self, state: str) -> float:
            """
            Total exit rate (sum of off-diagonal intensities) for a state.

            Parameters
            ----------
            state:
                Rating-scale label whose total off-diagonal migration intensity
                is returned; the default absorbing state has zero exit rate.

            Returns
            -------
            float
                Non-negative exit rate.  The default absorbing state has rate 0.

            Raises
            ------
            KeyError
                If *state* is not a label on the generator's rating scale.

            """
            ...

        def to_matrix(self) -> list[list[float]]:
            """
            Return the full generator matrix as nested lists.

            Returns
            -------
            list[list[float]]
                Row-major generator matrix.

            """
            ...

        def n_states(self) -> int:
            """
            Return the number of states on the underlying scale.

            Returns
            -------
            int
                State count.

            """
            ...

        @property
        def regularization_l1(self) -> float:
            """
            L1 mass clamped by Kreinin-Sidenius regularization.

            Returns ``0.0`` for directly constructed generators.

            Returns
            -------
            float
                The regularization l1 exposed by this `migration.GeneratorMatrix`.
            """
            ...

        @property
        def round_trip_error(self) -> float:
            """
            Infinity-norm reconstruction error against the source matrix.

            Returns ``0.0`` for directly constructed generators.

            Returns
            -------
            float
                The round trip error exposed by this `migration.GeneratorMatrix`.
            """
            ...

    class RatingPath:
        """
        Simulated rating migration path over a time horizon.

        Produced by :meth:`migration.MigrationSimulator.simulate`.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> generator = migration.GeneratorMatrix(scale, [0.0, 0.0, 0.0, 0.0])
        >>> path = migration.MigrationSimulator(generator, 1.0).simulate(0, 1, 42)[0]
        >>> (path.state_at(0.5), path.label_at(0.5), path.defaulted(), path.default_time())
        (0, 'A', False, None)
        >>> (path.n_transitions(), path.transitions(), path.horizon())
        (0, [(0.0, 0)], 1.0)

        """

        def state_at(self, t: float) -> int:
            """
            Return the state index occupied at time ``t``.

            Parameters
            ----------
            t:
                Time in years within ``[0, horizon]``.

            Returns
            -------
            int
                State index at time ``t``.

            """
            ...

        def label_at(self, t: float) -> str:
            """
            Return the rating label occupied at time ``t``.

            Parameters
            ----------
            t:
                Time in years within ``[0, horizon]``.

            Returns
            -------
            str
                Rating label at time ``t``.

            """
            ...

        def defaulted(self) -> bool:
            """
            Return whether this path entered the default state.

            Returns
            -------
            bool
                ``True`` if the path defaulted at any point.

            """
            ...

        def default_time(self) -> float | None:
            """
            Return the time of default, or ``None`` if not defaulted.

            Returns
            -------
            float or None
                Default time in years, or ``None``.

            """
            ...

        def n_transitions(self) -> int:
            """
            Return the number of rating transitions in this path.

            Returns
            -------
            int
                Transition count (excluding the initial state).

            """
            ...

        def transitions(self) -> list[tuple[float, int]]:
            """
            Return all transitions as ``(time, new_state)`` pairs.

            Returns
            -------
            list[tuple[float, int]]
                Ordered list of transition events.

            """
            ...

        def horizon(self) -> float:
            """
            Return the simulation horizon in years.

            Returns
            -------
            float
                Simulation horizon in years over which this rating path is
                defined.

            """
            ...

    class MigrationSimulator:
        """
        CTMC simulator for credit rating migration paths.

        Parameters
        ----------
        generator:
            Generator matrix defining migration intensities.
        horizon:
            Simulation horizon in years.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> generator = migration.GeneratorMatrix(scale, [0.0, 0.0, 0.0, 0.0])
        >>> simulator = migration.MigrationSimulator(generator, 1.0)
        >>> (len(simulator.simulate(0, 2, 42)), simulator.horizon())
        (2, 1.0)
        >>> simulator.empirical_matrix(2, 42).to_matrix()
        [[1.0, 0.0], [0.0, 1.0]]

        """

        def __init__(self, generator: migration.GeneratorMatrix, horizon: float) -> None:
            """
            Construct a continuous-time rating-migration simulator.

            Parameters
            ----------
            generator : migration.GeneratorMatrix
                Continuous-time transition intensities and rating-state order
                used to generate each path.
            horizon : float
                Finite, strictly positive simulation horizon in years.

            Raises
            ------
            ValueError
                If *horizon* is non-finite or not strictly positive.

            """
            ...

        def simulate(
            self,
            initial_state: int,
            n_paths: int,
            seed: int,
        ) -> list[migration.RatingPath]:
            """
            Simulate rating migration paths from a single starting state.

            Parameters
            ----------
            initial_state:
                0-based index of the starting rating.
            n_paths:
                Number of independent paths to simulate.
            seed:
                Random seed for reproducibility.

            Returns
            -------
            list[migration.RatingPath]
                Simulated paths.

            Raises
            ------
            ValueError
                If *initial_state* is outside the generator's state range.

            """
            ...

        def empirical_matrix(self, n_paths_per_state: int, seed: int) -> migration.TransitionMatrix:
            """
            Estimate a transition matrix by Monte Carlo simulation.

            Simulates ``n_paths_per_state`` paths from every non-default state
            and computes the empirical transition probabilities.

            Parameters
            ----------
            n_paths_per_state:
                Number of paths to simulate per starting state.
            seed:
                Random seed for reproducibility.

            Returns
            -------
            migration.TransitionMatrix
                Empirically estimated transition matrix.

            Raises
            ------
            ValueError
                If *n_paths_per_state* is zero.

            """
            ...

        def horizon(self) -> float:
            """
            Return the simulation horizon in years.

            Returns
            -------
            float
                Configured simulation horizon in years.

            """
            ...

    @staticmethod
    def project(generator: migration.GeneratorMatrix, t: float) -> migration.TransitionMatrix:
        """
        Compute the transition matrix at time ``t`` via matrix exponential.

        Computes ``P(t) = exp(Q * t)`` where ``Q`` is the generator matrix.

        Parameters
        ----------
        generator:
            Generator matrix to project.
        t:
            Time horizon in years.

        Returns
        -------
        migration.TransitionMatrix
            Transition matrix at time ``t``.

        Raises
        ------
        ValueError
            If *t* is non-finite or not strictly positive, or the projected
            matrix fails transition-matrix validation.
        RuntimeError
            If the matrix exponential encounters a singular or numerically
            degenerate system.

        Examples
        --------
        >>> from finstack_quant.core.credit import migration
        >>> scale = migration.RatingScale.custom_with_default(["A", "D"], "D")
        >>> generator = migration.GeneratorMatrix(scale, [-0.1, 0.1, 0.0, 0.0])
        >>> round(migration.project(generator, 1.0).probability("A", "D"), 6)
        0.095163

        """
        ...
