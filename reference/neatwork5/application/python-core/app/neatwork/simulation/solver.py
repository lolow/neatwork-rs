"""Primal-dual interior-point solver for hydraulic scenario problems."""

from dataclasses import dataclass

import numpy as np

from .exceptions import SolverNumericalError, SimulationCancelledError
from .models import (
    ScenarioProblem,
    SolverDiagnostics,
    SolverOptions,
    SolverResult,
    SolverStatus,
)

from .numerical_utils import chol_with_regularization, step_length
from collections.abc import Callable


# Interior Point Method (IPM) solver configuration
_MIN_INVD = 1e-16  # Minimum threshold for inverse diagonal scaling

@dataclass(slots=True)
class _SolverState:
    """Mutable primal-dual state used internally by the IPM solver.

    Attributes:
        x: Current primal flow vector.
        u: Dual variables for flow-balance constraints.
        s: Multipliers associated with lower bounds.
        t: Multipliers associated with upper bounds.
        lo: Internal lower flow bounds.
        up: Internal upper flow bounds.
        mu: Current barrier parameter.
    """
    x: np.ndarray
    u: np.ndarray
    s: np.ndarray
    t: np.ndarray

    lo: np.ndarray
    up: np.ndarray

    mu: float

class InteriorPointSolver:
    """Solve one hydraulic scenario using a primal-dual interior-point method.

    The solver is independent from ``SimulationData`` and only receives a
    typed :class:`ScenarioProblem` plus numerical :class:`SolverOptions`.

    Attributes:
        options: Numerical tolerances and iteration controls.
    """

    def __init__(self,options: SolverOptions) -> None:
        """Initialize the solver with validated numerical options."""

        self.options = options

    def _build_initial_state(self,problem: ScenarioProblem) -> _SolverState:
        """Build a strictly interior initial primal-dual state.

        Args:
            problem: Solver-ready hydraulic scenario.

        Returns:
            Initialized state for the IPM iterations.

        Raises:
            SolverNumericalError: If a finite strictly interior point cannot
                be constructed.
        """

        A = problem.incidence_matrix.tocsr()
        b_vec = problem.balance_rhs

        m, n = A.shape

        x = np.zeros(n, dtype=float)

        u = np.zeros(m, dtype=float)

        transit_mask = np.asarray(problem.transit_pipe_mask_open, dtype=bool)

        source_mask = np.asarray(problem.source_pipe_mask_open,dtype=bool)

        terminal_mask = np.asarray(problem.terminal_pipe_mask_open,dtype=bool)

        open_mask = problem.open_pipe_mask

        lo = problem.lower_bounds_by_pipe[open_mask].astype(float,copy=True)

        up = problem.upper_bounds_by_pipe[open_mask].astype(float,copy=True)

        # Preserve the historical unbounded-flow initialization.
        if not problem.bounded_flow:
            x[terminal_mask] = up[terminal_mask]
            up *= 5.0
            lo *= 5.0

        else:
            x[terminal_mask] = (up[terminal_mask] + lo[terminal_mask]) / 2.0

        source_indices = np.flatnonzero(source_mask)

        if len(source_indices) == 0:
            raise SolverNumericalError(
                ("The solver cannot initialize without an open source pipe."),
                scenario_index=problem.scenario_index,
            )

        initial_residual = A @ x - b_vec

        x_sum = np.zeros_like(x)

        for source_index in source_indices:
            selected_source = np.zeros(n,dtype=bool)

            selected_source[source_index] = True

            balance_pipe_mask = transit_mask | selected_source

            B = A.tocsc()[:,balance_pipe_mask]

            M = B @ B.T

            factor = chol_with_regularization(
                    M.tocsc(),
                    scenario_index=problem.scenario_index
                )
            

            du = factor.solve(initial_residual)

            dx = -B.T @ du

            candidate = x.copy()

            candidate[balance_pipe_mask] += dx

            x_sum += candidate

        x = x_sum / len(source_indices)

        lower_gap = x - lo
        upper_gap = up - x

        if (
            not np.isfinite(x).all()
            or not np.isfinite(lower_gap).all()
            or not np.isfinite(upper_gap).all()
            or np.any(lower_gap <= 0)
            or np.any(upper_gap <= 0)
        ):
            raise SolverNumericalError(
                "Initial solver point is not strictly inside the flow bounds.",
                scenario_index=problem.scenario_index,
            )

        s = 1.0 / lower_gap
        t = 1.0 / upper_gap

        mu = float((s @ lower_gap + t @ upper_gap) / (2 * n))

        if not np.isfinite(mu) or mu <= 0:
            raise SolverNumericalError(
                "Initial barrier parameter is invalid.",
                scenario_index=problem.scenario_index,
                details={"mu": mu},
            )

        return _SolverState(
            x=x,
            u=u,
            s=s,
            t=t,
            lo=lo,
            up=up,
            mu=mu,
        )

    def _friction_derivatives(self, x: np.ndarray, problem: ScenarioProblem, absx_lbd: np.ndarray, fvec: np.ndarray, gradf: np.ndarray, F: np.ndarray, G: np.ndarray) -> tuple[
        float,
        np.ndarray,
        np.ndarray,
        np.ndarray,
    ]:
        """Compute objective and regularized friction derivatives.

        Args:
            x: Current flow vector in open-pipe ordering.
            problem: Hydraulic scenario containing alpha, beta and gravity.
            absx_lbd: Reusable workspace for powers of absolute flow.
            fvec: Reusable workspace for objective contributions.
            gradf: Reusable gradient workspace.
            F: Reusable second-derivative workspace.
            G: Reusable third-order correction workspace.

        Returns:
            Objective value, gradient, ``F`` and ``G`` arrays.
        """

        open_mask = problem.open_pipe_mask

        alpha = problem.alpha_by_pipe[open_mask]

        beta = problem.beta_by_pipe[open_mask]

        gravity = problem.gravity_by_pipe[open_mask]

        lbd = problem.friction_exponent

        tolx = self.options.flow_tolerance

        tolx_lbd_plus_1 = tolx ** (lbd + 1)

        tolx_lbd_minus_1 = tolx ** (lbd - 1)

        tolx_inv_sq = tolx ** (-2)

        absx = np.abs(x)

        regular_flow_mask = absx > tolx

        small_flow_mask = ~regular_flow_mask

        absx_lbd.fill(0.0)

        absx_lbd[regular_flow_mask] = absx[regular_flow_mask] ** (lbd - 1)

        fvec.fill(0.0)

        fvec[regular_flow_mask] = (
            (
                beta[regular_flow_mask] * absx_lbd[regular_flow_mask]
                / (lbd + 1)
                + alpha[regular_flow_mask] * absx[regular_flow_mask]
                / 3
            )
            * x[regular_flow_mask] ** 2
        )

        fvec[small_flow_mask] = (tolx_lbd_plus_1 / (lbd + 1) * beta[small_flow_mask])

        fvec[small_flow_mask] += (0.5 * beta[small_flow_mask]
            * (x[small_flow_mask] ** 2 * tolx_inv_sq - tolx_lbd_plus_1)
        )

        objective = float(np.sum(fvec) + gravity @ x)

        gradf.fill(0.0)

        gradf[regular_flow_mask] = (
            beta[regular_flow_mask] * absx_lbd[regular_flow_mask] * x[regular_flow_mask]
            + alpha[regular_flow_mask] * absx[regular_flow_mask] * x[regular_flow_mask]
            + gravity[regular_flow_mask]
        )

        gradf[small_flow_mask] = (
            tolx_lbd_minus_1 * beta[small_flow_mask] * x[small_flow_mask]
            + tolx * alpha[small_flow_mask] * x[small_flow_mask]
            + gravity[small_flow_mask]
        )

        F.fill(0.0)

        F[regular_flow_mask] = (
            lbd * beta[regular_flow_mask] * absx_lbd[regular_flow_mask]
            + 2 * alpha[regular_flow_mask] * absx[regular_flow_mask]
        )

        F[small_flow_mask] = (
            tolx_lbd_minus_1 * beta[small_flow_mask]
            + tolx * alpha[small_flow_mask]
        )

        G.fill(0.0)

        G[regular_flow_mask] = (
            lbd * (lbd - 1) * beta[regular_flow_mask] * absx_lbd[regular_flow_mask] / x[regular_flow_mask]
            + 2 * alpha[regular_flow_mask] * np.sign(x[regular_flow_mask])
        )

        return (objective, gradf, F, G)

    def _has_converged(self, *, x: np.ndarray, lo: np.ndarray, up: np.ndarray, charge: np.ndarray, primal_residual: np.ndarray) -> bool:
        """Apply the historical active-set convergence criteria.

        Args:
            x: Current flow vector.
            lo: Internal lower bounds.
            up: Internal upper bounds.
            charge: Current hydraulic stationarity vector.
            primal_residual: Current flow-balance residual.

        Returns:
            ``True`` when all legacy convergence conditions are satisfied.
        """

        tolx = self.options.flow_tolerance

        tolpress = self.options.pressure_tolerance

        at_lower = (x - lo) < tolx

        at_upper = (up - x) < tolx

        interior = ~(at_lower | at_upper)

        primal_bad = (np.linalg.norm(primal_residual, np.inf) > tolx)

        if np.any(interior):
            interior_bad = (np.linalg.norm(charge[interior], np.inf) > tolpress)
        else:
            interior_bad = False

        if np.any(at_upper):
            upper_bad = (np.max(charge[at_upper]) > tolpress)
        else:
            upper_bad = False

        if np.any(at_lower):
            lower_bad = (np.min(charge[at_lower]) < -tolpress)
        else:
            lower_bad = False

        return not (primal_bad or interior_bad or upper_bad or lower_bad)

    def solve(self, problem: ScenarioProblem, *, cancel_requested: Callable[[], bool] | None = None) -> SolverResult:
        """Solve one hydraulic scenario with primal-dual IPM iterations.

        Args:
            problem: Validated solver-ready hydraulic problem.
            cancel_requested: Optional callback that returns True when the
                simulation should be cancelled.

        Returns:
            Typed flows, dual variables, diagnostics and convergence status.

        Raises:
            SolverNumericalError: If factorization or an iteration becomes
                numerically invalid.
        """

        if cancel_requested is not None and cancel_requested():
            raise SimulationCancelledError()

        A = problem.incidence_matrix.tocsr()
        A_T = A.T.tocsr()

        b_vec = problem.balance_rhs
        _, n = A.shape

        state = self._build_initial_state(problem)

        x = state.x
        u = state.u
        s = state.s
        t = state.t

        lo = state.lo
        up = state.up

        mu = state.mu

        absx_lbd = np.zeros(n)
        fvec = np.zeros(n)
        gradf = np.zeros(n)
        F = np.zeros(n)
        G = np.zeros(n)

        (objective, gradf, F, G) = self._friction_derivatives(
            x, problem, absx_lbd, fvec, gradf, F, G
        )

        invD = np.ones(n)
        A_scaled = A.copy()

        A_scaled.data *= invD[A_scaled.indices]

        M = A_scaled @ A_T

        factor = chol_with_regularization(M.tocsc(), scenario_index=problem.scenario_index)

        converged = False
        iteration = 0

        while iteration < self.options.max_iterations:
            if cancel_requested is not None and cancel_requested():
                raise SimulationCancelledError()

            iteration += 1

            charge = gradf + A_T @ u

            primal_residual = A @ x - b_vec

            dual_residual = gradf + A_T @ u - s + t

            if self._has_converged(x=x, lo=lo, up=up, charge=charge, primal_residual=primal_residual):
                converged = True

                (objective, gradf, F, G) = self._friction_derivatives(x, problem, absx_lbd, fvec, gradf, F, G)

                break

            # Predictor step.
            rho = -charge

            rhs_pred = primal_residual + A @ (invD * rho)

            du_pred = factor.solve(rhs_pred)

            dx_pred = -invD * (A_T @ du_pred - rho)

            ds_pred = -(s * dx_pred) / (x - lo) - s

            dt_pred = (t * dx_pred) / (up - x) - t

            theta_aff = step_length(x, lo, up, s, t, dx_pred, ds_pred, dt_pred)

            mu_aff = float(
                (
                    np.dot(s + theta_aff * ds_pred, x + theta_aff * dx_pred - lo) 
                    	+ np.dot(t + theta_aff * dt_pred, up - x - theta_aff * dx_pred)
                ) / (2 * n)
            )

            sigma_mu = min(1.0, (mu_aff / mu) ** self.options.centering_exponent) * mu

            # Corrector step.
            rho_cs = s + (dx_pred * ds_pred - sigma_mu) / (x - lo)

            rho_ct = t - (dx_pred * dt_pred + sigma_mu) / (up - x)

            rho_corr = -(dual_residual + 0.5 * dx_pred * G * dx_pred + rho_cs - rho_ct)

            rhs_corr = primal_residual + A @ (invD * rho_corr)

            du_corr = factor.solve(rhs_corr)

            dx_corr = -invD * (A_T @ du_corr - rho_corr)

            ds_corr = -(s * dx_corr) / (x - lo) - rho_cs

            dt_corr = (t * dx_corr) / (up - x) - rho_ct

            theta = self.options.step_safety * step_length(x, lo, up, s, t, dx_corr, ds_corr, dt_corr)

            if not np.isfinite(theta) or theta <= 0:
                raise SolverNumericalError(
                    "Interior-point step length is invalid.",
                    scenario_index=problem.scenario_index,
                    details={"iteration": iteration, "theta": float(theta)},
                )

            x = x + theta * dx_corr

            u = u + theta * du_corr

            s = s + theta * ds_corr

            t = t + theta * dt_corr

            mu = float(self.options.barrier_reduction * (np.dot(s, x - lo) + np.dot(t, up - x)) / (2 * n))

            if not (np.isfinite(x).all() and np.isfinite(u).all() and np.isfinite(s).all() and np.isfinite(t).all() and np.isfinite(mu)):
                raise SolverNumericalError(
                    "Interior-point iteration produced non-finite values.",
                    scenario_index=problem.scenario_index,
                    details={"iteration": iteration},
                )

            objective, gradf, F, G = self._friction_derivatives(x,problem,absx_lbd,fvec,gradf,F,G)

            D = F + s / (x - lo) + t / (up - x)

            invD = np.maximum(1.0 / D, _MIN_INVD)

            if not np.isfinite(invD).all():
                raise SolverNumericalError(
                    "Inverse diagonal scaling contains invalid values.",
                    scenario_index=problem.scenario_index,
                    details={"iteration": iteration},
                )

            A_scaled.data = A.data * invD[A_scaled.indices]

            M = A_scaled @ A_T

            factor = chol_with_regularization(M.tocsc(), factor, scenario_index=problem.scenario_index)

        charge = gradf + A_T @ u

        primal_residual = A @ x - b_vec

        dual_residual = gradf + A_T @ u - s + t
        
        if converged:
            status = SolverStatus.CONVERGED
        else:
            status = SolverStatus.MAX_ITERATIONS

        diagnostics = SolverDiagnostics(
            primal_residual_inf=float(np.linalg.norm(primal_residual, np.inf)),
            dual_residual_inf=float(np.linalg.norm(dual_residual, np.inf)),
            barrier_parameter=float(mu),
        )

        return SolverResult(
            scenario_index=problem.scenario_index,
            flow_by_open_pipe_m3s=x.copy(),
            node_dual=u.copy(),
            charge_by_open_pipe_m=charge.copy(),
            lower_multiplier=s.copy(),
            upper_multiplier=t.copy(),
            iterations=iteration,
            converged=converged,
            status=status,
            objective_value=float(objective),
            diagnostics=diagnostics,
        )