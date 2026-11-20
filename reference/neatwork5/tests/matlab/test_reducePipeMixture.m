function test_reducePipeMixture
% Compare the hull with exhaustive feasible pairs, independently of slopes.
    rngState = rng;
    restoreRng = onCleanup(@() rng(rngState)); %#ok<NASGU>
    rng(42);
    check([.2;.5;.3],[10;5;1],[1;9;10]); % dominated middle point
    check([.2;.5;.3],[10;5;1],[1;3;10]); % efficient middle point
    check([.2;.5;.3],[5;5;5],[7;2;8]);   % identical resistances
    check([.2;.5;.3],[1;2;3],[3;2;1]);   % collinear costs
    for trial = 1:100
        n = randi([3,15]);
        x = rand(n,1); x = x/sum(x);
        rho = randi([1,20],n,1);
        cost = 20*rand(n,1);
        check(x,rho,cost);
        % Equivalent coefficients with a real existing pipe, then a fictive
        % existing pipe (rho itself). No ordering of costs is assumed.
        lambda = 1.781;
        equivalent = (rho.^(-1/lambda)+7^(-1/lambda)).^(-lambda);
        check(x,equivalent,cost);
    end
    fprintf('OK: pipe mixtures preserve resistance and attain minimum pair cost.\n');
end

function check(x,rho,cost)
    result = reducePipeMixture(x,rho,cost);
    target = rho'*x;
    best = inf;
    for a = 1:numel(x)
        if abs(rho(a)-target)<1e-12
            best = min(best,cost(a));
        end
        for b = a+1:numel(x)
            if rho(a)==rho(b), continue; end
            weight = (target-rho(b))/(rho(a)-rho(b));
            if weight>=0 && weight<=1
                best = min(best,weight*cost(a)+(1-weight)*cost(b));
            end
        end
    end
    assert(nnz(result)<=2 && all(result>=0));
    assert(abs(sum(result)-sum(x))<1e-12);
    assert(abs(rho'*result-target)<1e-10*max(1,abs(target)));
    assert(abs(cost'*result-best)<1e-10*max(1,abs(best)));
end
