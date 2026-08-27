"""Tests for Monte Carlo pricing through the pricer and engine surfaces."""

from collections.abc import Callable
import math

import pytest

from finstack_quant.models.monte_carlo import (
    EuropeanPricer,
    GbmPathSummary,
    McEngine,
    TimeGrid,
    finite_diff_delta,
    finite_diff_delta_crn,
    finite_diff_gamma,
    finite_diff_gamma_crn,
    heston_satisfies_feller,
    simulate_gbm_paths,
)

GreekFunction = Callable[..., tuple[float, float]]


class TestEuropeanPricer:
    """EuropeanPricer produces reasonable option prices under GBM."""

    @pytest.fixture
    def pricer(self) -> EuropeanPricer:
        """Deterministic pricer with enough paths for rough convergence."""
        return EuropeanPricer(num_paths=50_000, seed=42)

    def test_call_price_positive(self, pricer: EuropeanPricer) -> None:
        """ATM call price should be positive."""
        result = pricer.price_call(
            spot=100.0,
            strike=100.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.20,
            expiry=1.0,
        )
        assert result.mean.amount > 0.0

    def test_put_price_positive(self, pricer: EuropeanPricer) -> None:
        """ATM put price should be positive."""
        result = pricer.price_put(
            spot=100.0,
            strike=100.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.20,
            expiry=1.0,
        )
        assert result.mean.amount > 0.0

    def test_call_itm_more_expensive(self, pricer: EuropeanPricer) -> None:
        """A lower strike (deeper ITM) call should be more expensive."""
        atm = pricer.price_call(
            spot=100.0,
            strike=100.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.20,
            expiry=1.0,
        )
        itm = pricer.price_call(
            spot=100.0,
            strike=80.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.20,
            expiry=1.0,
        )
        assert itm.mean.amount > atm.mean.amount

    def test_put_call_parity_approx(self, pricer: EuropeanPricer) -> None:
        """Put-call parity: C - P ≈ e^(-rT) * (S*e^((r-q)T) - K)."""
        spot, strike, r, q, vol, expiry = 100.0, 100.0, 0.05, 0.0, 0.20, 1.0
        call = pricer.price_call(
            spot=spot,
            strike=strike,
            rate=r,
            div_yield=q,
            vol=vol,
            expiry=expiry,
        )
        put = pricer.price_put(
            spot=spot,
            strike=strike,
            rate=r,
            div_yield=q,
            vol=vol,
            expiry=expiry,
        )
        lhs = call.mean.amount - put.mean.amount
        forward = spot * math.exp((r - q) * expiry)
        rhs = math.exp(-r * expiry) * (forward - strike)
        assert lhs == pytest.approx(rhs, abs=2.0)

    def test_result_attributes(self, pricer: EuropeanPricer) -> None:
        """MoneyEstimate exposes stderr, ci_lower, ci_upper, num_paths."""
        result = pricer.price_call(
            spot=100.0,
            strike=100.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.20,
            expiry=1.0,
        )
        assert result.stderr > 0.0
        assert result.ci_lower.amount < result.mean.amount
        assert result.ci_upper.amount > result.mean.amount
        assert result.num_paths == 50_000

    def test_seed_reproducibility(self) -> None:
        """Same seed produces identical results."""
        p1 = EuropeanPricer(num_paths=10_000, seed=123)
        p2 = EuropeanPricer(num_paths=10_000, seed=123)
        r1 = p1.price_call(spot=100.0, strike=100.0, rate=0.05, div_yield=0.0, vol=0.20, expiry=1.0)
        r2 = p2.price_call(spot=100.0, strike=100.0, rate=0.05, div_yield=0.0, vol=0.20, expiry=1.0)
        assert r1.mean.amount == pytest.approx(r2.mean.amount, abs=1e-10)


@pytest.mark.parametrize("is_call", [True, False], ids=["call", "put"])
def test_seeded_engine_and_european_pricer_results_are_identical(is_call: bool) -> None:
    """The retained Python entry points must remain deterministic equivalents."""
    num_paths = 2_048
    num_steps = 12
    seed = 123
    engine = McEngine(
        num_paths=num_paths,
        time_grid=TimeGrid(t_max=1.0, num_steps=num_steps),
        seed=seed,
        use_parallel=False,
        antithetic=False,
    )
    pricer = EuropeanPricer(num_paths=num_paths, seed=seed, use_parallel=False)

    if is_call:
        engine_result = engine.price_european_call(100.0, 100.0, 0.05, 0.0, 0.2)
        pricer_result = pricer.price_call(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, num_steps=num_steps)
    else:
        engine_result = engine.price_european_put(100.0, 100.0, 0.05, 0.0, 0.2)
        pricer_result = pricer.price_put(100.0, 100.0, 0.05, 0.0, 0.2, 1.0, num_steps=num_steps)

    assert engine_result.mean.amount == pricer_result.mean.amount
    assert engine_result.mean.currency.code == pricer_result.mean.currency.code
    assert engine_result.stderr == pricer_result.stderr
    assert engine_result.std_dev == pricer_result.std_dev
    assert engine_result.ci_lower.amount == pricer_result.ci_lower.amount
    assert engine_result.ci_upper.amount == pricer_result.ci_upper.amount
    assert engine_result.num_paths == pricer_result.num_paths
    assert engine_result.num_simulated_paths == pricer_result.num_simulated_paths


def test_mc_engine_antithetic_preserves_estimator_and_simulation_counts() -> None:
    engine = McEngine(
        num_paths=128,
        time_grid=TimeGrid(t_max=1.0, num_steps=8),
        seed=42,
        use_parallel=False,
        antithetic=True,
    )
    result = engine.price_european_call(100.0, 100.0, 0.05, 0.0, 0.2)
    assert result.num_paths == 128
    assert result.num_simulated_paths == 256


@pytest.mark.parametrize(
    ("t_max", "num_steps"),
    [
        (0.0, 8),
        (1.0, 0),
    ],
)
def test_time_grid_invalid_inputs_raise_value_error(t_max: float, num_steps: int) -> None:
    with pytest.raises(ValueError, match="Invalid input data"):
        TimeGrid(t_max=t_max, num_steps=num_steps)


def test_simulate_gbm_paths_is_typed_deterministic_and_shaped() -> None:
    first = simulate_gbm_paths(
        spot=100.0,
        rate=0.05,
        div_yield=0.01,
        vol=0.2,
        expiry=1.0,
        num_steps=4,
        num_paths=3,
        seed=42,
    )
    second = simulate_gbm_paths(100.0, 0.05, 0.01, 0.2, 1.0, 4, 3, 42)
    assert isinstance(first, GbmPathSummary)
    assert first.times == second.times
    assert first.paths == second.paths
    assert first.num_paths == 3
    assert first.num_simulated_paths == 3
    assert len(first.times) == 5
    assert len(first.paths) == 3
    assert all(len(path) == 5 for path in first.paths)
    assert all(path[0] == pytest.approx(100.0) for path in first.paths)


def test_simulate_gbm_paths_rejects_capture_with_antithetic() -> None:
    with pytest.raises(ValueError, match="antithetic"):
        simulate_gbm_paths(100.0, 0.05, 0.0, 0.2, 1.0, 4, 3, 42, antithetic=True)


def test_heston_feller_uses_inclusive_predicate_without_validation() -> None:
    assert heston_satisfies_feller(2.0, 0.04, 0.3)
    assert heston_satisfies_feller(1.0, 0.045, 0.3)
    assert not heston_satisfies_feller(1.0, 0.04, 0.5)
    assert not heston_satisfies_feller(0.0, 0.04, 0.3)


@pytest.mark.parametrize(
    "greek",
    [
        finite_diff_delta,
        finite_diff_delta_crn,
        finite_diff_gamma,
        finite_diff_gamma_crn,
    ],
)
@pytest.mark.parametrize(
    ("spot", "bump_size", "message"),
    [
        (0.0, 0.01, "initial_spot"),
        (float("nan"), 0.01, "initial_spot"),
        (100.0, 0.0, "bump_size"),
        (100.0, float("inf"), "bump_size"),
        (1e-8, 2.0, "central stencil"),
    ],
)
def test_finite_difference_greeks_map_invalid_stencils_to_value_error(
    greek: GreekFunction,
    spot: float,
    bump_size: float,
    message: str,
) -> None:
    with pytest.raises(ValueError, match=message):
        greek(
            spot=spot,
            strike=100.0,
            rate=0.05,
            div_yield=0.0,
            vol=0.2,
            expiry=1.0,
            num_paths=8,
            num_steps=1,
            bump_size=bump_size,
            option_type="call",
        )
