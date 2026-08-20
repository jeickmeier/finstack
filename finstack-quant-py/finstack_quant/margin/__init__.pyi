"""
Margin and collateral: VM/IM calculators, CSA specifications, XVA, metrics.

This module exposes variation and initial margin types, netting-set identifiers,
credit support annex (CSA) specs, eligible collateral schedules, XVA configuration
and results, and margin analytics helpers.

Examples
--------
>>> from finstack_quant.margin import CollateralAssetClass
>>> CollateralAssetClass.cash().standard_haircut()
0.0
"""

from __future__ import annotations

from typing import Any, Final

import pandas as pd

from finstack_quant.core.market_data import DiscountCurve, HazardCurve
from finstack_quant.margin import schema as schema

__all__ = [
    "ImMethodology",
    "MarginTenor",
    "MarginCallType",
    "ClearingStatus",
    "CollateralAssetClass",
    "NettingSetId",
    "CsaSpec",
    "EligibleCollateralSchedule",
    "CONSTANTS",
    "VmResult",
    "VmCalculator",
    "ImResult",
    "SimmSensitivities",
    "SimmCalculator",
    "ScheduleImCalculator",
    "HaircutImCalculator",
    "FundingConfig",
    "XvaConfig",
    "ExposureDiagnostics",
    "ExposureProfile",
    "XvaResult",
    "CsaTerms",
    "XvaNettingSet",
    "ImDecayProfile",
    "ImProfile",
    "MvaResult",
    "compute_bilateral_xva",
    "compute_mva",
    "im_profile_from_simm",
    "MarginUtilization",
    "ExcessCollateral",
    "MarginFundingCost",
    "Haircut01",
    "FrtbSbaResult",
    "EadResult",
    "FrtbSensitivities",
    "FrtbSbaEngine",
    "SaCcrTrade",
    "SaCcrNettingSetConfig",
    "SaCcrEngine",
    "frtb_sba_charge",
    "saccr_ead",
    "schema",
]

CONSTANTS: Final[dict[str, str]] = ...

class ImMethodology:
    """
    Initial margin calculation methodology.

    Immutable, hashable enum-style type. Constructed via class methods;
    not directly instantiated.

    Examples
    --------
    >>> ImMethodology.from_str("simm")
    ImMethodology(simm)
    """

    @staticmethod
    def haircut() -> ImMethodology:
        """
        Haircut-based IM (repos and securities financing).

        Returns
        -------
        ImMethodology
            Haircut methodology.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> ImMethodology.haircut()
        ImMethodology(haircut)
        """
        ...

    @staticmethod
    def simm() -> ImMethodology:
        """
        ISDA SIMM (sensitivities-based, OTC derivatives).

        Returns
        -------
        ImMethodology
            SIMM methodology.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> ImMethodology.simm()
        ImMethodology(simm)
        """
        ...

    @staticmethod
    def schedule() -> ImMethodology:
        """
        BCBS-IOSCO regulatory schedule approach.

        Returns
        -------
        ImMethodology
            Schedule methodology.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> ImMethodology.schedule()
        ImMethodology(schedule)
        """
        ...

    @staticmethod
    def internal_model() -> ImMethodology:
        """
        Internal model approved by regulator.

        Returns
        -------
        ImMethodology
            Internal model methodology.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> ImMethodology.internal_model()
        ImMethodology(internal_model)
        """
        ...

    @staticmethod
    def clearing_house() -> ImMethodology:
        """
        Clearing house (CCP-specific) methodology.

        Returns
        -------
        ImMethodology
            CCP methodology.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> ImMethodology.clearing_house()
        ImMethodology(clearing_house)
        """
        ...

    @staticmethod
    def from_str(s: str) -> ImMethodology:
        """
        Parse from a string (e.g. ``"simm"``, ``"schedule"``).

        Parameters
        ----------
        s : str
            Methodology name.

        Returns
        -------
        ImMethodology
            Parsed methodology.

        Raises
        ------
        ValueError
            If the string is not recognized.

        Examples
        --------
        >>> ImMethodology.from_str("schedule")
        ImMethodology(schedule)
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class MarginTenor:
    """
    How often variation margin is exchanged under the CSA.

    Parameters
    ----------
    (Constructed via class methods; not directly instantiated.)

    Returns
    -------
    MarginTenor
        Tenor for margin calls.

    Examples
    --------
    >>> MarginTenor.daily()
    MarginTenor(daily)
    """

    @staticmethod
    def daily() -> MarginTenor:
        """
        Daily margin calls (standard for OTC derivatives post-2016).

        Returns
        -------
        MarginTenor
            Daily tenor.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> str(MarginTenor.daily())
        'daily'
        """
        ...

    @staticmethod
    def weekly() -> MarginTenor:
        """
        Weekly variation-margin call frequency.

        Returns
        -------
        MarginTenor
            Weekly tenor.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginTenor.weekly()
        MarginTenor(weekly)
        """
        ...

    @staticmethod
    def monthly() -> MarginTenor:
        """
        Monthly variation-margin call frequency.

        Returns
        -------
        MarginTenor
            Monthly tenor.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginTenor.monthly()
        MarginTenor(monthly)
        """
        ...

    @staticmethod
    def on_demand() -> MarginTenor:
        """
        Margin calls issued when a threshold breach is observed, not on a calendar.

        Returns
        -------
        MarginTenor
            On-demand tenor.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginTenor.on_demand()
        MarginTenor(on_demand)
        """
        ...

    @staticmethod
    def from_str(s: str) -> MarginTenor:
        """
        Parse this variant from its canonical name string.

        Parameters
        ----------
        s : str
            Case-insensitive margin-frequency label such as ``"daily"``,
            ``"weekly"``, or ``"on_demand"``.

        Returns
        -------
        MarginTenor
            Parsed tenor.

        Raises
        ------
        ValueError
            If the string is not recognized.

        Examples
        --------
        >>> MarginTenor.from_str("daily")
        MarginTenor(daily)
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class MarginCallType:
    """
    Kind of margin call (top-up, return, or on-demand).

    Parameters
    ----------
    (Constructed via class methods.)

    Returns
    -------
    MarginCallType
        Kind of margin call.

    Examples
    --------
    >>> MarginCallType.initial_margin()
    MarginCallType(InitialMargin)
    """

    @staticmethod
    def initial_margin() -> MarginCallType:
        """
        Initial margin posting requirement.

        Returns
        -------
        MarginCallType
            Initial margin call type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginCallType.initial_margin()
        MarginCallType(InitialMargin)
        """
        ...

    @staticmethod
    def variation_margin_delivery() -> MarginCallType:
        """
        Variation margin delivery (margin to be posted).

        Returns
        -------
        MarginCallType
            VM delivery type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginCallType.variation_margin_delivery()
        MarginCallType(VariationMarginDelivery)
        """
        ...

    @staticmethod
    def variation_margin_return() -> MarginCallType:
        """
        Variation margin return (margin to be received back).

        Returns
        -------
        MarginCallType
            VM return type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginCallType.variation_margin_return()
        MarginCallType(VariationMarginReturn)
        """
        ...

    @staticmethod
    def top_up() -> MarginCallType:
        """
        Margin call that posts additional collateral to restore the threshold.

        Returns
        -------
        MarginCallType
            Top-up type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginCallType.top_up()
        MarginCallType(TopUp)
        """
        ...

    @staticmethod
    def substitution() -> MarginCallType:
        """
        Collateral substitution request.

        Returns
        -------
        MarginCallType
            Substitution type.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> MarginCallType.substitution()
        MarginCallType(Substitution)
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...

class ClearingStatus:
    """
    Clearing status for OTC derivatives.

    Parameters
    ----------
    (Use ``bilateral()`` or ``cleared()``.)

    Returns
    -------
    ClearingStatus
        Bilateral or cleared status.

    Examples
    --------
    >>> ClearingStatus.cleared("LCH").is_cleared
    True
    """

    @staticmethod
    def bilateral() -> ClearingStatus:
        """
        Bilateral (uncleared) trade governed by CSA.

        Returns
        -------
        ClearingStatus
            Bilateral status.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> ClearingStatus.bilateral().is_bilateral
        True
        """
        ...

    @staticmethod
    def cleared(ccp: str) -> ClearingStatus:
        """
        Trade cleared through a CCP.

        Parameters
        ----------
        ccp : str
            Clearing house identifier.

        Returns
        -------
        ClearingStatus
            Cleared status with CCP id.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> ClearingStatus.cleared("LCH").is_cleared
        True
        """
        ...

    @property
    def is_bilateral(self) -> bool:
        """
        Whether this is a bilateral trade.

        Returns
        -------
        bool
            True if bilateral.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ClearingStatus.bilateral().is_bilateral
        True
        """
        ...

    @property
    def is_cleared(self) -> bool:
        """
        Whether this is a cleared trade.

        Returns
        -------
        bool
            True if cleared.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ClearingStatus.cleared("CCP").is_cleared
        True
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class CollateralAssetClass:
    """
    Collateral asset class per BCBS-IOSCO standards.

    Parameters
    ----------
    (Use class factories or ``from_str``.)

    Returns
    -------
    CollateralAssetClass
        Asset class for haircuts and eligibility.

    Examples
    --------
    >>> CollateralAssetClass.cash().standard_haircut()
    0.0
    """

    @staticmethod
    def cash() -> CollateralAssetClass:
        """
        Eligible-collateral class for cash balances.

        Returns
        -------
        CollateralAssetClass
            Cash.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> CollateralAssetClass.cash()
        CollateralAssetClass(cash)
        """
        ...

    @staticmethod
    def government_bonds() -> CollateralAssetClass:
        """
        Eligible-collateral class for government bonds.

        Returns
        -------
        CollateralAssetClass
            Government bonds.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.government_bonds()
        CollateralAssetClass(government_bonds)
        """
        ...

    @staticmethod
    def agency_bonds() -> CollateralAssetClass:
        """
        Eligible-collateral class for agency (GSE) bonds.

        Returns
        -------
        CollateralAssetClass
            Agency bonds.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.agency_bonds()
        CollateralAssetClass(agency_bonds)
        """
        ...

    @staticmethod
    def covered_bonds() -> CollateralAssetClass:
        """
        Eligible-collateral class for covered bonds.

        Returns
        -------
        CollateralAssetClass
            Covered bonds.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.covered_bonds()
        CollateralAssetClass(covered_bonds)
        """
        ...

    @staticmethod
    def corporate_bonds() -> CollateralAssetClass:
        """
        Eligible-collateral class for corporate bonds.

        Returns
        -------
        CollateralAssetClass
            Corporate bonds.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.corporate_bonds()
        CollateralAssetClass(corporate_bonds)
        """
        ...

    @staticmethod
    def equity() -> CollateralAssetClass:
        """
        Eligible-collateral class for listed equity.

        Returns
        -------
        CollateralAssetClass
            Equity.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.equity()
        CollateralAssetClass(equity)
        """
        ...

    @staticmethod
    def gold() -> CollateralAssetClass:
        """
        Eligible-collateral class for allocated gold.

        Returns
        -------
        CollateralAssetClass
            Gold.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.gold()
        CollateralAssetClass(gold)
        """
        ...

    @staticmethod
    def mutual_funds() -> CollateralAssetClass:
        """
        Eligible-collateral class for mutual-fund shares.

        Returns
        -------
        CollateralAssetClass
            Mutual funds.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> CollateralAssetClass.mutual_funds()
        CollateralAssetClass(mutual_funds)
        """
        ...

    @staticmethod
    def from_str(s: str) -> CollateralAssetClass:
        """
        Parse this variant from its canonical name string.

        Parameters
        ----------
        s : str
            Asset class name.

        Returns
        -------
        CollateralAssetClass
            Parsed class.

        Raises
        ------
        ValueError
            If not recognized.

        Examples
        --------
        >>> CollateralAssetClass.from_str("cash")
        CollateralAssetClass(cash)
        """
        ...

    def standard_haircut(self) -> float:
        """
        BCBS-IOSCO standard haircut for this asset class.

        Returns
        -------
        float
            Haircut as decimal.

        Raises
        ------
        Exception
            If the core library returns an error.

        Examples
        --------
        >>> CollateralAssetClass.cash().standard_haircut()
        0.0
        """
        ...

    def fx_addon(self) -> float:
        """
        FX haircut add-on for currency mismatch.

        Returns
        -------
        float
            Add-on as decimal.

        Raises
        ------
        Exception
            If the core library returns an error.

        Examples
        --------
        >>> isinstance(CollateralAssetClass.cash().fx_addon(), float)
        True
        """
        ...

    def __eq__(self, other: object) -> bool: ...
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class NettingSetId:
    """
    Identifies a margin netting set.

    Parameters
    ----------
    (Use ``bilateral`` or ``cleared`` factories.)

    Returns
    -------
    NettingSetId
        Netting set key.

    Examples
    --------
    >>> NettingSetId.bilateral("CPTY", "CSA1").counterparty_id
    'CPTY'
    """

    @staticmethod
    def bilateral(counterparty_id: str, csa_id: str) -> NettingSetId:
        """
        Create a bilateral netting set.

        Parameters
        ----------
        counterparty_id : str
            Counterparty identifier.
        csa_id : str
            CSA agreement identifier.

        Returns
        -------
        NettingSetId
            Bilateral netting set id.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> NettingSetId.bilateral("A", "CSA").is_cleared
        False
        """
        ...

    @staticmethod
    def cleared(ccp_id: str) -> NettingSetId:
        """
        Create a cleared netting set.

        Parameters
        ----------
        ccp_id : str
            Central counterparty identifier.

        Returns
        -------
        NettingSetId
            Cleared netting set id.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> NettingSetId.cleared("LCH").is_cleared
        True
        """
        ...

    @property
    def is_cleared(self) -> bool:
        """
        Whether this is a cleared netting set.

        Returns
        -------
        bool
            True if cleared.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> NettingSetId.cleared("CCP").is_cleared
        True
        """
        ...

    @property
    def counterparty_id(self) -> str:
        """
        Counterparty identifier. For cleared netting sets this returns
        the CCP id; for bilateral, the explicit counterparty id.

        Returns
        -------
        str
            Counterparty id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> NettingSetId.bilateral("X", "Y").counterparty_id
        'X'
        >>> NettingSetId.cleared("LCH").counterparty_id
        'LCH'
        """
        ...

    @property
    def csa_id(self) -> str | None:
        """
        CSA identifier when bilateral; ``None`` for cleared sets.

        Returns
        -------
        str or None
            CSA id string, or ``None`` for cleared netting sets.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> NettingSetId.bilateral("X", "CSA-001").csa_id
        'CSA-001'
        >>> NettingSetId.cleared("LCH").csa_id is None
        True
        """
        ...

    @property
    def ccp_id(self) -> str | None:
        """
        CCP identifier when cleared; ``None`` for bilateral sets.

        Returns
        -------
        str or None
            CCP id string, or ``None`` for bilateral netting sets.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> NettingSetId.cleared("LCH").ccp_id
        'LCH'
        >>> NettingSetId.bilateral("X", "CSA").ccp_id is None
        True
        """
        ...

    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

class CsaSpec:
    """
    Credit Support Annex specification (ISDA standard).

    Parameters
    ----------
    (Use regulatory factories or ``from_json``.)

    Returns
    -------
    CsaSpec
        CSA terms for margin calculation.

    Examples
    --------
    >>> CsaSpec.usd_regulatory().base_currency
    'USD'
    """

    @staticmethod
    def usd_regulatory() -> CsaSpec:
        """
        Standard regulatory CSA for USD derivatives.

        Returns
        -------
        CsaSpec
            USD regulatory CSA.

        Raises
        ------
        Exception
            If construction fails in the core library.

        Examples
        --------
        >>> csa = CsaSpec.usd_regulatory()
        >>> (csa.id, csa.base_currency)
        ('USD-REGULATORY-CSA', 'USD')
        """
        ...

    @staticmethod
    def eur_regulatory() -> CsaSpec:
        """
        Standard regulatory CSA for EUR derivatives.

        Returns
        -------
        CsaSpec
            EUR regulatory CSA.

        Raises
        ------
        Exception
            If construction fails in the core library.

        Examples
        --------
        >>> CsaSpec.eur_regulatory().base_currency
        'EUR'
        """
        ...

    @staticmethod
    def from_json(json: str) -> CsaSpec:
        """
        Deserialize from a JSON string.

        Parameters
        ----------
        json : str
            JSON representation.

        Returns
        -------
        CsaSpec
            Parsed CSA.

        Raises
        ------
        ValueError
            If JSON is invalid.

        Examples
        --------
        >>> csa = CsaSpec.from_json(CsaSpec.usd_regulatory().to_json())
        >>> csa.base_currency
        'USD'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a JSON string.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            If serialization fails.

        Examples
        --------
        >>> isinstance(CsaSpec.usd_regulatory().to_json(), str)
        True
        """
        ...

    @property
    def id(self) -> str:
        """
        Stable CSA identifier used in margin lookups.

        Returns
        -------
        str
            Identifier of this CSA specification.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> isinstance(CsaSpec.usd_regulatory().id, str)
        True
        """
        ...

    @property
    def base_currency(self) -> str:
        """
        ISO currency code in which CSA amounts are expressed.

        Returns
        -------
        str
            ISO currency code.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> CsaSpec.usd_regulatory().base_currency
        'USD'
        """
        ...

    @property
    def calendar_id(self) -> str:
        """
        Contractual business-day calendar identifier.

        Returns
        -------
        str
            Contractual business-day calendar identifier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def requires_im(self) -> bool:
        """
        Whether this CSA requires initial margin.

        Returns
        -------
        bool
            True if IM required.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> isinstance(CsaSpec.usd_regulatory().requires_im, bool)
        True
        """
        ...

    def validate(self) -> None:
        """
        Validate CSA identifiers and the contractual holiday calendar.

        Runs the same Rust-side validation that ``from_json`` applies on
        ingest, so an already-constructed spec can be re-checked in place.

        Returns
        -------
        None
            Returns ``None`` when the spec is valid.

        Raises
        ------
        ValueError
            If an identifier is empty or the calendar id is unknown.

        Examples
        --------
        >>> from finstack_quant.margin import CsaSpec
        >>> CsaSpec.usd_regulatory().validate() is None
        True
        """
        ...

    def __repr__(self) -> str: ...

class EligibleCollateralSchedule:
    """
    Eligible collateral schedule with haircuts.

    Parameters
    ----------
    (Use factories or ``from_json``.)

    Returns
    -------
    EligibleCollateralSchedule
        Schedule of eligible assets and haircuts.

    Examples
    --------
    >>> EligibleCollateralSchedule.cash_only().eligible_count >= 1
    True
    """

    @staticmethod
    def cash_only() -> EligibleCollateralSchedule:
        """
        Eligible-collateral schedule that accepts cash only.

        Returns
        -------
        EligibleCollateralSchedule
            Schedule with cash only.

        Raises
        ------
        Exception
            If construction fails.

        Examples
        --------
        >>> schedule = EligibleCollateralSchedule.cash_only()
        >>> (schedule.eligible_count, schedule.rehypothecation_allowed)
        (1, False)
        """
        ...

    @staticmethod
    def bcbs_standard() -> EligibleCollateralSchedule:
        """
        Standard BCBS-IOSCO compliant schedule.

        Returns
        -------
        EligibleCollateralSchedule
            BCBS schedule.

        Raises
        ------
        Exception
            If construction fails.

        Examples
        --------
        >>> EligibleCollateralSchedule.bcbs_standard().eligible_count > 0
        True
        """
        ...

    @staticmethod
    def us_treasuries() -> EligibleCollateralSchedule:
        """
        US Treasuries repo schedule.

        Returns
        -------
        EligibleCollateralSchedule
            Treasury-focused schedule.

        Raises
        ------
        Exception
            If construction fails.

        Examples
        --------
        >>> EligibleCollateralSchedule.us_treasuries().eligible_count > 0
        True
        """
        ...

    @staticmethod
    def from_json(json: str) -> EligibleCollateralSchedule:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON representation.

        Returns
        -------
        EligibleCollateralSchedule
            Parsed schedule.

        Raises
        ------
        ValueError
            If JSON is invalid.

        Examples
        --------
        >>> original = EligibleCollateralSchedule.cash_only()
        >>> restored = EligibleCollateralSchedule.from_json(original.to_json())
        >>> restored.eligible_count
        1
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            If serialization fails.

        Examples
        --------
        >>> isinstance(EligibleCollateralSchedule.cash_only().to_json(), str)
        True
        """
        ...

    @property
    def rehypothecation_allowed(self) -> bool:
        """
        Whether rehypothecation is allowed.

        Returns
        -------
        bool
            Rehypothecation flag.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> isinstance(EligibleCollateralSchedule.cash_only().rehypothecation_allowed, bool)
        True
        """
        ...

    @property
    def eligible_count(self) -> int:
        """
        Number of eligible collateral types.

        Returns
        -------
        int
            Count of eligible entries.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> EligibleCollateralSchedule.cash_only().eligible_count >= 1
        True
        """
        ...

    def is_eligible(self, asset_class: CollateralAssetClass) -> bool:
        """
        Check if an asset class is eligible.

        Parameters
        ----------
        asset_class : CollateralAssetClass
            Asset class to test.

        Returns
        -------
        bool
            True if eligible under this schedule.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.

        Examples
        --------
        >>> s = EligibleCollateralSchedule.cash_only()
        >>> s.is_eligible(CollateralAssetClass.cash())
        True
        """
        ...

    def haircut_for(self, asset_class: CollateralAssetClass) -> float | None:
        """
        Get the haircut for an asset class.

        Parameters
        ----------
        asset_class : CollateralAssetClass
            Asset class.

        Returns
        -------
        float or None
            Haircut if defined, else None.

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.

        Examples
        --------
        >>> s = EligibleCollateralSchedule.cash_only()
        >>> s.haircut_for(CollateralAssetClass.cash()) is not None
        True
        """
        ...

    def __repr__(self) -> str: ...

class VmResult:
    """
    Variation margin calculation result.

    Parameters
    ----------
    (Returned by ``VmCalculator.calculate``.)

    Returns
    -------
    VmResult
        VM amounts and call flag.

    Examples
    --------
    >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
    >>> isinstance(r.net_margin, float)
    True
    """

    @property
    def gross_exposure(self) -> float:
        """
        Gross mark-to-market exposure amount.

        Returns
        -------
        float
            Gross exposure.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> r.gross_exposure >= 0
        True
        """
        ...

    @property
    def net_exposure(self) -> float:
        """
        Net exposure after threshold and independent amount.

        Returns
        -------
        float
            Net exposure.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> isinstance(r.net_exposure, float)
        True
        """
        ...

    @property
    def delivery_amount(self) -> float:
        """
        Delivery amount (positive = we post margin).

        Returns
        -------
        float
            Delivery amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> r.delivery_amount >= 0
        True
        """
        ...

    @property
    def return_amount(self) -> float:
        """
        Return amount (positive = we receive margin back).

        Returns
        -------
        float
            Return amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> r.return_amount >= 0
        True
        """
        ...

    @property
    def net_margin(self) -> float:
        """
        Net margin amount (delivery − return).

        Returns
        -------
        float
            Net margin.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> isinstance(r.net_margin, float)
        True
        """
        ...

    @property
    def requires_call(self) -> bool:
        """
        Whether a margin call is required.

        Returns
        -------
        bool
            Call required flag.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> r = VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        >>> isinstance(r.requires_call, bool)
        True
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the result as a single-row pandas ``DataFrame``.

        Columns: ``gross_exposure``, ``net_exposure``, ``delivery_amount``,
        ``return_amount``, ``net_margin``, ``requires_call``, ``currency``.

        All amount columns are floats in the single CSA currency reported by
        ``currency``; positive ``delivery_amount`` means we post margin and
        positive ``return_amount`` means we receive margin back.

        Returns
        -------
        pd.DataFrame
            One row describing the variation-margin result.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class VmCalculator:
    """
    Variation margin calculator following ISDA CSA rules.

    Parameters
    ----------
    csa : CsaSpec
        Credit Support Annex specification.

    Returns
    -------
    VmCalculator
        Calculator bound to ``csa``.

    Examples
    --------
    >>> calc = VmCalculator(CsaSpec.usd_regulatory())
    >>> out = calc.calculate(1e6, 0.0, "USD", 2024, 6, 15)
    >>> isinstance(out, VmResult)
    True
    """

    def __init__(self, csa: CsaSpec) -> None:
        """
        Bind a variation-margin calculator to one CSA specification.

        Parameters
        ----------
        csa : CsaSpec
            Thresholds, transfer minimums, rounding rules, eligible
            currencies, and calendar terms applied to each margin call.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    def calculate(
        self,
        exposure: float,
        posted_collateral: float,
        currency: str,
        year: int,
        month: int,
        day: int,
    ) -> VmResult:
        """
        Calculate variation margin.

        Parameters
        ----------
        exposure : float
            Mark-to-market exposure.
        posted_collateral : float
            Posted collateral amount.
        currency : str
            ISO currency code.
        year : int
            Four-digit valuation year used with ``month`` and ``day`` to apply
            the CSA calendar and collateral terms.
        month : int
            As-of month (1–12).
        day : int
            Calendar day of month used with ``year`` and ``month`` for the
            variation-margin calculation date.

        Returns
        -------
        VmResult
            VM breakdown.

        Raises
        ------
        ValueError
            Invalid currency, month, or calendar date.
        Exception
            Core calculation error.

        Examples
        --------
        >>> VmCalculator(CsaSpec.usd_regulatory()).calculate(1e6, 0.0, "USD", 2024, 6, 15)
        VmResult(...)
        """
        ...

class ImResult:
    """
    Initial margin calculation result.

    Parameters
    ----------
    (Produced by IM workflows in the margin crate; exposed for typing.)

    Returns
    -------
    ImResult
        IM amount and metadata.

    Examples
    --------
    >>> calc = ScheduleImCalculator.bcbs_standard()
    >>> result = calc.calculate_for_notional(1_000_000, "USD", "interest_rate", 5.0, 2025, 1, 15)
    >>> (result.amount, result.breakdown_keys())
    (40000.0, ['interest_rate'])
    """

    @property
    def amount(self) -> float:
        """
        Calculated initial margin amount.

        Returns
        -------
        float
            IM notional.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def currency(self) -> str:
        """
        Currency of the IM amount.

        Returns
        -------
        str
            ISO currency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def methodology(self) -> ImMethodology:
        """
        Methodology used for calculation.

        Returns
        -------
        ImMethodology
            IM methodology.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def mpor_days(self) -> int:
        """
        Margin Period of Risk (days).

        Returns
        -------
        int
            MPOR in days.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def approximation(self) -> bool:
        """
        True when the amount is a conservative proxy, not an exact methodology result.

        Returns
        -------
        bool
            Whether the amount is a conservative approximation (proxy) rather
            than an exact computation under the named methodology.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def as_of(self) -> str:
        """
        Calculation date as an ISO 8601 string.

        Returns
        -------
        str
            ISO 8601 date string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def breakdown_keys(self) -> list[str]:
        """
        Risk-class breakdown keys (if available).

        Returns
        -------
        list[str]
            Keys present in the breakdown map.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.
        """
        ...

    def breakdown_amount(self, key: str) -> float | None:
        """
        Get breakdown amount for a risk class.

        Parameters
        ----------
        key : str
            Risk class key.

        Returns
        -------
        float or None
            Amount if present.

        Notes
        -----
        This method does not raise; a missing result is ``None`` rather than an exception.
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the headline result as a single-row pandas ``DataFrame``.

        Columns: ``amount``, ``currency``, ``methodology``, ``mpor_days``,
        ``as_of``, ``approximation``.

        ``amount`` is a float in ``currency``; ``mpor_days`` is the margin
        period of risk in calendar days; ``as_of`` is an ISO 8601 date string.
        ``approximation`` is ``True`` when the amount is a conservative proxy
        rather than an exact computation under the named methodology - do not
        reconcile an approximated figure against an actual margin call.
        Per-risk-class detail lives in ``to_breakdown_dataframe``.

        Returns
        -------
        pd.DataFrame
            One row describing the initial-margin result.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_breakdown_dataframe(self) -> pd.DataFrame:
        """
        Export the per-risk-class breakdown as a pandas ``DataFrame``.

        Columns: ``risk_class``, ``amount``, ``currency``. One row per risk
        class (e.g. ``"interest_rate"``, ``"credit"``, ``"equity"``), sorted
        by ``risk_class`` so repeated runs are byte-identical; the underlying
        map is unordered. Methodologies that publish no breakdown yield a
        zero-row frame that still carries all three columns.

        Breakdown components do not generally sum to ``amount``: SIMM and
        other methodologies aggregate risk classes with correlations.

        Returns
        -------
        pd.DataFrame
            One row per risk class, sorted by ``risk_class``.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

class SimmSensitivities:
    """
    ISDA SIMM sensitivity portfolio.

    Stores signed sensitivity amounts by SIMM risk class and bucket. Amounts
    are currency amounts, not percentages or spot levels. Rate and credit
    delta inputs are DV01/CS01-style amounts per 1bp move, and the
    ``base_currency`` records the currency context in which those amounts were
    produced.

    Use ``from_json``/``to_json`` for full-fidelity interop with the canonical
    Rust JSON shape, or the ``add_*`` helpers for notebook-style construction.

    Examples
    --------
    >>> from finstack_quant.margin import SimmSensitivities
    >>> sensitivities = SimmSensitivities("USD")
    >>> sensitivities.is_empty()
    True
    """

    def __init__(self, base_currency: str = "USD") -> None:
        """
        Create an empty SIMM sensitivity set.

        Parameters
        ----------
        base_currency : str, default "USD"
            ISO currency code for the currency in which the sensitivity
            amounts are expressed.

        Raises
        ------
        ValueError
            If ``base_currency`` is not a known currency code.
        """
        ...

    @staticmethod
    def from_json(json: str) -> SimmSensitivities:
        """
        Deserialize SIMM sensitivities from the canonical JSON shape.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json`` or by Rust
            ``SimmSensitivities::to_json_pretty``. Tuple-keyed Rust maps are
            represented as arrays such as ``[currency, tenor, amount]``.

        Returns
        -------
        SimmSensitivities
            Sensitivity set populated from the JSON payload.

        Raises
        ------
        ValueError
            If the payload is not valid JSON or does not match the SIMM
            sensitivity schema.

        Examples
        --------
        >>> from finstack_quant.margin import SimmSensitivities
        >>> original = SimmSensitivities("USD")
        >>> original.add_ir_delta("USD", "5Y", 1_000.0)
        >>> restored = SimmSensitivities.from_json(original.to_json())
        >>> (restored.base_currency, restored.is_empty())
        ('USD', False)
        """
        ...

    def to_json(self) -> str:
        """
        Serialize sensitivities to the canonical pretty-printed JSON shape.

        Returns
        -------
        str
            JSON string containing all populated buckets and the base currency.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def add_ir_delta(self, currency: str, tenor: str, amount: float) -> None:
        """
        Add an interest-rate delta bucket.

        Parameters
        ----------
        currency : str
            Currency risk factor, such as ``"USD"``.
        tenor : str
            SIMM tenor bucket, such as ``"2W"``, ``"1Y"``, ``"5Y"``, or
            ``"30Y"``.
        amount : float
            Signed DV01-style currency amount per 1bp move.

        Raises
        ------
        ValueError
            If ``currency`` is not a known currency code.
        """
        ...

    def add_ir_vega(self, currency: str, tenor: str, amount: float) -> None:
        """
        Add an interest-rate vega bucket.

        Parameters
        ----------
        currency : str
            Currency risk factor, such as ``"USD"``.
        tenor : str
            SIMM tenor bucket.
        amount : float
            Signed currency vega amount compatible with SIMM vega weights.

        Raises
        ------
        ValueError
            If ``currency`` is not a known currency code.
        """
        ...

    def add_credit_qualifying_delta(self, sector: str, name: str, tenor: str, amount: float) -> None:
        """
        Add a sector-bucketed credit-qualifying delta sensitivity.

        Parameters
        ----------
        sector : str
            Canonical ISDA SIMM sector, such as ``"sovereign"``,
            ``"financial"``, ``"basic_materials"``,
            ``"high_yield_financial"``, or ``"residual"``.
        name : str
            Issuer, index, or reference-entity identifier.
        tenor : str
            Credit tenor bucket, such as ``"5Y"``.
        amount : float
            Signed CS01-style currency amount per 1bp move.

        Raises
        ------
        ValueError
            If ``sector`` is not a canonical SIMM credit sector.
        """
        ...

    def add_credit_non_qualifying_delta(self, name: str, tenor: str, amount: float) -> None:
        """
        Add a credit non-qualifying delta sensitivity.

        Parameters
        ----------
        name : str
            Securitization or other explicitly non-qualifying exposure identifier.
        tenor : str
            Credit tenor bucket, such as ``"5Y"``.
        amount : float
            Signed CS01-style currency amount per 1bp move.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_equity_delta(self, underlier: str, amount: float) -> None:
        """
        Add an equity delta bucket.

        Parameters
        ----------
        underlier : str
            Equity underlier or index identifier.
        amount : float
            Signed currency sensitivity amount.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_equity_vega(self, underlier: str, amount: float) -> None:
        """
        Add an equity vega bucket.

        Parameters
        ----------
        underlier : str
            Equity underlier or index identifier.
        amount : float
            Signed currency vega amount.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_fx_delta(self, currency: str, amount: float) -> None:
        """
        Add an FX delta sensitivity to a SIMM bucket.

        Parameters
        ----------
        currency : str
            FX risk-factor currency.
        amount : float
            Signed currency sensitivity amount.

        Raises
        ------
        ValueError
            If ``currency`` is not a known currency code.
        """
        ...

    def add_fx_vega(self, ccy1: str, ccy2: str, amount: float) -> None:
        """
        Add an FX vega bucket for a currency pair.

        Parameters
        ----------
        ccy1 : str
            First currency in the FX pair.
        ccy2 : str
            Second currency in the FX pair.
        amount : float
            Signed currency vega amount.

        Raises
        ------
        ValueError
            If either currency code is unknown.
        """
        ...

    def add_commodity_delta(self, bucket: str, amount: float) -> None:
        """
        Add a commodity delta bucket.

        Parameters
        ----------
        bucket : str
            Commodity bucket label expected by the configured SIMM registry,
            such as ``"energy"``.
        amount : float
            Signed currency sensitivity amount.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_curvature(self, risk_class: str, amount: float) -> None:
        """
        Add a curvature contribution for a SIMM risk class.

        Parameters
        ----------
        risk_class : str
            SIMM risk class alias. Supported aliases include
            ``"interest_rate"``, ``"rates"``, ``"credit_qualifying"``,
            ``"credit_non_qualifying"``, ``"equity"``, ``"commodity"``,
            and ``"fx"``.
        amount : float
            Signed curvature contribution in currency units before the SIMM
            curvature scale factor is applied.

        Raises
        ------
        ValueError
            If ``risk_class`` is not recognized.
        """
        ...

    def is_empty(self) -> bool:
        """
        Return whether no sensitivity buckets have been populated.

        Returns
        -------
        bool
            ``True`` when every SIMM bucket map is empty. A populated bucket
            with a zero net amount still makes the container non-empty.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.
        """
        ...

    @property
    def base_currency(self) -> str:
        """
        Currency context in which sensitivity amounts are expressed.

        Returns
        -------
        str
            ISO currency code for sensitivity amounts.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export every populated sensitivity bucket as one long-format pandas
        ``DataFrame``.

        Columns: ``risk_class``, ``bucket``, ``tenor``, ``issuer``, ``kind``,
        ``amount``. One row per populated bucket; an empty container still
        carries all six columns. Long format is used deliberately - a column
        per bucket would give a different schema for every portfolio, and it
        matches ``FrtbSensitivities.to_dataframe``.

        ``risk_class`` uses the SIMM labels ``interest_rate``,
        ``credit_qualifying``, ``credit_non_qualifying``, ``equity``,
        ``commodity`` and ``fx``. ``kind`` is ``delta``, ``vega`` or
        ``curvature``; SIMM curvature is a single signed contribution per risk
        class, not an up/down pair.

        ``issuer`` carries the name axis: a currency code for IR and FX delta,
        a ``"CCY1/CCY2"`` pair for FX vega, an issuer or index for credit, an
        underlier for equity. It is ``None`` for commodity (keyed by bucket
        alone) and for curvature. ``bucket`` holds the SIMM credit sector for
        bucketed credit deltas (e.g. ``"sovereign"``) and the commodity bucket
        label; it is ``None`` elsewhere. ``tenor`` is the SIMM tenor bucket
        (``"2W"``, ``"1M"``, ..., ``"30Y"``) where the risk class has one.

        ``amount`` is a signed currency sensitivity in the container's base
        currency, in whatever convention the caller supplied - SIMM does not
        re-scale these on ingest.

        Rows are sorted by ``(risk_class, kind, issuer, bucket, tenor)`` so
        repeated exports of the same portfolio are identical.

        Returns
        -------
        pd.DataFrame
            One row per populated sensitivity bucket.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class SimmCalculator:
    """
    ISDA SIMM initial-margin calculator.

    Loads registry-backed SIMM parameters for the requested rule version and
    calculates initial margin from explicit ``SimmSensitivities``.

    Examples
    --------
    >>> from finstack_quant.margin import SimmCalculator
    >>> calculator = SimmCalculator("v2_6")
    >>> (calculator.version, calculator.mpor_days)
    ('v2_6', 10)
    """

    def __init__(self, version: str | None = None, mpor_days: int | None = None) -> None:
        """
        Create a SIMM calculator from the embedded margin registry.

        Parameters
        ----------
        version : str or None, optional
            SIMM version alias. Supported values include ``"v2_5"``,
            ``"2.5"``, ``"SIMM 2.5"``, ``"v2_6"``, ``"2.6"``, and
            ``"SIMM 2.6"``. When omitted, the Rust ``SimmVersion::default()``
            (currently ``"v2_6"``) is used.
        mpor_days : int | None, optional
            Optional margin period of risk override in calendar days. When
            omitted, the registry default for the SIMM version is used.

        Raises
        ------
        ValueError
            If the version is unknown or registry parameters cannot be loaded.
        """
        ...

    @property
    def version(self) -> str:
        """
        Stable SIMM version label, either ``"v2_5"`` or ``"v2_6"``.

        Returns
        -------
        str
            Normalized SIMM version label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def mpor_days(self) -> int:
        """
        Margin period of risk in calendar days.

        Returns
        -------
        int
            MPOR in calendar days.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def calculate_from_sensitivities(
        self,
        sensitivities: SimmSensitivities,
        currency: str,
        year: int,
        month: int,
        day: int,
    ) -> ImResult:
        """
        Calculate SIMM from explicit sensitivities.

        Parameters
        ----------
        sensitivities : SimmSensitivities
            Sensitivity set to aggregate.
        currency : str
            Reporting currency for the resulting margin amount.
        year : int
            Calculation year.
        month : int
            Calculation month, from 1 to 12.
        day : int
            Calculation day of month.

        Returns
        -------
        ImResult
            Initial-margin amount, methodology, MPOR, calculation date, and
            risk-class breakdown.

        Raises
        ------
        ValueError
            If the reporting currency or date is invalid.
        """
        ...

class ScheduleImCalculator:
    """
    BCBS-IOSCO regulatory schedule initial-margin calculator.

    Applies registry-backed schedule rates to explicit notionals or to a
    single-asset-class netting set with the BCBS-IOSCO net-to-gross ratio
    reduction.

    Examples
    --------
    >>> from finstack_quant.margin import ScheduleImCalculator
    >>> ScheduleImCalculator.bcbs_standard().rate("interest_rate", 5.0)
    0.04
    """

    @staticmethod
    def bcbs_standard() -> ScheduleImCalculator:
        """
        Create the embedded BCBS-IOSCO standard schedule calculator.

        Returns
        -------
        ScheduleImCalculator
            Calculator configured with the standard embedded schedule grid.

        Raises
        ------
        ValueError
            If embedded registry data cannot be loaded.

        Examples
        --------
        >>> from finstack_quant.margin import ScheduleImCalculator
        >>> ScheduleImCalculator.bcbs_standard().rate("interest_rate", 5.0)
        0.04
        """
        ...

    @staticmethod
    def from_registry_id(schedule_id: str) -> ScheduleImCalculator:
        """
        Create a schedule calculator from a registry identifier.

        Parameters
        ----------
        schedule_id : str
            Schedule identifier in the embedded margin registry.

        Returns
        -------
        ScheduleImCalculator
            Calculator configured from the matching registry entry.

        Raises
        ------
        ValueError
            If ``schedule_id`` is unknown or registry data is invalid.

        Examples
        --------
        >>> from finstack_quant.margin import ScheduleImCalculator
        >>> ScheduleImCalculator.from_registry_id("bcbs_iosco").rate("interest_rate", 5.0)
        0.04
        """
        ...

    def with_asset_class(self, asset_class: str) -> ScheduleImCalculator:
        """
        Return a copy with a new default schedule asset class.

        Parameters
        ----------
        asset_class : str
            Schedule asset class alias such as ``"interest_rate"``,
            ``"credit"``, ``"equity"``, ``"commodity"``, ``"fx"``, or
            ``"other"``.

        Returns
        -------
        ScheduleImCalculator
            Copy of this calculator with the default asset class changed.

        Raises
        ------
        ValueError
            If ``asset_class`` is not recognized.
        """
        ...

    def with_maturity(self, years: float) -> ScheduleImCalculator:
        """
        Return a copy with a new default maturity.

        Parameters
        ----------
        years : float
            Representative remaining maturity in years.

        Returns
        -------
        ScheduleImCalculator
            Copy of this calculator with the default maturity changed.

        Notes
        -----
        This builder returns a copy with the field set and does not raise.

        """
        ...

    def rate(self, asset_class: str, maturity_years: float) -> float:
        """
        Look up a decimal schedule rate.

        Parameters
        ----------
        asset_class : str
            Schedule asset class alias.
        maturity_years : float
            Remaining maturity in years.

        Returns
        -------
        float
            Decimal IM rate, e.g. ``0.01`` for 1%.

        Raises
        ------
        ValueError
            If ``asset_class`` is not recognized.
        """
        ...

    def calculate_for_notional(
        self,
        notional: float,
        currency: str,
        asset_class: str,
        maturity_years: float,
        year: int,
        month: int,
        day: int,
    ) -> ImResult:
        """
        Calculate gross schedule IM from an explicit notional.

        Parameters
        ----------
        notional : float
            Regulatory notional or caller-supplied exposure base. The schedule
            formula uses ``abs(notional)``.
        currency : str
            Currency code for the notional and result.
        asset_class : str
            Schedule asset class alias.
        maturity_years : float
            Remaining maturity used for the schedule-rate lookup.
        year : int
            Calculation year.
        month : int
            Calculation month, from 1 to 12.
        day : int
            Calculation day of month.

        Returns
        -------
        ImResult
            Gross schedule IM with a breakdown key equal to the normalized
            asset class.

        Raises
        ------
        ValueError
            If the currency, asset class, amount, or date is invalid.
        """
        ...

    def calculate_netting_set_with_ngr(
        self,
        positions: list[tuple[float, float]],
        currency: str,
        asset_class: str,
        maturity_years: float,
        year: int,
        month: int,
        day: int,
    ) -> ImResult | None:
        """
        Calculate schedule IM for a netting set using NGR.

        Applies the BCBS-IOSCO reduction ``0.4 + 0.6 * NGR`` to a
        single-asset-class set of ``(signed_mtm, gross_notional)`` positions.
        The binding assumes every tuple is in ``currency`` and that the set has
        already been partitioned by asset class.

        Parameters
        ----------
        positions : list[tuple[float, float]]
            ``(signed_mtm, gross_notional)`` pairs. MTM signs drive the NGR
            numerator; gross notionals are summed as absolute values.
        currency : str
            Reporting currency for every MTM, notional, and result.
        asset_class : str
            Schedule asset class applied uniformly to all positions.
        maturity_years : float
            Representative remaining maturity used for the rate lookup.
        year : int
            Calculation year.
        month : int
            Calculation month, from 1 to 12.
        day : int
            Calculation day of month.

        Returns
        -------
        ImResult | None
            NGR-adjusted schedule IM. Returns ``None`` for an empty position
            list, zero gross notionals, or inconsistent currencies after
            conversion to Rust money values.

        Raises
        ------
        ValueError
            If the currency, asset class, amount, or date is invalid.
        """
        ...

class HaircutImCalculator:
    """
    Haircut-based initial-margin calculator.

    Applies eligible-collateral haircuts and optional FX add-ons to explicit
    collateral values. This path is intended for repo and securities-financing
    style collateral IM rather than SIMM sensitivities.

    Examples
    --------
    >>> from finstack_quant.margin import CollateralAssetClass, HaircutImCalculator
    >>> HaircutImCalculator.bcbs_standard().haircut_for(CollateralAssetClass.cash())
    0.0
    """

    @staticmethod
    def bcbs_standard() -> HaircutImCalculator:
        """
        Create a haircut calculator with the BCBS-IOSCO schedule.

        Returns
        -------
        HaircutImCalculator
            Calculator using the embedded BCBS-IOSCO collateral haircuts.

        Raises
        ------
        ValueError
            If embedded registry data cannot be loaded.

        Examples
        --------
        >>> from finstack_quant.margin import CollateralAssetClass, HaircutImCalculator
        >>> HaircutImCalculator.bcbs_standard().haircut_for(CollateralAssetClass.cash())
        0.0
        """
        ...

    @staticmethod
    def us_treasuries() -> HaircutImCalculator:
        """
        Create a haircut calculator for US Treasury collateral.

        Returns
        -------
        HaircutImCalculator
            Calculator using the embedded US Treasuries haircut schedule.

        Raises
        ------
        ValueError
            If embedded registry data cannot be loaded.

        Examples
        --------
        >>> from finstack_quant.margin import CollateralAssetClass, HaircutImCalculator
        >>> HaircutImCalculator.us_treasuries().haircut_for(CollateralAssetClass.cash())
        0.0
        """
        ...

    @staticmethod
    def from_schedule(schedule: EligibleCollateralSchedule) -> HaircutImCalculator:
        """
        Create a haircut calculator from an eligible-collateral schedule.

        Parameters
        ----------
        schedule : EligibleCollateralSchedule
            Collateral eligibility and haircut schedule.

        Returns
        -------
        HaircutImCalculator
            Calculator backed by ``schedule``.

        Notes
        -----
        This method does not raise; it returns a fixed instance.

        Examples
        --------
        >>> from finstack_quant.margin import CollateralAssetClass, EligibleCollateralSchedule, HaircutImCalculator
        >>> schedule = EligibleCollateralSchedule.cash_only()
        >>> HaircutImCalculator.from_schedule(schedule).haircut_for(CollateralAssetClass.cash())
        0.0
        """
        ...

    def with_default_asset_class(self, asset_class: CollateralAssetClass) -> HaircutImCalculator:
        """
        Return a copy configured with a default collateral asset class.

        Parameters
        ----------
        asset_class : CollateralAssetClass
            Asset class used by trait-based calculations.

        Returns
        -------
        HaircutImCalculator
            Copy of this calculator with the default asset class changed.

        Notes
        -----
        This builder returns a copy with the field set and does not raise.

        """
        ...

    def with_posted_collateral_currency(self, currency: str) -> HaircutImCalculator:
        """
        Return a copy configured with a posted-collateral currency.

        Parameters
        ----------
        currency : str
            Currency code used to detect FX mismatch in trait-based
            calculations.

        Returns
        -------
        HaircutImCalculator
            Copy of this calculator with the collateral currency configured.

        Raises
        ------
        ValueError
            If ``currency`` is not a known currency code.
        """
        ...

    def haircut_for(self, asset_class: CollateralAssetClass) -> float:
        """
        Look up the decimal haircut for a collateral asset class.

        Parameters
        ----------
        asset_class : CollateralAssetClass
            Collateral asset class.

        Returns
        -------
        float
            Decimal haircut including only the base haircut, not the optional
            FX add-on.

        Raises
        ------
        ValueError
            If no schedule or standard haircut exists for ``asset_class``.
        """
        ...

    def calculate_for_collateral(
        self,
        collateral_value: float,
        currency: str,
        asset_class: CollateralAssetClass,
        currency_mismatch: bool,
        year: int,
        month: int,
        day: int,
    ) -> ImResult:
        """
        Calculate haircut IM from explicit collateral value and asset class.

        Parameters
        ----------
        collateral_value : float
            Collateral market value in ``currency``.
        currency : str
            Currency code for the collateral value and result.
        asset_class : CollateralAssetClass
            Collateral asset class used for the haircut lookup.
        currency_mismatch : bool
            Whether to add the asset-class FX mismatch add-on.
        year : int
            Calculation year.
        month : int
            Calculation month, from 1 to 12.
        day : int
            Calculation day of month.

        Returns
        -------
        ImResult
            Haircut IM result. The MPOR is the Rust canonical repo haircut
            horizon, currently 2 calendar days.

        Raises
        ------
        ValueError
            If the currency, amount, date, haircut, or FX add-on cannot be
            resolved.
        """
        ...

class FundingConfig:
    """
    Funding cost/benefit configuration for FVA and MVA calculation.

    Supplying ``im_profile`` turns on MVA: :func:`compute_bilateral_xva` then
    prices the funding cost of posted initial margin and includes it in
    ``XvaResult.total_xva``.

    Parameters
    ----------
    funding_spread_bp : float
        Non-negative finite funding cost spread in basis points.
    funding_benefit_bp : float | None, optional
        Non-negative finite funding benefit in bp, no greater than the cost
        spread; ``None`` for symmetric funding.
    im_profile : ImProfile | None, optional
        Valid expected initial-margin profile driving MVA; ``None`` disables MVA.
    margin_funding_spread_bp : float | None, optional
        Non-negative finite spread applied to posted IM; ``None`` reuses
        ``funding_spread_bp``.

    Returns
    -------
    FundingConfig
        Funding parameters.

    Raises
    ------
    ValueError
        If a spread is negative or non-finite, the benefit exceeds the funding
        cost, or ``im_profile`` is invalid.

    Examples
    --------
    >>> FundingConfig(50.0, None).funding_spread_bp
    50.0
    """

    def __init__(
        self,
        funding_spread_bp: float,
        funding_benefit_bp: float | None = None,
        im_profile: ImProfile | None = None,
        margin_funding_spread_bp: float | None = None,
    ) -> None:
        """
        Initialize FVA and MVA funding parameters.

        Parameters
        ----------
        funding_spread_bp : float
            Non-negative finite funding cost spread in basis points.
        funding_benefit_bp : float | None
            Non-negative finite funding benefit in basis points, no greater
            than the cost spread; ``None`` uses the funding cost spread.
        im_profile : ImProfile | None
            Valid expected initial-margin profile driving MVA, or ``None``.
        margin_funding_spread_bp : float | None
            Non-negative finite IM funding spread in bp, or ``None`` to reuse
            ``funding_spread_bp``.

        Raises
        ------
        ValueError
            If a spread is negative or non-finite, the benefit exceeds the
            funding cost, or ``im_profile`` is invalid.
        """
        ...

    @property
    def funding_spread_bp(self) -> float:
        """
        Funding spread in basis points.

        Returns
        -------
        float
            Spread in bp.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> FundingConfig(10.0).funding_spread_bp
        10.0
        """
        ...

    @property
    def funding_benefit_bp(self) -> float | None:
        """
        Funding benefit spread in basis points (or None).

        Returns
        -------
        float or None
            Benefit bp if asymmetric.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> FundingConfig(10.0, 8.0).funding_benefit_bp
        8.0
        """
        ...

    @property
    def im_profile(self) -> ImProfile | None:
        """
        Expected initial-margin profile driving MVA (or None).

        Returns
        -------
        ImProfile or None
            The IM profile when MVA is enabled.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> FundingConfig(10.0).im_profile is None
        True
        """
        ...

    @property
    def margin_funding_spread_bp(self) -> float | None:
        """
        IM funding spread in basis points (or None).

        Returns
        -------
        float or None
            Explicit IM funding spread, when overridden.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> FundingConfig(10.0, None, None, 6.0).margin_funding_spread_bp
        6.0
        """
        ...

    def effective_margin_spread_bp(self) -> float:
        """
        Effective IM funding spread in basis points.

        Falls back to ``funding_spread_bp`` when
        ``margin_funding_spread_bp`` is ``None``.

        Returns
        -------
        float
            Effective IM funding spread in bp.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> FundingConfig(10.0).effective_margin_spread_bp()
        10.0
        """
        ...

    def effective_benefit_bp(self) -> float:
        """
        Effective funding benefit spread in basis points.

        Returns
        -------
        float
            Effective benefit bp.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> isinstance(FundingConfig(1.0).effective_benefit_bp(), float)
        True
        """
        ...

    def __repr__(self) -> str: ...

class XvaConfig:
    """
    XVA calculation configuration.

    Parameters
    ----------
    time_grid : list[float] | None, optional
        Time grid in years; defaults to library default.
    recovery_rate : float | None, optional
        Counterparty recovery; defaults to library default.
    own_recovery_rate : float | None, optional
        Own recovery; optional.
    funding : FundingConfig | None, optional
        FVA funding configuration.

    Returns
    -------
    XvaConfig
        Configuration for XVA runs.

    Examples
    --------
    >>> cfg = XvaConfig()
    >>> cfg.recovery_rate > 0
    True
    """

    def __init__(
        self,
        time_grid: list[float] | None = None,
        recovery_rate: float | None = None,
        own_recovery_rate: float | None = None,
        funding: FundingConfig | None = None,
    ) -> None:
        """
        Configure exposure dates, recovery assumptions, and FVA funding.

        Parameters
        ----------
        time_grid : list[float] or None, default None
            Exposure times in years; ``None`` uses the library's standard XVA grid.
        recovery_rate : float or None, default None
            Counterparty recovery as a decimal fraction; ``None`` uses the
            library default.
        own_recovery_rate : float or None, default None
            Own recovery as a decimal fraction for DVA; ``None`` uses the
            library default.
        funding : FundingConfig or None, default None
            Funding and collateral spread assumptions for FVA; ``None``
            disables explicit funding configuration.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @staticmethod
    def from_json(json: str) -> XvaConfig:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON string.

        Returns
        -------
        XvaConfig
            Parsed config.

        Raises
        ------
        ValueError
            Invalid JSON.

        Examples
        --------
        >>> config = XvaConfig.from_json(XvaConfig(recovery_rate=0.35).to_json())
        >>> config.recovery_rate
        0.35
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            Serialization error.

        Examples
        --------
        >>> isinstance(XvaConfig().to_json(), str)
        True
        """
        ...

    def validate(self) -> None:
        """
        Validate configuration parameters.

        Returns
        -------
        None

        Raises
        ------
        Exception
            If parameters are invalid.

        Examples
        --------
        >>> XvaConfig().validate()
        """
        ...

    @property
    def time_grid(self) -> list[float]:
        """
        Time grid for exposure simulation (years from today).

        Returns
        -------
        list[float]
            Exposure or IM observation times in years from the valuation date.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> len(XvaConfig().time_grid) > 0
        True
        """
        ...

    @property
    def recovery_rate(self) -> float:
        """
        Recovery rate for counterparty default.

        Returns
        -------
        float
            Recovery fraction.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> 0 <= XvaConfig().recovery_rate <= 1
        True
        """
        ...

    @property
    def own_recovery_rate(self) -> float | None:
        """
        Recovery rate for own default (or None).

        Returns
        -------
        float or None
            Own recovery if set.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> XvaConfig(own_recovery_rate=0.4).own_recovery_rate
        0.4
        """
        ...

    def __repr__(self) -> str: ...

class ExposureDiagnostics:
    """
    Diagnostics from exposure simulation.

    Parameters
    ----------
    (Embedded in exposure results when provided by the engine.)

    Returns
    -------
    ExposureDiagnostics
        Counters for simulation health.

    Examples
    --------
    >>> try:
    ...     ExposureDiagnostics()
    ... except TypeError as exc:
    ...     print(exc)
    cannot create 'finstack_quant.margin.ExposureDiagnostics' instances
    """

    @property
    def market_roll_failures(self) -> int:
        """
        Number of market-roll failures.

        Returns
        -------
        int
            Failure count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def valuation_failures(self) -> int:
        """
        Total instrument valuation failures.

        Returns
        -------
        int
            Failure count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def total_time_points(self) -> int:
        """
        Total time grid points evaluated.

        Returns
        -------
        int
            Point count.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str: ...

class ExposureProfile:
    """
    Exposure profile at each time grid point.

    Parameters
    ----------
    times : list[float]
        Exposure or IM observation times in years from the valuation date.
    mtm_values : list[float]
        Portfolio MtM at each time.
    epe : list[float]
        Expected positive exposure series.
    ene : list[float]
        Expected negative exposure series.

    Returns
    -------
    ExposureProfile
        Profile vectors.

    Examples
    --------
    >>> p = ExposureProfile([0.0, 1.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0])
    >>> len(p)
    2
    """

    def __init__(
        self,
        times: list[float],
        mtm_values: list[float],
        epe: list[float],
        ene: list[float],
    ) -> None:
        """
        Create aligned MtM, EPE, and ENE vectors on an exposure time grid.

        Parameters
        ----------
        times : list[float]
            Exposure times in years from the valuation date.
        mtm_values : list[float]
            Portfolio mark-to-market amounts at the corresponding times.
        epe : list[float]
            Expected positive exposure amounts at the corresponding times.
        ene : list[float]
            Expected negative exposure amounts at the corresponding times.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @staticmethod
    def from_json(json: str) -> ExposureProfile:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON string.

        Returns
        -------
        ExposureProfile
            Parsed profile.

        Raises
        ------
        ValueError
            Invalid JSON.

        Examples
        --------
        >>> original = ExposureProfile([0.0, 1.0], [0.0, 2.0], [0.0, 2.0], [0.0, 0.0])
        >>> restored = ExposureProfile.from_json(original.to_json())
        >>> restored.mtm_values
        [0.0, 2.0]
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            Serialization error.

        Examples
        --------
        >>> '"times"' in ExposureProfile([1.0], [0.0], [0.0], [0.0]).to_json()
        True
        """
        ...

    def validate(self) -> None:
        """
        Validate internal consistency.

        Returns
        -------
        None

        Raises
        ------
        Exception
            If vectors are inconsistent.

        Examples
        --------
        >>> ExposureProfile([1.0], [0.0], [0.0], [0.0]).validate()
        """
        ...

    @property
    def times(self) -> list[float]:
        """
        Exposure or IM observation times in years from the valuation date.

        Returns
        -------
        list[float]
            Times.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExposureProfile([0.0, 1.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]).times
        [0.0, 1.0]
        """
        ...

    @property
    def mtm_values(self) -> list[float]:
        """
        Portfolio MtM values at each time point.

        Returns
        -------
        list[float]
            MtM path.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExposureProfile([0.0], [1.0], [0.0], [0.0]).mtm_values
        [1.0]
        """
        ...

    @property
    def epe(self) -> list[float]:
        """
        Expected Positive Exposure at each time point.

        Returns
        -------
        list[float]
            EPE series.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExposureProfile([0.0], [0.0], [2.0], [0.0]).epe
        [2.0]
        """
        ...

    @property
    def ene(self) -> list[float]:
        """
        Expected Negative Exposure at each time point.

        Returns
        -------
        list[float]
            ENE series.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExposureProfile([0.0], [0.0], [0.0], [1.0]).ene
        [1.0]
        """
        ...

    def __len__(self) -> int:
        """Number of time points.

        Returns
        -------
        int
            Length of time grid.

        Examples
        --------
        >>> len(ExposureProfile([0.0, 1.0], [0.0, 0.0], [0.0, 0.0], [0.0, 0.0]))
        2
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a pandas DataFrame with time (years) as index.

        Columns: ``mtm_values``, ``epe``, ``ene``.

        Returns
        -------
        pd.DataFrame
            Exposure profile as a DataFrame.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class XvaResult:
    """
    Result of XVA calculations (CVA, DVA, FVA, MVA, exposure profiles).

    Adjustments compose as ``total_xva = CVA - DVA + FVA + MVA``; uncomputed
    optional legs contribute zero.

    Parameters
    ----------
    (Produced by XVA engine; also loadable via ``from_json``.)

    Returns
    -------
    XvaResult
        XVA amounts and profiles.

    Examples
    --------
    >>> doc = (
    ...     '{"cva":1.0,"total_xva":1.0,"epe_profile":[[0.0,2.0]],'
    ...     '"ene_profile":[[0.0,0.0]],"pfe_profile":[[0.0,2.0]],"max_pfe":2.0,'
    ...     '"effective_epe_profile":[[0.0,2.0]],"effective_epe":2.0}'
    ... )
    >>> result = XvaResult.from_json(doc)
    >>> (result.cva, result.total_xva)
    (1.0, 1.0)
    """

    @staticmethod
    def from_json(json: str) -> XvaResult:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON string.

        Returns
        -------
        XvaResult
            Parsed result.

        Raises
        ------
        ValueError
            Invalid JSON.

        Examples
        --------
        >>> doc = (
        ...     '{"cva":1.0,"total_xva":1.0,"epe_profile":[[0.0,2.0]],'
        ...     '"ene_profile":[[0.0,0.0]],"pfe_profile":[[0.0,2.0]],"max_pfe":2.0,'
        ...     '"effective_epe_profile":[[0.0,2.0]],"effective_epe":2.0}'
        ... )
        >>> result = XvaResult.from_json(doc)
        >>> (result.max_pfe, result.effective_epe)
        (2.0, 2.0)
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            Serialization error.

        """
        ...

    @property
    def cva(self) -> float:
        """
        Unilateral CVA (positive = cost).

        Returns
        -------
        float
            CVA amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def dva(self) -> float | None:
        """
        DVA (own-default benefit, or None).

        Returns
        -------
        float or None
            DVA if computed.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def fva(self) -> float | None:
        """
        FVA (net funding cost/benefit, or None).

        Returns
        -------
        float or None
            FVA if computed.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def mva(self) -> float | None:
        """
        MVA (funding cost of posted initial margin, or None).

        ``None`` unless the run supplied ``FundingConfig.im_profile``.

        Returns
        -------
        float or None
            MVA if computed; positive is a cost to the desk.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def total_xva(self) -> float:
        """
        All-in adjustment = CVA − DVA + FVA + MVA.

        Uncomputed legs contribute zero. This is the quantity subtracted from
        the risk-free value of the netting set.

        Returns
        -------
        float
            Total XVA.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def max_pfe(self) -> float:
        """
        Maximum PFE across the profile.

        Returns
        -------
        float
            Maximum PFE across the profile.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def effective_epe(self) -> float:
        """
        Effective EPE (time-weighted average, regulatory metric).

        Returns
        -------
        float
            Effective EPE.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def epe_profile(self) -> list[tuple[float, float]]:
        """
        EPE profile as list of (time, value) tuples.

        Returns
        -------
        list[tuple[float, float]]
            (time, EPE) pairs.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def ene_profile(self) -> list[tuple[float, float]]:
        """
        ENE profile as list of (time, value) tuples.

        Returns
        -------
        list[tuple[float, float]]
            (time, ENE) pairs.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def pfe_profile(self) -> list[tuple[float, float]]:
        """
        PFE profile as list of (time, value) tuples.

        Returns
        -------
        list[tuple[float, float]]
            (time, PFE) pairs.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def effective_epe_profile(self) -> list[tuple[float, float]]:
        """
        Effective EPE profile as list of (time, value) tuples.

        Returns
        -------
        list[tuple[float, float]]
            (time, effective EPE) pairs.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the XVA components as a single-row pandas DataFrame.

        Columns: ``cva``, ``dva``, ``fva``, ``mva``, ``total_xva``,
        ``max_pfe``, ``effective_epe`` -- all in the netting set's currency
        units, matching the properties of the same name. Uncomputed legs
        (``dva`` / ``fva`` / ``mva``) are ``NaN`` rather than absent, so the
        frame keeps its schema across netting sets.

        This is the default export; the time-indexed exposure profiles are a
        separate table -- see :meth:`to_profiles_dataframe`.

        Returns
        -------
        pd.DataFrame
            Single-row DataFrame, so a portfolio of netting sets stacks with
            ``pd.concat``.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def to_profiles_dataframe(self) -> pd.DataFrame:
        """
        Export exposure profiles as a pandas DataFrame.

        Columns: ``epe``, ``ene``, ``pfe``, ``effective_epe`` -- indexed
        by time in years.

        Returns
        -------
        pd.DataFrame
            Profile DataFrame.

        Notes
        -----
        This accessor does not raise; it returns the stored or derived value.
        """
        ...

    def __repr__(self) -> str: ...

class CsaTerms:
    """
    Credit Support Annex terms for XVA collateralization.

    Parameters
    ----------
    threshold : float
        Threshold below which no collateral is required.
    mta : float
        Minimum transfer amount.
    mpor_days : int
        Margin period of risk in calendar days.
    independent_amount : float
        Independent amount (initial margin).

    Returns
    -------
    CsaTerms
        Collateral terms for XVA.

    Examples
    --------
    >>> CsaTerms(0.0, 0.0, 10, 0.0).mpor_days
    10
    """

    def __init__(
        self,
        threshold: float,
        mta: float,
        mpor_days: int,
        independent_amount: float,
    ) -> None:
        """
        Set collateral threshold, transfer minimum, MPOR, and independent amount.

        Parameters
        ----------
        threshold : float
            Unsecured exposure amount allowed before collateral is required,
            in the netting set's reporting currency.
        mta : float
            Minimum transfer amount in the reporting currency.
        mpor_days : int
            Margin period of risk in calendar days.
        independent_amount : float
            Independent amount or initial margin in the reporting currency.

        Notes
        -----
        Construction does not raise; arguments are stored as supplied.
        """
        ...

    @property
    def threshold(self) -> float:
        """
        Threshold below which no collateral is required.

        Returns
        -------
        float
            Threshold amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> CsaTerms(1e6, 0.0, 5, 0.0).threshold
        1000000.0
        """
        ...

    @property
    def mta(self) -> float:
        """
        Minimum transfer amount.

        Returns
        -------
        float
            MTA.

            Minimum transfer amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> CsaTerms(0.0, 5e4, 5, 0.0).mta
        50000.0
        """
        ...

    @property
    def mpor_days(self) -> int:
        """
        Margin period of risk in calendar days.

        Returns
        -------
        int
            Margin period of risk in calendar days.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> CsaTerms(0.0, 0.0, 14, 0.0).mpor_days
        14
        """
        ...

    @property
    def independent_amount(self) -> float:
        """
        Independent amount (initial margin).

        Returns
        -------
        float
            IA amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> CsaTerms(0.0, 0.0, 5, 1e5).independent_amount
        100000.0
        """
        ...

    def __repr__(self) -> str: ...

class XvaNettingSet:
    """
    XVA netting set: trades under a single ISDA master agreement.

    Parameters
    ----------
    id : str
        Identifier of the XVA netting set in the CSA graph.
    counterparty_id : str
        Counterparty identifier.
    csa : CsaTerms | None, optional
        Collateral terms if collateralized.
    reporting_currency : str | None, optional
        ISO currency for reporting.

    Returns
    -------
    XvaNettingSet
        Netting set descriptor.

    Raises
    ------
    ValueError
        If ``reporting_currency`` is not a valid currency code.

    Examples
    --------
    >>> XvaNettingSet("NS1", "CPTY").is_collateralized
    False
    """

    def __init__(
        self,
        id: str,
        counterparty_id: str,
        csa: CsaTerms | None = None,
        reporting_currency: str | None = None,
    ) -> None:
        """
        Create a counterparty netting set with optional collateral terms.

        Parameters
        ----------
        id : str
            Stable netting-set identifier carried into XVA results.
        counterparty_id : str
            Counterparty identifier used to group the netting set's exposures.
        csa : CsaTerms or None, default None
            Collateral agreement applied to the netting set; ``None`` models
            an uncollateralized agreement.
        reporting_currency : str or None, default None
            ISO-4217 currency code for XVA amounts; ``None`` leaves the
            reporting currency unspecified.

        Raises
        ------
        ValueError
            If a supplied ``reporting_currency`` is not a recognized ISO currency code.
        """
        ...

    @property
    def id(self) -> str:
        """
        Identifier of the XVA netting set in the CSA graph.

        Returns
        -------
        str
            Identifier of the XVA netting set in the CSA graph.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> XvaNettingSet("A", "B").id
        'A'
        """
        ...

    @property
    def counterparty_id(self) -> str:
        """
        Counterparty identifier.

        Returns
        -------
        str
            Counterparty id.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> XvaNettingSet("A", "CP").counterparty_id
        'CP'
        """
        ...

    @property
    def is_collateralized(self) -> bool:
        """
        Whether this netting set is collateralized.

        Returns
        -------
        bool
            True if CSA terms are set.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> XvaNettingSet("A", "B", CsaTerms(0, 0, 5, 0)).is_collateralized
        True
        """
        ...

    def __repr__(self) -> str: ...

class ImDecayProfile:
    """
    Deterministic IM decay profile for MVA (Green 2015, ch. 10).

    Approximates the expected initial-margin path ``E[IM(t)] = IM(0) *
    factor(t)`` used by :func:`im_profile_from_simm`. Constructed via the
    static factories below; there is no direct constructor.

    Parameters
    ----------
    (Use ``constant()``, ``linear_to_maturity()``, or ``sqrt_time()``.)

    Returns
    -------
    ImDecayProfile
        Decay profile applied to today's IM.

    Examples
    --------
    >>> ImDecayProfile.constant().factor(5.0)
    1.0
    """

    @staticmethod
    def constant() -> ImDecayProfile:
        """
        IM stays at today's level for the whole horizon.

        Returns
        -------
        ImDecayProfile
            Decay profile with ``factor(t) == 1`` for all ``t``.

        Notes
        -----
        This factory does not raise; it returns a new instance with the documented defaults.

        Examples
        --------
        >>> ImDecayProfile.constant().factor(10.0)
        1.0
        """
        ...

    @staticmethod
    def linear_to_maturity(maturity_years: float) -> ImDecayProfile:
        """
        IM decays linearly to zero at ``maturity_years``.

        Parameters
        ----------
        maturity_years : float
            Portfolio maturity ``T`` in years; must be positive and finite.

        Returns
        -------
        ImDecayProfile
            Decay profile with ``factor(t) = max(1 - t/T, 0)``.

        Raises
        ------
        ValueError
            If ``maturity_years`` is non-positive or non-finite.

        Examples
        --------
        >>> ImDecayProfile.linear_to_maturity(2.0).factor(1.0)
        0.5
        """
        ...

    @staticmethod
    def sqrt_time(maturity_years: float) -> ImDecayProfile:
        """
        IM decays like the square root of remaining time to ``maturity_years``.

        Parameters
        ----------
        maturity_years : float
            Portfolio maturity ``T`` in years; must be positive and finite.

        Returns
        -------
        ImDecayProfile
            Decay profile with ``factor(t) = sqrt(max(1 - t/T, 0))``.

        Raises
        ------
        ValueError
            If ``maturity_years`` is non-positive or non-finite.

        Examples
        --------
        >>> round(ImDecayProfile.sqrt_time(2.0).factor(1.0), 6)
        0.707107
        """
        ...

    def factor(self, t: float) -> float:
        """
        IM decay multiplier applied at time ``t`` in the MVA profile.

        Parameters
        ----------
        t : float
            Time in years from the valuation date.

        Returns
        -------
        float
            Decay factor, always in ``[0, 1]`` for ``t >= 0``.

        Raises
        ------
        (None)
            This is a pure arithmetic evaluation; it never raises.

        Examples
        --------
        >>> ImDecayProfile.constant().factor(3.0)
        1.0
        """
        ...

    @staticmethod
    def from_json(json: str) -> ImDecayProfile:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON representation produced by ``to_json``.

        Returns
        -------
        ImDecayProfile
            Parsed decay profile.

        Raises
        ------
        ValueError
            If the payload is not valid JSON or does not match the schema.

        Examples
        --------
        >>> ImDecayProfile.from_json(ImDecayProfile.constant().to_json())
        ImDecayProfile(Constant)
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.

        Examples
        --------
        >>> isinstance(ImDecayProfile.constant().to_json(), str)
        True
        """
        ...

    def __repr__(self) -> str: ...

class ImProfile:
    """
    Expected initial-margin profile ``E[IM(t)]`` on a time grid.

    Values are in the aggregation currency chosen when the profile was
    built (e.g. the ``currency`` argument of :func:`im_profile_from_simm`).

    Parameters
    ----------
    times : list[float]
        Time points in years from the valuation date; strictly increasing
        and positive.
    im_values : list[float]
        Expected IM at each time point; non-negative and finite.

    Returns
    -------
    ImProfile
        IM profile as constructed; not validated until ``validate()`` is
        called or the profile is passed to ``compute_mva`` or
        ``im_profile_from_simm``.

    Examples
    --------
    >>> ImProfile([1.0, 2.0], [100.0, 50.0]).times
    [1.0, 2.0]
    """

    def __init__(self, times: list[float], im_values: list[float]) -> None:
        """
        Construct from time and IM vectors.

        Values are stored as given and are not validated at construction
        time. Call ``validate()`` explicitly, or rely on downstream
        functions (``compute_mva``, ``im_profile_from_simm``) to reject an
        inconsistent profile when it is used.

        Parameters
        ----------
        times : list[float]
            Time points in years; strictly increasing and positive.
        im_values : list[float]
            Expected IM at each time point; non-negative and finite.

        Raises
        ------
        (None)
            Construction never raises; values are stored unchecked and
            validated only by ``validate()`` or downstream consumers.

        Examples
        --------
        >>> ImProfile([1.0, 2.0], [100.0, 50.0]).times
        [1.0, 2.0]
        """
        ...

    @staticmethod
    def from_json(json: str) -> ImProfile:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON representation produced by ``to_json``.

        Returns
        -------
        ImProfile
            Parsed IM profile.

        Raises
        ------
        ValueError
            If the payload is not valid JSON or does not match the schema.

        Examples
        --------
        >>> ImProfile.from_json(ImProfile([1.0], [1.0]).to_json()).times
        [1.0]
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.

        Examples
        --------
        >>> isinstance(ImProfile([1.0], [1.0]).to_json(), str)
        True
        """
        ...

    def validate(self) -> None:
        """
        Validate internal consistency.

        Raises
        ------
        ValueError
            If the profile is empty, lengths differ, times are not
            strictly increasing and positive, or IM values are
            negative/non-finite.

        Examples
        --------
        >>> ImProfile([1.0], [1.0]).validate()
        """
        ...

    @property
    def times(self) -> list[float]:
        """
        Exposure or IM observation times in years from the valuation date.

        Returns
        -------
        list[float]
            Strictly increasing, positive time points.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ImProfile([1.0, 2.0], [1.0, 2.0]).times
        [1.0, 2.0]
        """
        ...

    @property
    def im_values(self) -> list[float]:
        """
        Expected IM at each time point.

        Returns
        -------
        list[float]
            Non-negative IM values in the profile's aggregation currency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ImProfile([1.0], [100.0]).im_values
        [100.0]
        """
        ...

    def __len__(self) -> int:
        """
        Number of time points.

        Returns
        -------
        int
            Length of ``times`` (and ``im_values``).

        Examples
        --------
        >>> len(ImProfile([1.0, 2.0], [1.0, 2.0]))
        2
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export as a pandas ``DataFrame`` with time (years) as index.

        Returns
        -------
        pandas.DataFrame
            Single column ``im``, indexed by time in years.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.

        Examples
        --------
        >>> df = ImProfile([1.0, 2.0], [100.0, 50.0]).to_dataframe()
        >>> list(df.columns)
        ['im']
        """
        ...

    def __repr__(self) -> str: ...

class MvaResult:
    """
    Result of an MVA computation.

    All monetary quantities are ``float`` in the IM profile's currency.
    Returned by :func:`compute_mva`.

    Parameters
    ----------
    (Returned by ``compute_mva``; not directly instantiated.)

    Returns
    -------
    MvaResult
        MVA value, average IM, and echoed IM profile.

    Examples
    --------
    >>> result = MvaResult.from_json('{"mva":1.0,"average_im":100.0,"im_profile":[[1.0,100.0]]}')
    >>> (result.mva, result.average_im)
    (1.0, 100.0)
    """

    @staticmethod
    def from_json(json: str) -> MvaResult:
        """
        Parse this object from a JSON object or JSON string.

        Parameters
        ----------
        json : str
            JSON representation produced by ``to_json``.

        Returns
        -------
        MvaResult
            Parsed MVA result.

        Raises
        ------
        ValueError
            If the payload is not valid JSON or does not match the schema.

        Examples
        --------
        >>> result = MvaResult.from_json('{"mva":1.0,"average_im":100.0,"im_profile":[[1.0,100.0]]}')
        >>> result.im_profile
        [(1.0, 100.0)]
        """
        ...

    def to_json(self) -> str:
        """
        Serialize this object to a JSON-compatible dict.

        Returns
        -------
        str
            Pretty-printed JSON.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def mva(self) -> float:
        """
        MVA (positive = lifetime funding cost of posting IM).

        Returns
        -------
        float
            MVA amount in the IM profile's currency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def average_im(self) -> float:
        """
        Time-weighted average IM over the profile horizon.

        Returns
        -------
        float
            ``(1/T) * integral_0^T IM(t) dt`` under the same trapezoid
            convention as ``mva``, in the IM profile's currency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def im_profile(self) -> list[tuple[float, float]]:
        """
        IM profile used, as ``(time, value)`` tuples.

        Returns
        -------
        list[tuple[float, float]]
            ``(time_years, im_value)`` pairs echoing the input profile.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the IM profile as a pandas ``DataFrame``.

        Returns
        -------
        pandas.DataFrame
            Single column ``im``, indexed by time in years.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

    def __repr__(self) -> str: ...

class MarginUtilization:
    """
    Margin utilization result (ratio of posted to required margin).

    Parameters
    ----------
    posted_amount : float
        Margin amount actually posted, in the CSA currency.
    required_amount : float
        Margin amount required by the CSA calculation, in the CSA currency.
    currency : str
        ISO currency code (both amounts use this currency).

    Returns
    -------
    MarginUtilization
        Utilization metrics.

    Raises
    ------
    ValueError
        Invalid currency code.

    Examples
    --------
    >>> u = MarginUtilization(100.0, 100.0, "USD")
    >>> u.is_adequate()
    True
    """

    def __init__(
        self,
        posted_amount: float,
        required_amount: float,
        currency: str,
    ) -> None:
        """
        Compare posted and required margin in one reporting currency.

        Parameters
        ----------
        posted_amount : float
            Margin already posted, in ``currency`` units.
        required_amount : float
            Margin requirement, in the same ``currency`` units.
        currency : str
            ISO-4217 code shared by both amounts.

        Raises
        ------
        ValueError
            If ``currency`` is unrecognized, or either amount is non-finite or
            outside the representable monetary range.
        """
        ...

    @property
    def posted(self) -> float:
        """
        Margin amount actually posted, in the CSA currency.

        Returns
        -------
        float
            Posted amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginUtilization(10.0, 20.0, "USD").posted
        10.0
        """
        ...

    @property
    def required(self) -> float:
        """
        Margin amount required by the CSA calculation, in the CSA currency.

        Returns
        -------
        float
            Required amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginUtilization(10.0, 20.0, "USD").required
        20.0
        """
        ...

    @property
    def ratio(self) -> float:
        """
        Utilization ratio (posted / required).

        Returns
        -------
        float
            Ratio.

            Utilization ratio (posted / required).

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginUtilization(50.0, 100.0, "EUR").ratio
        0.5
        """
        ...

    def is_adequate(self) -> bool:
        """
        Whether margin is adequate (ratio >= 1.0).

        Returns
        -------
        bool
            Adequacy flag.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.

        Examples
        --------
        >>> MarginUtilization(100.0, 100.0, "USD").is_adequate()
        True
        """
        ...

    def shortfall(self) -> float:
        """
        Shortfall amount (if any).

        Returns
        -------
        float
            Shortfall in currency units.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> MarginUtilization(0.0, 100.0, "USD").shortfall() >= 0
        True
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the result as a single-row pandas ``DataFrame``.

        Columns: ``posted``, ``required``, ``ratio``, ``shortfall``,
        ``is_adequate``, ``currency``.

        ``posted``, ``required`` and ``shortfall`` are floats in ``currency``.
        ``ratio`` is ``posted / required`` as a decimal fraction (``1.0`` =
        fully covered); it is ``inf`` when nothing is required but margin is
        posted, and ``1.0`` when neither is.

        Returns
        -------
        pd.DataFrame
            One row describing margin utilization.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class ExcessCollateral:
    """
    Excess collateral result.

    Parameters
    ----------
    collateral_value : float
        Collateral mark.
    required_value : float
        Required collateral.
    currency : str
        ISO currency code.

    Returns
    -------
    ExcessCollateral
        Excess or shortfall view.

    Raises
    ------
    ValueError
        Invalid currency.

    Examples
    --------
    >>> ExcessCollateral(120.0, 100.0, "USD").has_excess()
    True
    """

    def __init__(
        self,
        collateral_value: float,
        required_value: float,
        currency: str,
    ) -> None:
        """
        Compare collateral value with the required collateral amount.

        Parameters
        ----------
        collateral_value : float
            Current collateral mark, in ``currency`` units.
        required_value : float
            Required collateral amount in the same currency.
        currency : str
            ISO-4217 code shared by both values.

        Raises
        ------
        ValueError
            If ``currency`` is unrecognized, or either amount is non-finite or
            outside the representable monetary range.
        """
        ...

    @property
    def collateral_value(self) -> float:
        """
        Market value of posted collateral in the CSA currency.

        Returns
        -------
        float
            Collateral mark.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExcessCollateral(10.0, 5.0, "USD").collateral_value
        10.0
        """
        ...

    @property
    def required_value(self) -> float:
        """
        Required collateral amount in the same currency as the mark.

        Returns
        -------
        float
            Requirement.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExcessCollateral(10.0, 5.0, "USD").required_value
        5.0
        """
        ...

    @property
    def excess(self) -> float:
        """
        Excess amount (positive) or shortfall (negative).

        Returns
        -------
        float
            Net excess.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> ExcessCollateral(10.0, 5.0, "USD").excess > 0
        True
        """
        ...

    def has_excess(self) -> bool:
        """
        Whether there is excess collateral.

        Returns
        -------
        bool
            True if excess > 0.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.

        Examples
        --------
        >>> ExcessCollateral(2.0, 1.0, "USD").has_excess()
        True
        """
        ...

    def has_shortfall(self) -> bool:
        """
        Whether there is a shortfall.

        Returns
        -------
        bool
            True if under-collateralized.

        Notes
        -----
        This method does not raise; it returns ``True`` or ``False``.

        Examples
        --------
        >>> ExcessCollateral(1.0, 2.0, "USD").has_shortfall()
        True
        """
        ...

    def excess_percentage(self) -> float:
        """
        Excess as a percentage of required.

        Returns
        -------
        float
            Fractional excess vs required.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> isinstance(ExcessCollateral(110.0, 100.0, "USD").excess_percentage(), float)
        True
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the result as a single-row pandas ``DataFrame``.

        Columns: ``collateral_value``, ``required_value``, ``excess``,
        ``excess_percentage``, ``has_excess``, ``has_shortfall``,
        ``currency``.

        The three amount columns are floats in ``currency``; ``excess`` is
        ``collateral_value - required_value`` and is negative on a shortfall.
        ``excess_percentage`` is a decimal fraction of ``required_value``
        (``0.1`` = 10% over-collateralised).

        Returns
        -------
        pd.DataFrame
            One row describing the collateral position.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class MarginFundingCost:
    """
    Margin funding cost result.

    Parameters
    ----------
    margin_posted : float
        Margin amount actually posted, in the CSA currency.
    funding_rate : float
        Funding rate (annualized).
    collateral_rate : float
        Rate earned on posted collateral, as a decimal.
    currency : str
        ISO currency code.

    Returns
    -------
    MarginFundingCost
        Annual and periodic funding cost view.

    Raises
    ------
    ValueError
        Invalid currency.

    Examples
    --------
    >>> m = MarginFundingCost(1e6, 0.05, 0.01, "USD")
    >>> m.spread() == 0.04
    True
    """

    def __init__(
        self,
        margin_posted: float,
        funding_rate: float,
        collateral_rate: float,
        currency: str,
    ) -> None:
        """
        Describe the annual funding spread and cost of posted margin.

        Parameters
        ----------
        margin_posted : float
            Posted margin principal, in ``currency`` units.
        funding_rate : float
            Annual funding rate as a decimal, such as ``0.05`` for 5%.
        collateral_rate : float
            Annual collateral remuneration rate as a decimal.
        currency : str
            ISO-4217 code for the margin and calculated costs.

        Raises
        ------
        ValueError
            If ``currency`` is unrecognized, or ``margin_posted`` is non-finite
            or outside the representable monetary range.
        """
        ...

    @property
    def margin_posted(self) -> float:
        """
        Margin amount actually posted, in the CSA currency.

        Returns
        -------
        float
            Margin posted.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginFundingCost(1.0, 0.1, 0.0, "USD").margin_posted
        1.0
        """
        ...

    @property
    def funding_rate(self) -> float:
        """
        Funding rate (annualized).

        Returns
        -------
        float
            Funding rate.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginFundingCost(1.0, 0.06, 0.02, "USD").funding_rate
        0.06
        """
        ...

    @property
    def collateral_rate(self) -> float:
        """
        Rate earned on posted collateral, as a decimal.

        Returns
        -------
        float
            Collateral rate.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginFundingCost(1.0, 0.06, 0.02, "USD").collateral_rate
        0.02
        """
        ...

    @property
    def annual_cost(self) -> float:
        """
        Annualized funding cost.

        Returns
        -------
        float
            Annual cost amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> MarginFundingCost(1e6, 0.05, 0.0, "USD").annual_cost > 0
        True
        """
        ...

    def spread(self) -> float:
        """
        Funding spread (funding rate − collateral rate).

        Returns
        -------
        float
            Net spread.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> round(MarginFundingCost(0.0, 0.05, 0.02, "USD").spread(), 2)
        0.03
        """
        ...

    def cost_for_period(self, year_fraction: float) -> float:
        """
        Cost for a specific period.

        Parameters
        ----------
        year_fraction : float
            Length of period in years.

        Returns
        -------
        float
            Cost over the period.

        Notes
        -----
        This method does not raise; out-of-domain or non-finite inputs yield ``NaN`` or ``inf`` rather than an exception.

        Examples
        --------
        >>> MarginFundingCost(1e6, 0.04, 0.0, "USD").cost_for_period(0.5) >= 0
        True
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export the result as a single-row pandas ``DataFrame``.

        Columns: ``margin_posted``, ``funding_rate``, ``collateral_rate``,
        ``spread``, ``annual_cost``, ``currency``.

        ``margin_posted`` and ``annual_cost`` are floats in ``currency``; the
        three rate columns are annualized decimal fractions (``0.03`` = 3%).
        ``annual_cost`` is ``margin_posted * spread``, so a collateral rate
        above the funding rate makes it negative (a funding benefit).

        Returns
        -------
        pd.DataFrame
            One row describing the funding cost.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class Haircut01:
    """
    Haircut sensitivity: PV change for +1bp haircut change.

    Parameters
    ----------
    collateral_value : float
        Collateral mark.
    current_haircut : float
        Current haircut as decimal.
    currency : str
        ISO currency code.

    Returns
    -------
    Haircut01
        Sensitivity metrics.

    Raises
    ------
    ValueError
        Invalid currency.

    Examples
    --------
    >>> h = Haircut01(1e6, 0.05, "USD")
    >>> isinstance(h.pv_change, float)
    True
    """

    def __init__(
        self,
        collateral_value: float,
        current_haircut: float,
        currency: str,
    ) -> None:
        """
        Measure collateral-value sensitivity to a one-basis-point haircut increase.

        Parameters
        ----------
        collateral_value : float
            Pre-haircut collateral mark, in ``currency`` units.
        current_haircut : float
            Current haircut as a decimal fraction, such as ``0.05`` for 5%.
        currency : str
            ISO-4217 code for the collateral value and PV change.

        Raises
        ------
        ValueError
            If ``currency`` is unrecognized, or ``collateral_value`` is
            non-finite or outside the representable monetary range.
        """
        ...

    @property
    def collateral_value(self) -> float:
        """
        Market value of posted collateral in the CSA currency.

        Returns
        -------
        float
            Collateral mark.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> Haircut01(100.0, 0.1, "USD").collateral_value
        100.0
        """
        ...

    @property
    def current_haircut(self) -> float:
        """
        Current haircut (decimal).

        Returns
        -------
        float
            Haircut.

            Current haircut (decimal).

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> Haircut01(100.0, 0.1, "USD").current_haircut
        0.1
        """
        ...

    @property
    def pv_change(self) -> float:
        """
        PV change for +1bp haircut.

        Returns
        -------
        float
            Sensitivity amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.

        Examples
        --------
        >>> isinstance(Haircut01(1e6, 0.05, "USD").pv_change, float)
        True
        """
        ...

    def haircut_bp(self) -> float:
        """
        Current haircut in basis points.

        Returns
        -------
        float
            Haircut in bp.

        Notes
        -----
        This method does not raise; it returns the stored or derived value.

        Examples
        --------
        >>> Haircut01(1.0, 0.01, "USD").haircut_bp()
        100.0
        """
        ...

    def __repr__(self) -> str: ...

class FrtbSensitivities:
    """
    FRTB sensitivity portfolio for the Sensitivity-Based Approach.

    Build up delta / vega / curvature inputs with the ``add_*`` methods, then
    pass to :func:`frtb_sba_charge` to compute the capital charge under one or
    more correlation scenarios per BCBS d457.

    Parameters
    ----------
    base_currency : str, default "USD"
        Reporting / base currency ISO code.

    Examples
    --------
    >>> sens = FrtbSensitivities("USD")
    >>> sens.add_girr_delta("5Y", 100_000.0)
    """

    def __init__(self, base_currency: str = "USD") -> None:
        """
        Create an empty FRTB sensitivity set in one reporting currency.

        Parameters
        ----------
        base_currency : str, default "USD"
            Recognized ISO-4217 currency code used for all SBA sensitivities
            and capital amounts.

        Raises
        ------
        ValueError
            If ``base_currency`` is not a recognized ISO currency code.
        """
        ...

    @staticmethod
    def from_json(json: str) -> FrtbSensitivities:
        """
        Construct from a JSON serialization.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        FrtbSensitivities
            Sensitivity set populated from the JSON payload.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not match the serialized
            ``FrtbSensitivities`` schema.

        Examples
        --------
        >>> from finstack_quant.margin import FrtbSensitivities
        >>> original = FrtbSensitivities("USD")
        >>> original.add_girr_delta("5Y", 100_000.0)
        >>> restored = FrtbSensitivities.from_json(original.to_json())
        >>> restored.base_currency
        'USD'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a JSON string.

        Returns
        -------
        str
            JSON serialization of the sensitivity portfolio.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    def add_girr_delta(self, tenor: str, amount: float, currency: str | None = None) -> None:
        """
        Add a GIRR delta sensitivity (currency P&L per 1 percentage-point move).

        Parameters
        ----------
        tenor : str
            GIRR tenor bucket, such as ``"5Y"``.
        amount : float
            Signed sensitivity amount per 1 percentage-point move (``100 * DV01``).
        currency : str, optional
            Currency code; defaults to the base currency.

        Raises
        ------
        ValueError
            If a supplied ``currency`` is not a recognized ISO currency code.
        """
        ...

    def add_csr_delta(self, issuer: str, bucket: int, tenor: str, amount: float) -> None:
        """
        Add a CSR (non-securitization) delta sensitivity.

        Parameters
        ----------
        issuer : str
            Issuer or reference-entity identifier.
        bucket : int
            CSR bucket number.
        tenor : str
            Credit tenor bucket.
        amount : float
            Signed sensitivity amount per 1bp move.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_equity_delta(self, underlier: str, bucket: int, amount: float) -> None:
        """
        Add an equity delta sensitivity.

        Parameters
        ----------
        underlier : str
            Equity underlier or index identifier.
        bucket : int
            Equity bucket number.
        amount : float
            Signed sensitivity amount per 1bp move.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_fx_delta(self, ccy1: str, ccy2: str, amount: float) -> None:
        """
        Add an FX delta sensitivity for the pair (ccy1, ccy2).

        Parameters
        ----------
        ccy1 : str
            First currency in the FX pair.
        ccy2 : str
            Second currency in the FX pair.
        amount : float
            Signed sensitivity amount per 1bp move.

        Raises
        ------
        ValueError
            If ``ccy1`` or ``ccy2`` is not a recognized ISO currency code.
        """
        ...

    def add_commodity_delta(self, name: str, bucket: int, tenor: str, amount: float) -> None:
        """
        Add a commodity delta sensitivity.

        Parameters
        ----------
        name : str
            Commodity identifier.
        bucket : int
            Commodity bucket number.
        tenor : str
            Commodity tenor bucket.
        amount : float
            Signed sensitivity amount per 1bp move.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_girr_vega(
        self,
        option_maturity: str,
        underlying_tenor: str,
        amount: float,
        currency: str | None = None,
    ) -> None:
        """
        Add a GIRR vega sensitivity.

        Parameters
        ----------
        option_maturity : str
            Option maturity bucket.
        underlying_tenor : str
            Underlying tenor bucket.
        amount : float
            Signed vega amount.
        currency : str, optional
            Currency code; defaults to the base currency.

        Raises
        ------
        ValueError
            If a supplied ``currency`` is not a recognized ISO currency code.
        """
        ...

    def add_equity_vega(self, underlier: str, bucket: int, maturity: str, amount: float) -> None:
        """
        Add an equity vega sensitivity.

        Parameters
        ----------
        underlier : str
            Equity underlier or index identifier.
        bucket : int
            Equity bucket number.
        maturity : str
            Option maturity bucket.
        amount : float
            Signed vega amount.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_fx_vega(self, ccy1: str, ccy2: str, maturity: str, amount: float) -> None:
        """
        Add an FX vega sensitivity.

        Parameters
        ----------
        ccy1 : str
            First currency in the FX pair.
        ccy2 : str
            Second currency in the FX pair.
        maturity : str
            Option maturity bucket.
        amount : float
            Signed vega amount.

        Raises
        ------
        ValueError
            If ``ccy1`` or ``ccy2`` is not a recognized ISO currency code.
        """
        ...

    def add_girr_curvature(self, cvr_up: float, cvr_down: float, currency: str | None = None) -> None:
        """
        Add a GIRR curvature sensitivity.

        Parameters
        ----------
        cvr_up : float
            Curvature sensitivity for upward shock.
        cvr_down : float
            Curvature sensitivity for downward shock.
        currency : str, optional
            Currency code; defaults to the base currency.

        Raises
        ------
        ValueError
            If a supplied ``currency`` is not a recognized ISO currency code.
        """
        ...

    def add_equity_curvature(self, underlier: str, bucket: int, cvr_up: float, cvr_down: float) -> None:
        """
        Add an equity curvature sensitivity.

        Parameters
        ----------
        underlier : str
            Equity underlier or index identifier.
        bucket : int
            Equity bucket number.
        cvr_up : float
            Curvature sensitivity for upward shock.
        cvr_down : float
            Curvature sensitivity for downward shock.

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    def add_fx_curvature(self, ccy1: str, ccy2: str, cvr_up: float, cvr_down: float) -> None:
        """
        Add an FX curvature sensitivity.

        Parameters
        ----------
        ccy1 : str
            First currency in the FX pair.
        ccy2 : str
            Second currency in the FX pair.
        cvr_up : float
            Curvature sensitivity for upward shock.
        cvr_down : float
            Curvature sensitivity for downward shock.

        Raises
        ------
        ValueError
            If ``ccy1`` or ``ccy2`` is not a recognized ISO currency code.
        """
        ...

    def add_rrao_position(self, instrument_id: str, notional: float, is_exotic: bool = False) -> None:
        """
        Add a Residual Risk Add-On position.

        Parameters
        ----------
        instrument_id : str
            Instrument identifier.
        notional : float
            Notional amount for the RRAO position.
        is_exotic : bool, default False
            Whether the instrument is exotic (higher RRAO weight).

        Notes
        -----
        This method does not raise; it updates stored state in place.
        """
        ...

    @property
    def base_currency(self) -> str:
        """
        Base / reporting currency code.

        Returns
        -------
        str
            ISO currency code for FRTB reporting.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def __repr__(self) -> str: ...
    def to_dataframe(self) -> pd.DataFrame:
        """
        Export every populated sensitivity bucket as one long-format pandas
        ``DataFrame``.

        Columns: ``risk_class``, ``bucket``, ``tenor``, ``issuer``, ``kind``,
        ``amount``. One row per populated bucket; an empty container still
        carries all six columns. Long format is used deliberately - a column
        per bucket would give a different schema for every portfolio.

        ``risk_class`` uses the same labels as the ``frtb_sba_charge``
        breakdown (``girr``, ``csr_non_sec``, ``csr_sec_ctp``,
        ``csr_sec_non_ctp``, ``equity``, ``commodity``, ``fx``), plus ``drc``
        and ``rrao`` for the two position lists.

        ``kind`` is ``delta``, ``vega``, ``curvature_up``, ``curvature_down``,
        ``inflation_delta``, ``xccy_basis_delta``, ``jtd`` (DRC notional), or
        ``exotic_notional`` / ``other_notional`` (RRAO). A curvature pair is
        split across two rows so ``amount`` stays scalar.

        ``issuer`` carries the name axis: a currency code for GIRR, a
        ``"CCY1/CCY2"`` pair for FX, an issuer, tranche, underlier, commodity
        name, or instrument id elsewhere. ``bucket`` is the FRTB bucket index
        as a **string** (``pd.to_numeric`` if you need it numeric); ``tenor``
        is the tenor or option maturity, and for GIRR vega the
        ``"{option_maturity}/{underlying_tenor}"`` pair. Both are ``None``
        where the risk class has no such axis.

        ``amount`` keeps each bucket's own convention: GIRR deltas are
        base-currency P&L per **1 percentage point** of curve shift (that is,
        ``100 x DV01``), DRC rows are signed JTD notionals before LGD, and
        RRAO rows are gross notionals.

        Rows are sorted by ``(risk_class, kind, issuer, bucket, tenor)`` so
        repeated exports of the same portfolio are identical.

        Returns
        -------
        pd.DataFrame
            One row per populated sensitivity bucket.

        Raises
        ------
        ValueError
            If the result cannot be serialized into a pandas object.
        """
        ...

class FrtbSbaEngine:
    """
    FRTB SBA engine matching the canonical Rust API.

    Calculates the Sensitivity-Based Approach capital charge under one or more
    correlation scenarios per BCBS d457.

    Parameters
    ----------
    correlation_scenario : str or None, optional
        If provided (``"low"``, ``"medium"``, or ``"high"``), only that
        scenario is evaluated. Otherwise all three are run.

    Examples
    --------
    >>> from finstack_quant.margin import FrtbSbaEngine, FrtbSensitivities
    >>> sensitivities = FrtbSensitivities("USD")
    >>> sensitivities.add_girr_delta("5Y", 100_000.0)
    >>> round(FrtbSbaEngine("medium").calculate(sensitivities).total, 2)
    110000.0
    """

    def __init__(self, correlation_scenario: str | None = None) -> None:
        """
        Select the correlation scenarios evaluated by the FRTB SBA engine.

        Parameters
        ----------
        correlation_scenario : str or None, default None
            ``"low"``, ``"medium"``, or ``"high"`` to evaluate one BCBS
            correlation scenario; ``None`` evaluates all three.

        Raises
        ------
        ValueError
            If ``correlation_scenario`` is not ``"low"``, ``"medium"``, or
            ``"high"``.
        """
        ...

    def calculate(self, sensitivities: FrtbSensitivities) -> FrtbSbaResult:
        """
        Calculate the FRTB SBA charge for a sensitivity portfolio.

        Parameters
        ----------
        sensitivities : FrtbSensitivities
            Portfolio of FRTB sensitivities (delta, vega, curvature, DRC, RRAO).

        Returns
        -------
        FrtbSbaResult
            Total charge with the per-risk-class delta/vega/curvature
            breakdown, DRC, RRAO, and the per-scenario charges.

        Raises
        ------
        ValueError
            If a sensitivity has an unsupported tenor or bucket, an empty
            required identifier, or a non-finite numeric value.
        """
        ...

class FrtbSbaResult:
    """
    FRTB SBA capital-charge result (BCBS d457).

    Returned by :func:`frtb_sba_charge` and :meth:`FrtbSbaEngine.calculate`.
    Amounts are floats in the sensitivity portfolio's base currency.

    Examples
    --------
    >>> from finstack_quant.margin import FrtbSensitivities, frtb_sba_charge
    >>> sens = FrtbSensitivities("USD")
    >>> sens.add_girr_delta("5Y", 100_000.0)
    >>> result = frtb_sba_charge(sens)
    >>> result.total > 0.0
    True
    >>> result.binding_scenario in {"low", "medium", "high"}
    True
    """

    def __reduce__(self) -> tuple[Any, tuple[str]]:
        """Support ``pickle`` via the ``to_json`` / ``from_json`` round-trip."""
        ...

    @staticmethod
    def from_json(json: str) -> FrtbSbaResult:
        """
        Deserialize from the JSON produced by ``to_json``.

        Parameters
        ----------
        json : str
            JSON-encoded ``FrtbSbaResult``.

        Returns
        -------
        FrtbSbaResult
            The decoded result.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for the ``FrtbSbaResult`` shape.

        Examples
        --------
        >>> from finstack_quant.margin import FrtbSbaResult
        >>> restored = FrtbSbaResult.from_json(result.to_json())  # doctest: +SKIP
        >>> restored.to_json() == result.to_json()  # doctest: +SKIP
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize back to the same JSON shape ``from_json`` accepts.

        Returns
        -------
        str
            JSON-encoded ``FrtbSbaResult``.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    @property
    def total(self) -> float:
        """
        Total capital charge, in the portfolio's base currency.

        Returns
        -------
        float
            Total capital charge, in the portfolio's base currency.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def drc(self) -> float:
        """
        Default Risk Charge (credit + equity jump-to-default).

        Returns
        -------
        float
            Default Risk Charge (credit + equity jump-to-default).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rrao(self) -> float:
        """
        Residual Risk Add-On.

        Returns
        -------
        float
            Residual Risk Add-On.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def binding_scenario(self) -> str:
        """
        Correlation scenario that bound: ``"low"``, ``"medium"``, or ``"high"``.

        Returns
        -------
        str
            Correlation scenario that bound: ``"low"``, ``"medium"``, or ``"high"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def delta_by_risk_class(self) -> dict[str, float]:
        """
        Delta risk charge keyed by risk-class wire label (e.g. ``"girr"``).

        Returns
        -------
        dict[str, float]
            Delta risk charge keyed by risk-class wire label (e.g. ``"girr"``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def vega_by_risk_class(self) -> dict[str, float]:
        """
        Vega risk charge keyed by risk-class wire label.

        Returns
        -------
        dict[str, float]
            Vega risk charge keyed by risk-class wire label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def curvature_by_risk_class(self) -> dict[str, float]:
        """
        Curvature risk charge keyed by risk-class wire label.

        Returns
        -------
        dict[str, float]
            Curvature risk charge keyed by risk-class wire label.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def scenario_charges(self) -> dict[str, float]:
        """
        Delta+vega+curvature charge under each evaluated correlation scenario.

        Returns
        -------
        dict[str, float]
            Delta+vega+curvature charge under each evaluated correlation scenario.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> Any:
        """
        Export the headline charge as a single-row pandas ``DataFrame``.

        Returns
        -------
        pandas.DataFrame
            Columns ``total``, ``drc``, ``rrao`` (floats) and
            ``binding_scenario`` (string). One row.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    def to_breakdown_dataframe(self) -> Any:
        """
        Export the per-risk-class breakdown as a long-format ``DataFrame``.

        Returns
        -------
        pandas.DataFrame
            Columns ``component`` (``"delta"``/``"vega"``/``"curvature"``),
            ``risk_class`` and ``charge``. Components do not sum to ``total``:
            SBA aggregates them with prescribed correlations, and ``drc`` /
            ``rrao`` sit outside this frame.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    def __repr__(self) -> str:
        """
        Return ``repr(self)``.

        Returns
        -------
        str
            Return ``repr(self)``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class EadResult:
    """
    SA-CCR Exposure at Default result (BCBS 279).

    Returned by :func:`saccr_ead` and :meth:`SaCcrEngine.calculate_ead`.
    ``ead == alpha * (rc + pfe)`` and ``pfe == multiplier * add_on_aggregate``;
    amounts are floats in the netting set's reporting currency.

    Examples
    --------
    >>> from finstack_quant.margin import SaCcrEngine, SaCcrNettingSetConfig
    >>> config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)
    >>> SaCcrEngine().calculate_ead(config, []).ead
    0.0
    """

    def __reduce__(self) -> tuple[Any, tuple[str]]:
        """Support ``pickle`` via the ``to_json`` / ``from_json`` round-trip."""
        ...

    @staticmethod
    def from_json(json: str) -> EadResult:
        """
        Deserialize from the JSON produced by ``to_json``.

        Parameters
        ----------
        json : str
            JSON-encoded ``EadResult``.

        Returns
        -------
        EadResult
            The decoded result.

        Raises
        ------
        ValueError
            If ``json`` is not valid JSON for the ``EadResult`` shape.

        Examples
        --------
        >>> from finstack_quant.margin import EadResult
        >>> restored = EadResult.from_json(result.to_json())  # doctest: +SKIP
        >>> restored.to_json() == result.to_json()  # doctest: +SKIP
        True
        """
        ...

    def to_json(self) -> str:
        """
        Serialize back to the same JSON shape ``from_json`` accepts.

        Returns
        -------
        str
            JSON-encoded ``EadResult``.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    @property
    def ead(self) -> float:
        """
        Exposure at Default: ``alpha * (rc + pfe)``.

        Returns
        -------
        float
            Exposure at Default: ``alpha * (rc + pfe)``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def rc(self) -> float:
        """
        Replacement cost component.

        Returns
        -------
        float
            Replacement cost component.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def pfe(self) -> float:
        """
        Potential future exposure: ``multiplier * add_on_aggregate``.

        Returns
        -------
        float
            Potential future exposure: ``multiplier * add_on_aggregate``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def multiplier(self) -> float:
        """
        PFE multiplier recognising over-collateralization (floored at 0.05).

        Returns
        -------
        float
            PFE multiplier recognising over-collateralization (floored at 0.05).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def add_on_aggregate(self) -> float:
        """
        Aggregate add-on across asset classes, before the multiplier.

        Returns
        -------
        float
            Aggregate add-on across asset classes, before the multiplier.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def alpha(self) -> float:
        """
        Alpha multiplier (1.4 per BCBS 279 unless overridden on the engine).

        Returns
        -------
        float
            Alpha multiplier (1.4 per BCBS 279 unless overridden on the engine).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def maturity_factor(self) -> float:
        """
        Maturity factor applied to the netting set.

        Returns
        -------
        float
            Maturity factor applied to the netting set.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def add_on_by_asset_class(self) -> dict[str, float]:
        """
        Add-on keyed by asset-class wire label (e.g. ``"interest_rate"``).

        Returns
        -------
        dict[str, float]
            Add-on keyed by asset-class wire label (e.g. ``"interest_rate"``).

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    def to_dataframe(self) -> Any:
        """
        Export the headline exposure as a single-row pandas ``DataFrame``.

        Returns
        -------
        pandas.DataFrame
            Columns ``ead``, ``rc``, ``pfe``, ``multiplier``,
            ``add_on_aggregate``, ``alpha``, ``maturity_factor``. One row.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    def to_add_on_dataframe(self) -> Any:
        """
        Export the per-asset-class add-on as a pandas ``DataFrame``.

        Returns
        -------
        pandas.DataFrame
            Columns ``asset_class`` and ``add_on``, one row per asset class
            present. A netting set with no trades yields a zero-row frame that
            still carries both columns with their real dtypes.

        Notes
        -----
        This method does not raise; it derives the value from stored state.
        """
        ...

    def __repr__(self) -> str:
        """
        Return ``repr(self)``.

        Returns
        -------
        str
            Return ``repr(self)``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class SaCcrTrade:
    """
    A derivative trade for SA-CCR EAD computation per BCBS 279.

    Positional construction is intentionally unavailable. Use ``from_json``
    with every canonical trade field so supervisory delta, direction, and
    option classification remain explicit and are validated together.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.margin import SaCcrTrade
    >>> payload = {
    ...     "trade_id": "t1",
    ...     "asset_class": "interest_rate",
    ...     "notional": 1_000_000,
    ...     "start_date": "2025-01-01",
    ...     "end_date": "2030-01-01",
    ...     "underlier": "USD-SOFR",
    ...     "hedging_set": "rates",
    ...     "direction": 1.0,
    ...     "supervisory_delta": 1.0,
    ...     "mtm": 0.0,
    ...     "is_option": False,
    ...     "option_type": None,
    ... }
    >>> trade = SaCcrTrade.from_json(json.dumps(payload))
    >>> (trade.trade_id, trade.asset_class)
    ('t1', 'interest_rate')
    """

    @staticmethod
    def from_json(json: str) -> SaCcrTrade:
        """
        Construct from a JSON serialization.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        SaCcrTrade
            Parsed trade.

        Raises
        ------
        ValueError
            If ``json`` is malformed, does not match the canonical schema, or
            violates direction, supervisory-delta, or option-type invariants.

        Examples
        --------
        >>> import json
        >>> from finstack_quant.margin import SaCcrTrade
        >>> payload = {
        ...     "trade_id": "t1",
        ...     "asset_class": "interest_rate",
        ...     "notional": 1_000_000,
        ...     "start_date": "2025-01-01",
        ...     "end_date": "2030-01-01",
        ...     "underlier": "USD-SOFR",
        ...     "hedging_set": "rates",
        ...     "direction": 1.0,
        ...     "supervisory_delta": 1.0,
        ...     "mtm": 0.0,
        ...     "is_option": False,
        ...     "option_type": None,
        ... }
        >>> SaCcrTrade.from_json(json.dumps(payload)).trade_id
        't1'
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a JSON string.

        Returns
        -------
        str
            JSON serialization of the trade.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def trade_id(self) -> str:
        """
        Unique trade identifier.

        Returns
        -------
        str
            Trade id string.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def asset_class(self) -> str:
        """
        SA-CCR asset-class label used to select supervisory factors.

        Returns
        -------
        str
            One of ``"ir"``, ``"fx"``, ``"credit"``, ``"equity"``, ``"commodity"``.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def notional(self) -> float:
        """
        Adjusted notional in reporting currency.

        Returns
        -------
        float
            Notional amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def mtm(self) -> float:
        """
        Current mark-to-market value.

        Returns
        -------
        float
            MtM value.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...
    def __repr__(self) -> str: ...

class SaCcrNettingSetConfig:
    """
    SA-CCR netting-set configuration with explicit valuation date.

    Parameters
    ----------
    (Use ``un margined`` or ``margined`` factories.)

    Examples
    --------
    >>> from finstack_quant.margin import SaCcrNettingSetConfig
    >>> config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)
    >>> config.is_margined
    False
    """

    @staticmethod
    def unmargined(
        counterparty_id: str,
        csa_id: str,
        collateral: float,
        as_of_year: int,
        as_of_month: int,
        as_of_day: int,
    ) -> SaCcrNettingSetConfig:
        """
        Create an unmargined netting-set configuration.

        Parameters
        ----------
        counterparty_id : str
            Counterparty identifier.
        csa_id : str
            CSA agreement identifier.
        collateral : float
            Net collateral currently held.
        as_of_year : int
            Valuation year.
        as_of_month : int
            Valuation month (1–12).
        as_of_day : int
            Valuation day.

        Returns
        -------
        SaCcrNettingSetConfig
            Unmargined netting-set config.

        Raises
        ------
        ValueError
            If the supplied valuation date is not a valid calendar date.

        Examples
        --------
        >>> from finstack_quant.margin import SaCcrNettingSetConfig
        >>> config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)
        >>> config.is_margined
        False
        """
        ...

    @staticmethod
    def margined(
        counterparty_id: str,
        csa_id: str,
        collateral: float,
        threshold: float,
        mta: float,
        nica: float,
        mpor_days: int,
        as_of_year: int,
        as_of_month: int,
        as_of_day: int,
    ) -> SaCcrNettingSetConfig:
        """
        Create a margined netting-set configuration.

        Parameters
        ----------
        counterparty_id : str
            Counterparty identifier.
        csa_id : str
            CSA agreement identifier.
        collateral : float
            Net collateral currently held.
        threshold : float
            Threshold below which no collateral is required.
        mta : float
            Minimum transfer amount.
        nica : float
            Net independent collateral amount.
        mpor_days : int
            Margin period of risk in calendar days.
        as_of_year : int
            Valuation year.
        as_of_month : int
            Valuation month (1–12).
        as_of_day : int
            Valuation day.

        Returns
        -------
        SaCcrNettingSetConfig
            Margined netting-set config.

        Raises
        ------
        ValueError
            If the supplied valuation date is not a valid calendar date.

        Examples
        --------
        >>> from finstack_quant.margin import SaCcrNettingSetConfig
        >>> config = SaCcrNettingSetConfig.margined("CPTY", "CSA", 0.0, 0.0, 0.0, 0.0, 10, 2025, 1, 15)
        >>> config.is_margined
        True
        """
        ...

    @staticmethod
    def from_json(json: str) -> SaCcrNettingSetConfig:
        """
        Construct from a JSON serialization.

        Parameters
        ----------
        json : str
            JSON string produced by ``to_json``.

        Returns
        -------
        SaCcrNettingSetConfig
            Parsed netting-set config.

        Raises
        ------
        ValueError
            If ``json`` is malformed or does not match the serialized
            ``SaCcrNettingSetConfig`` schema.

        Examples
        --------
        >>> from finstack_quant.margin import SaCcrNettingSetConfig
        >>> original = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)
        >>> SaCcrNettingSetConfig.from_json(original.to_json()).is_margined
        False
        """
        ...

    def to_json(self) -> str:
        """
        Serialize to a JSON string.

        Returns
        -------
        str
            JSON serialization of the netting-set config.

        Raises
        ------
        ValueError
            If the value cannot be serialized to JSON.
        """
        ...

    @property
    def is_margined(self) -> bool:
        """
        Whether the netting set is margined.

        Returns
        -------
        bool
            True if subject to daily margin agreement.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

    @property
    def collateral(self) -> float:
        """
        Net collateral currently held.

        Returns
        -------
        float
            Collateral amount.

        Notes
        -----
        This accessor does not raise; it returns the stored value.
        """
        ...

class SaCcrEngine:
    """
    SA-CCR EAD engine matching the canonical Rust API.

    Parameters
    ----------
    alpha : float or None, optional
        Supervisory alpha factor; defaults to 1.4 when ``None``.

    Monetary values in a calculation must already use one consistent currency;
    the engine does not perform currency conversion.

    Examples
    --------
    >>> from finstack_quant.margin import SaCcrEngine, SaCcrNettingSetConfig
    >>> config = SaCcrNettingSetConfig.unmargined("CPTY", "CSA", 0.0, 2025, 1, 15)
    >>> SaCcrEngine().calculate_ead(config, []).ead
    0.0
    """

    def __init__(self, alpha: float | None = None) -> None:
        """
        Configure the supervisory multiplier for SA-CCR.

        Parameters
        ----------
        alpha : float or None, default None
            Supervisory alpha multiplier; ``None`` uses the regulatory default
            of ``1.4`` and explicit values must be at least ``1.0``.

        Raises
        ------
        ValueError
            If a supplied ``alpha`` is non-finite or less than ``1.0``.
        """
        ...

    def calculate_ead(self, config: SaCcrNettingSetConfig, trades: list[SaCcrTrade]) -> EadResult:
        """
        Calculate SA-CCR EAD for a netting set and trade list.

        Parameters
        ----------
        config : SaCcrNettingSetConfig
            Netting-set configuration with valuation date and collateral.
        trades : list[SaCcrTrade]
            Derivative trades in the netting set.

        Returns
        -------
        EadResult
            Exposure at default with replacement cost, PFE, the multiplier,
            the aggregate and per-asset-class add-ons, alpha, and the
            maturity factor.

        Raises
        ------
        ValueError
            If netting-set amounts or trade numeric fields are non-finite, a
            threshold or MTA is negative, a margined MPOR is zero, or a trade's
            direction and supervisory-delta fields are inconsistent.
        """
        ...

def im_profile_from_simm(
    calculator: SimmCalculator,
    sensitivities: SimmSensitivities,
    currency: str,
    decay: ImDecayProfile,
    time_grid: list[float],
) -> ImProfile:
    """
    Build a deterministic IM profile from current SIMM sensitivities.

    Computes ``IM(t) = SIMM(sensitivities) * decay(t)`` on ``time_grid``,
    where the base IM is the full cross-risk-class ISDA SIMM aggregate from
    ``calculator.calculate_from_sensitivities(sensitivities, currency)``.

    Parameters
    ----------
    calculator : SimmCalculator
        SIMM calculator fixing the version/registry parameters.
    sensitivities : SimmSensitivities
        Current portfolio sensitivities in SIMM buckets.
    currency : str
        Aggregation currency for the SIMM total.
    decay : ImDecayProfile
        Deterministic decay profile applied to the base IM.
    time_grid : list[float]
        Strictly increasing, positive year fractions.

    Returns
    -------
    ImProfile
        Expected IM profile ``E[IM(t)]`` on ``time_grid``.

    Raises
    ------
    ValueError
        If ``currency`` is not a known currency code, or ``decay`` or
        ``time_grid`` fails validation.

    Examples
    --------
    >>> sens = SimmSensitivities("USD")
    >>> sens.add_ir_delta("USD", "5Y", 50_000.0)
    >>> calc = SimmCalculator("v2_6")
    >>> decay = ImDecayProfile.linear_to_maturity(4.0)
    >>> profile = im_profile_from_simm(calc, sens, "USD", decay, [1.0, 2.0, 4.0])
    >>> profile.times
    [1.0, 2.0, 4.0]
    """
    ...

def compute_mva(
    im_profile: ImProfile,
    funding_spread_curve: list[tuple[float, float]],
    discount_curve: DiscountCurve,
    survival_curve: HazardCurve | None = None,
) -> MvaResult:
    """
    Compute MVA over an expected-IM profile.

    Integrates ``spread(t) * IM(t) * DF(t) * S(t)`` over the profile's time
    grid using the same midpoint/trapezoid convention as the CVA engine,
    with a ``t = 0`` bucket edge (``DF(0) = 1``, ``S(0) = 1``). IM is
    treated as flat (left-constant) before the first grid point.

    Parameters
    ----------
    im_profile : ImProfile
        Expected IM profile ``E[IM(t)]`` (from ``im_profile_from_simm`` or
        the stochastic engine's mean per-path IM).
    funding_spread_curve : list[tuple[float, float]]
        ``(time_years, spread_bp)`` pairs in basis points, linearly
        interpolated with flat extrapolation; a single pair means a flat
        spread.
    discount_curve : DiscountCurve
        Risk-free discount curve.
    survival_curve : HazardCurve or None, optional
        Optional bank (own) hazard curve; when ``None``, survival
        probability ``S(t)`` is treated as 1 for all ``t``.

    Returns
    -------
    MvaResult
        MVA, time-weighted average IM, and the echoed IM profile.

    Raises
    ------
    ValueError
        If the profile or spread curve fails validation, or if any curve
        evaluation returns a non-finite value.

    Examples
    --------
    >>> from datetime import date
    >>> from finstack_quant.core.market_data import DiscountCurve
    >>> profile = ImProfile([1.0, 2.0], [1_000_000.0, 1_000_000.0])
    >>> result = compute_mva(
    ...     profile,
    ...     [(0.0, 50.0)],
    ...     DiscountCurve.flat("USD-OIS", date(2025, 1, 1), 0.0),
    ... )
    >>> round(result.mva, 2)
    10000.0
    """
    ...

def frtb_sba_charge(sensitivities: FrtbSensitivities, correlation_scenario: str | None = None) -> FrtbSbaResult:
    """
    Compute the FRTB SBA capital charge.

    Parameters
    ----------
    sensitivities : FrtbSensitivities
        Portfolio of FRTB sensitivities (delta, vega, curvature, DRC, RRAO).
    correlation_scenario : str or None, optional
        If provided (``"low"``, ``"medium"``, or ``"high"``), only that scenario
        is evaluated. Otherwise all three are run and the max-binding one is
        reported per BCBS d457.

    Returns
    -------
    FrtbSbaResult
        Total charge with the per-risk-class delta/vega/curvature breakdown,
        DRC, RRAO, and the per-scenario charges with the binding one named.

    Raises
    ------
    ValueError
        If ``correlation_scenario`` is unknown, or a sensitivity has an
        unsupported tenor or bucket, an empty required identifier, or a
        non-finite numeric value.

    Examples
    --------
    >>> sens = FrtbSensitivities("USD")
    >>> sens.add_girr_delta("5Y", 100_000.0)
    >>> result = frtb_sba_charge(sens)
    >>> result.total > 0.0
    True
    """
    ...

def saccr_ead(
    trades: list[SaCcrTrade],
    as_of_year: int,
    as_of_month: int,
    as_of_day: int,
    margined: bool = False,
    collateral: float = 0.0,
    threshold: float | None = None,
    mta: float | None = None,
    nica: float | None = None,
    mpor_days: int | None = None,
    counterparty_id: str = "CPTY",
    csa_id: str = "CSA",
) -> EadResult:
    """
    Compute SA-CCR Exposure at Default per BCBS 279.

    Parameters
    ----------
    trades : list[SaCcrTrade]
        Derivative trades making up the netting set.
    as_of_year : int
        Valuation-date calendar year used for remaining maturity.
    as_of_month : int
        Valuation-date month in ``1..12``.
    as_of_day : int
        Valuation-date day of month.
    margined : bool, default False
        Whether the netting set is subject to a daily margin agreement.
    collateral : float, default 0.0
        Net collateral currently held (positive = bank holds collateral).
    threshold : float or None
        CSA threshold; only consumed when ``margined`` is ``True``.
    mta : float or None
        Minimum transfer amount; only consumed when ``margined`` is ``True``.
    nica : float or None
        Net independent collateral amount; only consumed when ``margined`` is ``True``.
    mpor_days : int or None
        Margin period of risk in business days; only consumed when ``margined`` is ``True``.
    counterparty_id : str, default ``CPTY``
        Counterparty identifier used to build the bilateral netting-set id.
    csa_id : str, default ``CSA``
        CSA identifier used to build the bilateral netting-set id.

    Returns
    -------
    EadResult
        ``ead = alpha * (rc + pfe)`` with alpha = 1.4, together with the
        multiplier, the aggregate and per-asset-class add-ons, and the
        maturity factor.

    Raises
    ------
    ValueError
        If ``trades`` is empty, ``collateral`` or a trade numeric field is
        non-finite, a trade's direction and supervisory-delta fields are
        inconsistent, the as-of date is invalid, or margined-only CSA terms
        are supplied when ``margined`` is ``False``.

    Examples
    --------
    >>> import json
    >>> from finstack_quant.margin import SaCcrTrade, saccr_ead
    >>> payload = {
    ...     "trade_id": "t1",
    ...     "asset_class": "interest_rate",
    ...     "notional": 1_000_000,
    ...     "start_date": "2025-01-01",
    ...     "end_date": "2030-01-01",
    ...     "underlier": "USD-SOFR",
    ...     "hedging_set": "rates",
    ...     "direction": 1.0,
    ...     "supervisory_delta": 1.0,
    ...     "mtm": 0.0,
    ...     "is_option": False,
    ...     "option_type": None,
    ... }
    >>> trade = SaCcrTrade.from_json(json.dumps(payload))
    >>> result = saccr_ead([trade], 2025, 1, 1)
    >>> (round(result.rc, 2), round(result.pfe, 2), round(result.ead, 2))
    (0.0, 22130.59, 30982.83)
    """
    ...

def compute_bilateral_xva(
    exposure_profile: ExposureProfile,
    counterparty_hazard_curve: HazardCurve,
    own_hazard_curve: HazardCurve,
    discount_curve: DiscountCurve,
    counterparty_recovery_rate: float,
    own_recovery_rate: float,
    funding: FundingConfig | None = None,
) -> XvaResult:
    """
    Compute bilateral XVA: CVA, DVA, FVA, MVA, and the all-in adjustment.

    All legs are weighted by joint (first-to-default) survival, so the credit
    and funding components are not double-counted. MVA is computed only when
    ``funding`` carries an ``im_profile``; that posted IM also reduces ENE for
    bilateral DVA.

    The result reports ``total_xva = CVA - DVA + FVA + MVA``; uncomputed
    optional legs contribute zero.

    Parameters
    ----------
    exposure_profile : ExposureProfile
        EPE/ENE profile from exposure simulation.
    counterparty_hazard_curve : HazardCurve
        Hazard curve for the counterparty's credit.
    own_hazard_curve : HazardCurve
        Hazard curve for the institution's own credit.
    discount_curve : DiscountCurve
        Risk-free discount curve for present-valuing.
    counterparty_recovery_rate : float
        Recovery rate on counterparty default, in ``[0, 1]``.
    own_recovery_rate : float
        Recovery rate on own default, in ``[0, 1]``.
    funding : FundingConfig | None
        Funding configuration driving FVA and, when it carries an
        ``im_profile``, MVA. ``None`` computes the credit legs only.

    Returns
    -------
    XvaResult
        CVA, DVA, FVA, MVA, ``total_xva``, and the exposure/regulatory
        profiles.

    Raises
    ------
    ValueError
        If the exposure profile is empty or inconsistent, a recovery rate is
        outside ``[0, 1]``, funding inputs are invalid, the IM and exposure
        horizons differ, or a curve evaluation is non-finite.

    Examples
    --------
    >>> import datetime as dt
    >>> from finstack_quant.core.market_data.curves import DiscountCurve, HazardCurve
    >>> profile = ExposureProfile([1.0, 2.0], [1e6, 1e6], [1e6, 1e6], [0.0, 0.0])
    >>> df = DiscountCurve(
    ...     "USD-OIS",
    ...     dt.date(2025, 1, 1),
    ...     [(0.5 * i, 1.0) for i in range(9)],
    ...     interp="log_linear",
    ... )
    >>> hz = HazardCurve("CPTY", dt.date(2025, 1, 1), [(0.0, 0.02), (30.0, 0.02)])
    >>> result = compute_bilateral_xva(profile, hz, hz, df, 0.40, 0.40)
    >>> result.total_xva == result.cva - result.dva
    True
    """
    ...
