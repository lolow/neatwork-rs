function [f, gradf, F, G] = friction_derivatives(x, alpha, beta, gravit, param,options)
% Friction function and derivatives (same formulas as your fDeriv)
% Returns:
%   f     scalar objective contribution
%   gradf gradient (n x 1)
%   F     second derivative diagonal (n x 1)
%   G     third derivative diagonal (n x 1)

tolx  = options.tolx;
lbd   = param.lambda;

vec1 = (abs(x) > tolx);
vec2 = ~vec1;

absx = abs(x);

absx_lbd = zeros(length(x),1);
absx_lbd(vec1) = absx(vec1).^(lbd-1);

% Compute the objective function f
fvec = zeros(length(x),1);
fvec(vec1) = (beta(vec1) .* absx_lbd(vec1) / (lbd+1) ...
           +  alpha(vec1).* absx(vec1) / 3) .* x(vec1).^2;

fvec(vec2) = tolx^(lbd+1) /(lbd+1) * beta(vec2);
fvec(vec2) = fvec(vec2) + 0.5* beta(vec2).*(x(vec2).^2 *tolx^(-2)-tolx^(lbd+1));

f = sum(fvec) + gravit' * x;

% gradf
gradf = zeros(length(x),1);
gradf(vec1) = beta(vec1).* (absx_lbd(vec1) .* x(vec1)) + ...
              alpha(vec1).* absx(vec1) .* x(vec1) + gravit(vec1);

gradf(vec2) = tolx^(lbd-1) * beta(vec2) .* x(vec2) + ...
              tolx*alpha(vec2) .* x(vec2) + gravit(vec2);

% F (second derivative)
F = zeros(length(x),1);
F(vec1) = lbd*beta(vec1) .* absx_lbd(vec1) + 2 * alpha(vec1) .* absx(vec1);
F(vec2) = tolx^(lbd-1) * beta(vec2) + tolx*alpha(vec2);

% G (third derivative)
G = zeros(length(x),1);
G(vec1) = lbd*(lbd-1)*beta(vec1).*absx_lbd(vec1)./x(vec1) ...
        + 2*alpha(vec1).*sign(x(vec1));
% G(vec2) remains 0 (as in the original code)

end
