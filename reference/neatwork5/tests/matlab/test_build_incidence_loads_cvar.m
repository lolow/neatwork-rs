function test_build_incidence_loads_cvar
% Small synthetic network: verify CVaR against a weighted sorted tail.
    rngState = rng;
    restoreRng = onCleanup(@() rng(rngState)); %#ok<NASGU>
    TA = table("B1",2,'VariableNames',{'succ','statut'});
    TN = table(["S";"B1"],'VariableNames',{'noeud'});
    TC = table(repmat("B1",3,1),[1;.3;.7],[1;2;4], ...
        'VariableNames',{'borne','proba','cible'});
    dat = struct('M_incid',sparse(1),'M_path',sparse(1));
    par = struct('uniform_dem',0,'nScenLoad',101,'servicequal',.65);

    % Absence of the new option must retain the historical VaR behavior.
    rng(42);
    [~,defaultTL] = build_incidence_loads(TA,TN,TC,par,dat);
    par.use_cvar = 0;
    rng(42);
    [~,varTL] = build_incidence_loads(TA,TN,TC,par,dat);
    assert(isequal(defaultTL,varTL));

    rng(42);
    scenarios = (rand(height(TC),par.nScenLoad)<TC.proba).*TC.cible;
    descending = sort(sum(scenarios,1),'descend');
    par.use_cvar = 1;
    for alpha = [.5 .65 .95 1]
        par.servicequal = alpha;
        rng(42);
        [~,TL] = build_incidence_loads(TA,TN,TC,par,dat);
        if alpha==1
            expected = descending(1);
        else
            tailMass = (1-alpha)*par.nScenLoad;
            whole = floor(tailMass);
            fraction = tailMass-whole;
            expected = (sum(descending(1:whole)) + ...
                fraction*descending(whole+1))/tailMass;
        end
        assert(abs(TL.B1-expected)<1e-12);
        assert(isequal(TL.ID,"B1"));
        if alpha==.65
            assert(TL.B1>=varTL.B1);
        end
    end
    % All observations tied: CVaR must equal the constant demand.
    TC.proba(:) = 1;
    par.servicequal = .95;
    [~,TL] = build_incidence_loads(TA,TN,TC,par,dat);
    assert(abs(TL.B1-sum(TC.cible))<1e-12);

    % Exact binomial branch: independently integrate the discrete upper tail.
    par.uniform_dem = 1;
    par.targetflow = .4;
    for n = [0 1 4 20]
        TA.nbrob = n+1;
        values = n:-1:0;
        for p = [.01 .3 .95 1]
            par.opentaps = p;
            probabilities = binopdf(values,n,p);
            for alpha = [.5 .65 .95 .999 1]
                par.servicequal = alpha;
                [~,TL] = build_incidence_loads(TA,TN,TC,par,dat);
                if alpha==1
                    expected = 1+n;
                else
                    remaining = 1-alpha;
                    tailSum = 0;
                    for j = 1:numel(values)
                        take = min(remaining,probabilities(j));
                        tailSum = tailSum+take*(1+values(j));
                        remaining = max(0,remaining-take);
                    end
                    expected = tailSum/(1-alpha);
                end
                assert(abs(TL.B1/par.targetflow-expected)<1e-9);
            end
        end
    end
    % Historical uniform-demand behavior also remains the default.
    par.servicequal = .65;
    par.opentaps = .3;
    par = rmfield(par,'use_cvar');
    [~,defaultTL] = build_incidence_loads(TA,TN,TC,par,dat);
    par.use_cvar = 0;
    [~,varTL] = build_incidence_loads(TA,TN,TC,par,dat);
    assert(isequal(defaultTL,varTL));
    checkUniformProbabilityBounds();
    fprintf('OK: empirical and exact binomial CVaR, ties and boundary cases.\n');
end

function checkUniformProbabilityBounds()
% S-J with two clients on B1 and one on B2: the last link has no idx1 entry.
    TA = table(["J";"B1";"B2"],[0;2;2],[0;2;1], ...
        'VariableNames',{'succ','statut','nbrob'});
    TN = table(["S";"J";"B1";"B2"],'VariableNames',{'noeud'});
    TC = table(["B1";"B1";"B2"],ones(3,1),.4*ones(3,1), ...
        'VariableNames',{'borne','proba','cible'});
    dat = struct('M_incid',speye(3), ...
        'M_path',sparse([1 1 1;0 1 0;0 0 1]));
    par = struct('uniform_dem',1,'targetflow',.4,'opentaps',1);
    expected = [1.2 1.2;.8 0;0 .4]; % Total flow of all dependent clients, L/s.
    for risk = [0 1]
        par.use_cvar = risk;
        par.opentaps = 1;
        for alpha = [.5 .95 1]
            par.servicequal = alpha;
            [~,TL] = build_incidence_loads(TA,TN,TC,par,dat);
            assert(isequal(TL.ID,TA.succ));
            actual = full(TL{:,{'B1','B2'}});
            assert(all(abs(actual(:)-expected(:))<1e-12));
        end
        par.opentaps = 0;
        assertZeroProbabilityRejected(TA,TN,TC,par,dat);
        % A single client leaves idx1 empty; it must also be rejected.
        singleTA = TA(3,:);
        singleTN = TN([1 4],:);
        singleTC = TC(3,:);
        singleDat = struct('M_incid',sparse(1),'M_path',sparse(1));
        assertZeroProbabilityRejected(singleTA,singleTN,singleTC,par,singleDat);
    end
end

function assertZeroProbabilityRejected(TA,TN,TC,par,dat)
    try
        build_incidence_loads(TA,TN,TC,par,dat);
    catch err
        assert(strcmp(err.identifier,'NW5:LoadProbability'));
        return
    end
    error('test_build_incidence_loads_cvar:MissingError', ...
        'Uniform design must reject opentaps=0.');
end
