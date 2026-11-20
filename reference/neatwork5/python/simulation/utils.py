from fractions import Fraction
import pandas as pd
import numpy as np
from scipy import sparse
from sksparse.cholmod import cholesky

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

def chol_with_regularization(M, factor_precalc=None):
	"""Performs Cholesky factorization with adaptive regularization if needed."""
	n = M.shape[0]
	
	if factor_precalc is None:
		try:
			return cholesky(M)
		except:
			pass
	else:
		try:
			factor_precalc.cholesky_inplace(M)
			return factor_precalc
		except:
			pass
	
	delta = _DELTA_MIN
	I = sparse.eye(n, format='csc')
	
	for _ in range(_MAX_REG_TRIES):
		M_reg = M + I * delta
		try:
			if factor_precalc is None:
				return cholesky(M_reg)
			else:
				factor_precalc.cholesky_inplace(M_reg)
				return factor_precalc
		except:
			delta *= _DELTA_MULT
			
	raise RuntimeError("Échec de la factorisation de Cholesky après régularisation maximale.")