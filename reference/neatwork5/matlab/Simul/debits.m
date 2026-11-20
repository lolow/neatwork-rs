function work = debits(work,dat,param,options) %#ok<INUSD>
% Core IPM for flow computation (box constraints + equality Ax=0)

A = work.A;
[m,n] = size(A); %#ok<ASGLU>

max_iter = options.maxiter;

% --- IPM constants (unchanged semantics)
sigma   = options.sigma;        % reduction factor for mu (kept as in your code)
tau     = options.tau;        % exponent used in sigma_mu = min(1,(mu_aff/mu)^tau)*mu
a_secur = options.a_secur;      % safeguard on final step
tolx    = options.tolx;
tolpress = options.tolpress;
tolcent = options.tolcent;    


% === 1) Initial point (your logic moved out) ===
[x,u,s,t,lo,up,mu] = init_point_debits(work,param);

work.x  = x;
work.u  = u;
work.s  = s;
work.t  = t;
work.lo = lo;
work.up = up;

% === 2) Friction derivatives at current x ===
[~, gradf, F, G] = friction_derivatives( ...
    x, work.alpha, work.beta, work.gravit, param,options);

% === 3) Build Dinv and factorize M = A*Dinv*A' (as before)
D    = (F + s ./ (x - lo) + t ./ (up - x));
invD = 1 ./ D;
Dinv = spdiags(invD, 0, n, n);

M = A * Dinv * A';
[L, delta_used, flag] = chol_with_regularization(M, param); %#ok<ASGLU>

k = 0;
stop = 0;

while k < max_iter && stop == 0
    k = k + 1;

    % --- Residuals for the barrier problem
    charge   = gradf + A' * u;
    r_primal = A * x;
    r_dual   = charge - s + t;
    r_cent_s = s .* (x - lo) - mu;
    r_cent_t = t .* (up - x) - mu;

    % --- Active-set indicators for the original problem
    idlo  = (x - lo) < tolx;
    idup  = (up - x) < tolx;
    idmed = ~(idlo | idup);

    % --- Convergence tests for the original problem
    np = norm(r_primal, inf) > tolx;
    if ~any(idmed)
        nd = false;
    else
        nd = norm(charge(idmed), inf) > tolpress;
    end

    if ~any(idup)
        ndup = false;
    else
        ndup = max(charge(idup)) > tolpress;
    end

    if ~any(idlo)
        ndlo = false;
    else
        ndlo = min(charge(idlo)) < -tolpress;
    end

    % –--- Test convergence and compute new iterate
    if ~(np || nd || ndlo || ndup)
        stop = true;
        % Recompute derivatives at final x for consistency
        [~, gradf, F, G] = friction_derivatives( ...
            x, work.alpha, work.beta, work.gravit, param,options);
    else
        stop = false;
        %% === Predictor step (mu = 0) ===
        rho = -(gradf + A' * u); % == -charge (unchanged; you plan to clarify)

        rhs_pred = r_primal + A * Dinv * rho;

        % solve M * du = rhs with existing Cholesky (or regularized)
        du_pred = L' \ (L \ rhs_pred);

        dx_pred = - Dinv * (A' * du_pred - rho);

        ds_pred = - (s .* dx_pred) ./ (x - lo) - s;
        dt_pred =   (t .* dx_pred) ./ (up - x) - t;

        theta_aff = step_length(x, lo, up, s, t, dx_pred, ds_pred, dt_pred);

        mu_aff = (s + theta_aff * ds_pred)' * (x + theta_aff * dx_pred - lo);
        mu_aff = mu_aff + (t + theta_aff * dt_pred)' * (up - x - theta_aff * dx_pred);
        mu_aff = mu_aff / (2 * n);

        %% === Corrector step ===
        sigma_mu = min(1, (mu_aff/mu)^tau) * mu;

        rho_cs = s + (dx_pred .* ds_pred - sigma_mu) ./ (x - lo);
        rho_ct = t - (dx_pred .* dt_pred + sigma_mu) ./ (up - x);

        rho = -(r_dual + 0.5 * dx_pred .* G .* dx_pred + rho_cs - rho_ct);

        rhs_corr = r_primal + A * Dinv * rho;

        du_corr = L' \ (L \ rhs_corr);
        dx_corr = - Dinv * (A' * du_corr - rho);

        ds_corr = - (s .* dx_corr) ./ (x - lo) - rho_cs;
        dt_corr =   (t .* dx_corr) ./ (up - x) - rho_ct;

        theta = a_secur * step_length(x, lo, up, s, t, dx_corr, ds_corr, dt_corr);

        % --- Update variables (unchanged)
        x = x + theta * dx_corr;
        u = u + theta * du_corr;
        s = s + theta * ds_corr;
        t = t + theta * dt_corr;

        % --- Update mu (kept exactly as your code)
        mu = sigma * (s'*(x-lo) + t'*(up-x)) /(2 * n);

        % --- Refresh derivatives and matrices for next iter
        [~, gradf, F, G] = friction_derivatives( ...
            x, work.alpha, work.beta, work.gravit, param,options);

        D    = F + s ./ (x - lo) + t ./ (up - x);
        invD = 1 ./ D;
        Dinv = spdiags(invD, 0, n, n);

        M = A * Dinv * A';
        [L, delta_used, flag] = chol_with_regularization(M, param); %#ok<ASGLU>
    end
end
% U = zeros(size(x));
% U(~work.ArcT) = u;

% --- Pack outputs
work.x      = x;
work.s      = s;
work.t      = t;
work.gradf  = gradf;
work.u      = u;
work.charge = charge;
work.up     = up;
work.lo     = lo;
work.iter   = k;

if k == max_iter
    fprintf('Max iterations reached without convergence.\n');
end

end


% ===== helper: step length (kept local; can be moved out later) =====
function theta = step_length(x, lo, up, s, t, dx, ds, dt)

theta1 = 1;
idx = dx < 0;
if any(idx)
    theta1 = min(-(x(idx) - lo(idx)) ./ dx(idx));
end

theta2 = 1;
idx = dx > 0;
if any(idx)
    theta2 = min((up(idx) - x(idx)) ./ dx(idx));
end

theta3 = 1;
idx = ds < 0;
if any(idx)
    theta3 = min(s(idx) ./ (-ds(idx)));
end

theta4 = 1;
idx = dt < 0;
if any(idx)
    theta4 = min(t(idx) ./ (-dt(idx)));
end

theta = min([1, theta1, theta2, theta3, theta4]);

end
