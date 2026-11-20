from data import *
from utils import *
import scipy.sparse.linalg
from sksparse.cholmod import cholesky
import numpy as np
import pandas as pd
import scipy.sparse as sparse

# Interior Point Method (IPM) solver configuration
_MIN_INVD = 1e-16  # Minimum threshold for inverse diagonal scaling

class Solver:
	"""
	Flow solver using a primal-dual interior-point method.
	Methods
	-------
	init_point_debit(work)
		initialize a strictly interior point for the primal-dual iterations.
	friction_derivatives(x, work, absx_lbd, fvec, gradf, F, G)
		compute f, its gradient, and regularized second-order derivatives.
	debits(work)
		solve the bounded-flow problem with primal-dual iterations.
	"""

	def __init__(self, inst):
		self.inst = inst

	def init_point_debit(self, work):
		"""Build a strictly interior initial point consistent with A x = 0."""
		A = work['A']
		A_csr = A.tocsr()
		A_csc = A.tocsc()
		b_vec = work.get('b', np.zeros(A.shape[0]))
		m, n = A.shape
		x = np.zeros(n)
		u = np.zeros(m)
		s = np.ones(n)
		t = np.ones(n)
		ArcB = np.array(work['ArcB'], dtype=bool) # intermediary edges 
		ArcS = np.array(work['ArcS'], dtype=bool) # source edges
		ArcT = np.array(work['ArcT'], dtype=bool) # terminal arcs

		# bounds 
		lo = work['xinf'].copy()
		up = work['xsup'].copy()
		
		# --- initial proposal on terminal arcs ---
		if self.inst.param['bounded_flow'] == 0:
			x[ArcT] = up[ArcT]
			up *= 5
			lo *= 5
		else:
			x[ArcT] = (up[ArcT] + lo[ArcT]) / 2

		idx_sources = np.flatnonzero(ArcS)
		nS = len(idx_sources)
		r_init = A @ x - b_vec
		x_sum = np.zeros_like(x)

		for i in range(nS):
			ArcS_sel = np.zeros(len(x), dtype=bool)
			ArcS_sel[idx_sources[i]] = True
			ArcBS = ArcB | ArcS_sel
		
			B = A.tocsc()[:, ArcBS]
			M = B @ B.T
		
			factor = cholesky(M.tocsc())
			du = factor(r_init)
			dx = -B.T @ du
		
			x_temp = x.copy()
			x_temp[ArcBS] = x_temp[ArcBS] + dx
			x_sum += x_temp
		x = x_sum / nS

		# --- initialize s,t,mu consistently with current x
		s = 1 / (x - lo)
		t = 1 / (up - x)
		mu = (s @ (x - lo) + t @ (up - x)) / (2 * n)
		
		work['x'] = x
		work['u'] = u
		work['s'] = s
		work['t'] = t
		work['lo'] = lo
		work['up'] = up
		work['mu'] = mu
		return work

	def friction_derivatives(self, x, work, absx_lbd, fvec, gradf, F, G):
		"""Compute f, its gradient, and regularized second-order derivatives."""
		alpha = work['alpha']
		beta = work['beta']
		gravite = work['gravit']
		
		lbd = self.inst.param['lambda']
		tolx = self.inst.options['tolx']

		tolx_lbd_plus_1 = tolx**(lbd + 1)
		tolx_lbd_minus_1 = tolx**(lbd - 1)
		tolx_inv_sq = tolx**(-2)

		absx = np.abs(x)
		vec1 = (absx > tolx)
		vec2 = ~vec1

		absx_lbd.fill(0.0)
		absx_lbd[vec1] = absx[vec1]**(lbd - 1)

		fvec.fill(0.0)
		fvec[vec1] = (beta[vec1] * absx_lbd[vec1] / (lbd + 1) + alpha[vec1] * absx[vec1] / 3) * x[vec1]**2
		fvec[vec2] = tolx_lbd_plus_1 / (lbd + 1) * beta[vec2]
		fvec[vec2] = fvec[vec2] + 0.5 * beta[vec2] * (x[vec2]**2 * tolx_inv_sq - tolx_lbd_plus_1)

		f = np.sum(fvec) + gravite @ x

		gradf.fill(0.0)
		gradf[vec1] = beta[vec1] * (absx_lbd[vec1] * x[vec1]) + alpha[vec1] * absx[vec1] * x[vec1] + gravite[vec1]
		gradf[vec2] = tolx_lbd_minus_1 * beta[vec2] * x[vec2] + tolx * alpha[vec2] * x[vec2] + gravite[vec2]

		F.fill(0.0)
		F[vec1] = lbd * beta[vec1] * absx_lbd[vec1] + 2 * alpha[vec1] * absx[vec1]
		F[vec2] = tolx_lbd_minus_1 * beta[vec2] + tolx * alpha[vec2]

		G.fill(0.0)
		G[vec1] = lbd * (lbd - 1) * beta[vec1] * absx_lbd[vec1] / x[vec1] + 2 * alpha[vec1] * np.sign(x[vec1])

		return f, gradf, F, G

	def debits(self, work):
		"""
		Solve the bounded-flow problem with primal-dual interior-point iterations.
		
		Uses a predictor-corrector variant of the IPM algorithm. Each iteration:
		1. Computes affine step and step length (predictor)
		2. Adjusts centering based on affine progress (corrector)
		3. Updates primal/dual variables with damping factor a_secur
		4. Checks convergence on primal/dual/centrality residuals
		
		Parameters
		----------
		work : dict
			Dictionary with keys: 'A' (incidence), 'alpha', 'beta', 'xsup', 'xinf',
			'x', 's', 't', 'u' (ipm state), 'lo', 'up' (bounds), 'mu' (centering param)
		
		Returns
		-------
		work : dict
			Updated dictionary with final solution 'x', multipliers 'u', 's', 't',
			iteration count 'iter', and dual variables 'charge'.
		"""
		A = work['A']
		A_csr = A.tocsr()
		A_T_csr = A.T.tocsr()
		b_vec = work.get('b', np.zeros(A.shape[0]))
		m, n = A.shape
		options = self.inst.options
		max_iter, tau, a_secur = options['maxiter'], options['tau'], options['a_secur']
		tolx, tolpress, tolcent = options['tolx'], options['tolpress'], options['tolcent']

		# === 1) Initial point ===
		work = self.init_point_debit(work)
		x, u, s, t, lo, up, mu = work['x'], work['u'], work['s'], work['t'], work['lo'], work['up'], work['mu']

		# === 2) Friction derivatives at current x ===
		absx_lbd, fvec, gradf, F, G = np.zeros(n), np.zeros(n), np.zeros(n), np.zeros(n), np.zeros(n)
		_, gradf, F, G = self.friction_derivatives(x, work, absx_lbd, fvec, gradf, F, G)

		# === 3) Build Dinv and factorize M = A*Dinv*A' ===
		invD = np.ones(n)
		A_scaled = A_csr.copy()
		A_scaled.data *= invD[A_scaled.indices]
		M_csr_init = A_scaled @ A_T_csr
		M_csc_init = sparse.csc_matrix((M_csr_init.data, M_csr_init.indices, M_csr_init.indptr), shape=M_csr_init.shape)
		factor = chol_with_regularization(M_csc_init)

		k = 0
		stop = 0
		while k < max_iter and stop == 0:
			k += 1
			# --- Residuals for the barrier problem ---
			charge = gradf + A_T_csr @ u
			r_primal = A_csr @ x - b_vec
			r_dual = gradf + A_T_csr @ u - s + t
			r_cent_s = s * (x - lo)
			r_cent_t = t * (up - x)

			# --- Active-set indicators for the original problem
			idlo = (x - lo) < tolx
			idup = (up - x) < tolx
			idmed = ~(idlo | idup)

			# --- Convergence tests for the original problem
			np_ = np.linalg.norm(r_primal, np.inf) > tolx
			if not np.any(idmed):
				nd = False
			else:
				nd = np.linalg.norm(charge[idmed], np.inf) > tolpress

			if not np.any(idup):
				ndup = False
			else:
				ndup = np.max(charge[idup]) > tolpress

			if not np.any(idlo):
				ndlo = False
			else:
				ndlo = np.min(charge[idlo]) < -tolpress

				# –--- Test convergence and compute new iterate
			if not (np_ or nd or ndlo or ndup):
				stop = True
				# Recompute derivatives at final x for consistency
				_, gradf, F, G = self.friction_derivatives(x, work, absx_lbd, fvec, gradf, F, G)
			else:
				stop = False
				# === Predictor step (mu = 0) ===
				rho = -charge
				rhs_pred = r_primal + A_csr @ (invD * rho)

				# solve M * du = rhs with existing Cholesky (or regularized)
				du_pred = factor(rhs_pred)
				dx_pred = -invD * (A_T_csr @ du_pred - rho)
				ds_pred = -(s * dx_pred) / (x - lo) - s
				dt_pred = (t * dx_pred) / (up - x) - t

				theta_aff = step_length(x, lo, up, s, t, dx_pred, ds_pred, dt_pred)
				mu_aff = (np.dot(s + theta_aff * ds_pred, x + theta_aff * dx_pred - lo) + 
						  np.dot(t + theta_aff * dt_pred, up - x - theta_aff * dx_pred)) / (2 * n)
				
				# === Corrector step ===
				sigma_mu = min(1, (mu_aff / mu)**tau)*mu

				rho_cs = s + (dx_pred * ds_pred - sigma_mu) / (x - lo)
				rho_ct = t - (dx_pred * dt_pred + sigma_mu) / (up - x)
				rho_corr = -(r_dual + 0.5 * dx_pred * G * dx_pred + rho_cs - rho_ct)
				
				# ------------------- Solve corrected system -------------------
				rhs_corr = r_primal + A_csr @ (invD * rho_corr)
				du_corr = factor(rhs_corr)
				dx_corr = -invD * (A_T_csr @ du_corr - rho_corr)
				ds_corr = -(s * dx_corr) / (x - lo) - rho_cs
				dt_corr = (t * dx_corr) / (up - x) - rho_ct

				theta = a_secur * step_length(x, lo, up, s, t, dx_corr, ds_corr, dt_corr)

				# --- Update variables (unchanged)
				x = x + theta * dx_corr
				u = u + theta * du_corr
				s = s + theta * ds_corr
				t = t + theta * dt_corr
				# --- Update mu
				mu = options['sigma'] * (np.dot(s, x - lo) + np.dot(t, up - x)) / (2 * n)
				# --- Refresh derivatives and matrices for next iter
				_, gradf, F, G = self.friction_derivatives(x, work, absx_lbd, fvec, gradf, F, G)
				# ------------------- Compute scaled system matrix -------------------
				D = F + s / (x - lo) + t / (up - x)
				invD = np.maximum(1.0 / D, _MIN_INVD)

				A_scaled.data = A_csr.data * invD[A_scaled.indices]
				M_csr = A_scaled @ A_T_csr
				M_csc = sparse.csc_matrix((M_csr.data, M_csr.indices, M_csr.indptr), shape=M_csr.shape)
				factor = chol_with_regularization(M_csc, factor)

		work.update({'x': x, 's': s, 't': t, 'u': u, 'iter': k, 'charge': gradf + A_T_csr @ u})
		return work