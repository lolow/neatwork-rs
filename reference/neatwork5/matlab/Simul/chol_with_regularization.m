function [L, delta_used, flag] = chol_with_regularization(M, options)
% Robust Cholesky with diagonal regularization delta*I if needed.
% Only used when chol fails. Keeps overhead negligible in normal cases.

% A MON AVIS IL FAUT EFFECTUER UNE FACTORISATION QUI TIENT COMPTE DU CREUX

% Defaults (can be added to options if you want)
delta0      = 0;
delta_min   = 1e-14;
delta_mult  = 10;
max_tries   = 8;

if isfield(options,'chol_delta0'),    delta0 = options.chol_delta0; end
if isfield(options,'chol_delta_min'), delta_min = options.chol_delta_min; end
if isfield(options,'chol_delta_mult'),delta_mult = options.chol_delta_mult; end
if isfield(options,'chol_max_tries'), max_tries = options.chol_max_tries; end

n = size(M,1);
I = speye(n);

% First attempt without (or with) initial delta
delta = delta0;
if delta > 0
    [L, flag] = chol(M + delta*I, 'lower');
else
    [L, flag] = chol(M, 'lower');
end

if flag == 0
    delta_used = delta;
    return;
end

% Regularize progressively
delta = max(delta_min, delta_mult * max(1, delta0));
for k = 1:max_tries
    [L, flag] = chol(M + delta*I, 'lower');
    if flag == 0
        delta_used = delta;
        return;
    end
    delta = delta * delta_mult;
end

% If we arrive here, everything failed: return last L/flag and delta
delta_used = delta / delta_mult;

end
