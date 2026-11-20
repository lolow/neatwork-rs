import pandas as pd
import numpy as np
from scipy import sparse
from sksparse.cholmod import cho_factor
from .exceptions import SolverNumericalError

# Cholesky regularization parameters
_DELTA_MIN = 1e-14
_DELTA_MULT = 10.0
_MAX_REG_TRIES = 8

def get_section_indices(lines:list[str], section_name:str):
	start, end = -1, -1
	for i, line in enumerate(lines):
		if line.startswith(f"!{section_name}"):
			start = i + 1
			continue 
		if start != -1:
			if line.startswith('!'):
				end = i
				break
			elif i == len(lines) - 1:
				end = i + 1
				break
	if start==-1 or end==-1:return None
	return start, end

def map_diam_cost(IDcol, lenCol, diameters, defaultDiam, defaultCost):
	# Remplit diam/cost à partir de TD(ID→Diameter/Cost), uniquement là où lenCol>0
	lenCol = pd.to_numeric(lenCol, errors='coerce').fillna(0)
	n = len(IDcol)
	diam = pd.Series([defaultDiam for _ in range(n)], dtype=float)
	cost = pd.Series([defaultCost for _ in range(n)], dtype=float)
	if 'diam' not in diameters.columns: return diam, cost
	if 'cost' not in diameters.columns: return diam, cost
	idx = lenCol > 0
	if idx.any():
		tf = IDcol.loc[idx].isin(diameters['ID'])
		diam.loc[idx] = defaultDiam
		cost.loc[idx] = defaultCost
		diam.loc[idx] = np.where(tf, IDcol.loc[idx].map(diameters.set_index('ID')['diam']), diam.loc[idx])
		cost.loc[idx] = np.where(tf, IDcol.loc[idx].map(diameters.set_index('ID')['cost']), cost.loc[idx])

	return diam, cost

def step_length(x, lo, up, s, t, dx, ds, dt):
	"""Return the largest feasible primal-dual step in ``[0, 1]``."""
	theta1 = 1
	idx = dx < 0
	if idx.any():
		theta1 = np.min(-(x[idx] - lo[idx]) / dx[idx])

	theta2 = 1
	idx = dx > 0
	if idx.any():
		theta2 = np.min((up[idx] - x[idx]) / dx[idx])

	theta3 = 1
	idx = ds < 0
	if idx.any():
		theta3 = np.min(s[idx] / (-ds[idx]))

	theta4 = 1
	idx = dt < 0
	if idx.any():
		theta4 = np.min(t[idx] / (-dt[idx]))

	return np.min([1.0, theta1, theta2, theta3, theta4])

def chol_with_regularization(matrix, factor_precalc=None, *, scenario_index: int | None = None):
	"""Factorize a sparse matrix using adaptive diagonal regularization.

    Args:
        matrix: Sparse symmetric matrix to factorize.
        factor_precalc: Optional CHOLMOD factor whose symbolic analysis can
            be reused for a matrix with the same sparsity structure.
        scenario_index: Scenario index attached to numerical errors.

    Returns:
        A CHOLMOD ``CholeskyFactor`` used to solve linear systems.

    Raises:
        SolverNumericalError: If factorization still fails after all regularization attempts.
	"""

	matrix = matrix.tocsc()
	matrix_size = matrix.shape[0]

	last_error: Exception | None = None
	
	if factor_precalc is None:
		try:
			return cho_factor(matrix)
		except Exception as exc:
			last_error = exc
	else:
		try:
			factor_precalc.factorize(matrix)
			return factor_precalc
		except Exception as exc:
			last_error = exc
	
	delta = _DELTA_MIN
	identity = sparse.eye(matrix_size, format='csc', dtype=matrix.dtype)
	
	for _ in range(_MAX_REG_TRIES):
		regularized_matrix = matrix + identity * delta
		try:
			if factor_precalc is None:
				return cho_factor(regularized_matrix)
			else:
				factor_precalc.factorize(regularized_matrix)
				return factor_precalc
		except Exception as exc:
			last_error = exc
			delta *= _DELTA_MULT
			
	raise SolverNumericalError(
        "Cholesky factorization failed after adaptive regularization.",
        scenario_index=scenario_index,
        details={
            "regularizationAttempts": _MAX_REG_TRIES,
            "lastDelta": delta / _DELTA_MULT,
        },
    ) from last_error