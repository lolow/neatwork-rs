"""Minimal stand-in for scikit-sparse 0.5 ``cho_factor`` used only to run the
NW5 python-core reference locally (SuiteSparse is not installed).
Dense LL^T via scipy.linalg; raises on non-positive-definite input like CHOLMOD."""
import numpy as np
import scipy.linalg as sla


class CholmodNotPositiveDefiniteError(Exception):
    pass


class CholeskyFactor:
    def __init__(self, A=None):
        self._c = None
        if A is not None:
            self.factorize(A)

    def factorize(self, A):
        dense = A.toarray() if hasattr(A, "toarray") else np.asarray(A)
        try:
            self._c = sla.cho_factor(dense, lower=True, check_finite=True)
        except sla.LinAlgError as exc:
            raise CholmodNotPositiveDefiniteError(str(exc)) from exc
        return self

    def solve(self, b):
        return sla.cho_solve(self._c, np.asarray(b, dtype=float))


def cho_factor(A, **kwargs):
    return CholeskyFactor(A)
