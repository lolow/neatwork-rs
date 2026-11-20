function [x,u,s,t,lo,up,mu] = init_point_debits(work,param)
% Build an initial primal-dual point for the IPM.
% Keeps your original logic and corrections.

    A = work.A;
    [~,n] = size(A);
    
    % variables
    x = zeros(n,1);
    u = zeros(size(A,1),1);   % dual for Ax=0
    s = ones(n,1);            % dual for x>=lo
    t = ones(n,1);            % dual for x<=up
    
    ArcB = work.ArcB;
    ArcS = work.ArcS;
    ArcT = work.ArcT;
    
    % bounds (by default use work; you consider this a "preprocessing condition")
    lo = work.xinf;
    up = work.xsup;
    
    % --- initial proposal on terminal arcs
    if param.bounded_flow == 0
        x(ArcT) = up(ArcT);
        % In your original code, bounds are multiplied by 5 in work itself.
        % Here we apply it to the local lo/up (same effect inside IPM).
        up = 5 * up;
        lo = 5 * lo;
    else
        x(ArcT) = (up(ArcT) + lo(ArcT)) / 2;
    end
    
    if 1
        b = find(ArcS == 1);
        nS = numel(b);
        X = zeros(length(x),nS);
        for i = 1:nS
            X(:,i) = x;
            ArcS_sel = false(length(x),1);
            ArcS_sel(b(i)) = 1;
            rinit = A * X(:,i);
            ArcBS = ArcB | ArcS_sel;
            B = A(:, ArcBS);
            M = B * B';
            du = M \ rinit;
            dx = -B' * du;
            X(ArcBS,i) = X(ArcBS,i) + dx;
        end
        x =  mean(X,2);
    end
   



    if 0
        % --- assign flows at stations
        tot = sum(x);
        x(ArcS) = - tot / sum(ArcS);

        % Keep only one station active for the feasibility correction (your code)
        ArcS_sel = ArcS;
        b = find(ArcS_sel == 1);
        if sum(ArcS_sel) > 1
            ArcS_sel(b(2:end)) = 0;
            x(b(1)) = 0;
        end

        r_init = A * x;

        ArcBS = ArcB | ArcS_sel;
        B = A(:, ArcBS);

        % Solve M*du = r_init, dx = -B'*du
        M = B * B';
        du = M \ r_init;
        dx = -B' * du;

        x(ArcBS) = x(ArcBS) + dx;
    end
    
    % --- initialize s,t,mu consistently with current x
    s = 1 ./ (x - lo);
    t = 1 ./ (up - x);
    mu = (s'*(x-lo) + t'*(up-x)) /(2 * n);

end
