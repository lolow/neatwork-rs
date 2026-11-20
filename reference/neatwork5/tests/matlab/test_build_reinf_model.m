function test_build_reinf_model
% TEST_BUILD_REINF_MODEL Test autonome de la formulation de renforcement.
% Executer : test_build_reinf_model (Optimization Toolbox pour la resolution).
    [TA,TD,TL,TLv,TR,pardsg,partec,dat] = fixture();
    build = @(a,d,l,v,r,p,t,m) build_reinf_model(a,d,l,v,r,p,t,m);
    model = build(TA,TD,TL,TLv,TR,pardsg,partec,dat);
    assert(model.nbA==3 && model.nbD==2 && model.nVar==9);
    assert(issparse(model.Aeq) && issparse(model.Aineq));
    assert(isequal(size(model.Aeq),[3 9]));
    assert(isequal(size(model.Aineq),[3 9]));
    assert(isequal(model.f,[20;50;40;100;60;150;0;0;0]));
    assert(isequal(model.lb,[zeros(6,1);1;0;0]));
    % Arc 1 : D1 trop lent ; arc 2 : pression insuffisante pour D1.
    % Arc 3 : renforcement bloque, meme si les x restent admissibles.
    assert(isequal(model.ub,[0;1;0;1;1;1;1;1;0]));
    assert(isequal(model.activ_variables,model.ub==1));
    assert(isequal(model.beq,zeros(3,1)));
    assert(isequal(full(model.Aeq), ...
        [1 1 0 0 0 0 -1 0 0;0 0 1 1 0 0 0 -1 0; ...
         0 0 0 0 1 1 0 0 -1]));
    assert(isequal(model.Aineq,[model.Aineq_x model.Aineq_y]));

    % Oracle hydraulique : calculer directement les pertes sur chaque chemin,
    % sans reconstruire les matrices de la fonction testee.
    for bounded = [0 1]
        for uniform = [0 1]
            if ~bounded && ~uniform, continue; end
            p = pardsg;
            p.bounded_flow = bounded;
            p.uniform_dem = uniform;
            for lambda = [1.2 1.781 2]
                t = partec; t.lambda = lambda;
                d = TD; d.cond = d.resist.^(-1/lambda);
                r = TR;
                for a = 1:3
                    for j = 1:2
                        r{a,j+2} = (r.rexist(a)^(-1/lambda) + ...
                            d.resist(j+1)^(-1/lambda))^(-lambda);
                    end
                end
                m = build(TA,d,TL,TLv,r,p,t,dat);
                % Inclure le reseau initial, le renforcement total et un
                % melange fractionnaire ; l'identite vaut hors faisabilite.
                for X = {zeros(2,3),ones(2,3)/2,[.2 .7 0;.8 0 .3]}
                    x = X{1}; y = sum(x,1).'; z = [x(:);y];
                    loss = zeros(3,1);
                    for a = 1:3
                        q = max(TL{a,2:end})/1000;
                        base = 0;
                        if isfinite(r.rexist(a)), base = r.rexist(a); end
                        resistance = (1-y(a))*base;
                        for j = 1:2
                            resistance = resistance+x(j,a)*r{a,j+2};
                        end
                        loss(a) = TA.lon(a)*resistance*q^lambda;
                    end
                    margin = zeros(3,1);
                    for node = 1:3
                        available = -TA.haut(node);
                        if TA.statut(node)==2
                            available = available/p.secu;
                            if bounded, available = available-TA.pressure(node); end
                            if uniform
                                available = available-(p.targetflow/1000)^2/t.faucetcoef;
                            end
                        end
                        path = find(dat.M_path(:,node));
                        margin(node) = sum(loss(path))-available;
                    end
                    closeTo(m.Aineq*z-m.bineq,margin);
                    closeTo(m.Aeq*z,zeros(3,1));
                end
            end
        end
    end

    % Chaque candidat doit pouvoir porter le debit total, meme en presence
    % d'un existant. D1 est donc interdit sur les trois arcs a ce debit.
    slow = pardsg; slow.vmax = .18;
    v = TLv; v{:,2:end} = .5;
    unblocked = TA; unblocked.N(:) = "N";
    m = build(unblocked,TD,TL,v,TR,slow,partec,dat);
    assert(isequal(m.ub(1:6),[0;1;0;1;0;1]));
    assert(isequal(m.lb(7:9),ones(3,1)));
    checkMandatorySpeedReinforcement();

    fixed = TA; fixed.fixedCost = [7;11;13];
    m = build(fixed,TD,TL,TLv,TR,pardsg,partec,dat);
    closeTo(m.f,[model.f(1:6);7;11;13]);

    % Les colonnes TR sont identifiees par leur nom, comme le catalogue.
    reordered = TR(:,{'ID','D2','rexist','D1'});
    m = build(TA,TD,TL,TLv,reordered,pardsg,partec,dat);
    closeTo(m.Aineq,model.Aineq);
    closeTo(m.ub,model.ub);

    mustFail(@() build(TA,removevars(TD,'resist'),TL,TLv,TR,pardsg,partec,dat),'NW5:MissingResistance');
    mustFail(@() build(TA,removevars(TD,'cond'),TL,TLv,TR,pardsg,partec,dat),'NW5:MissingConductance');
    mustFail(@() build(TA,TD(2:end,:),TL,TLv,TR,pardsg,partec,dat),'NW5:FictitiousPipe');
    mustFail(@() build(TA,TD([1 1 2 3],:),TL,TLv,TR,pardsg,partec,dat),'NW5:FictitiousPipe');
    mustFail(@() build(TA,TD,TL,TLv,TR(1:2,:),pardsg,partec,dat),'NW5:TRSize');
    mustFail(@() build(TA,TD,TL,TLv,TR([2 1 3],:),pardsg,partec,dat),'NW5:TRAlignment');
    mustFail(@() build(TA,TD,TL,TLv,removevars(TR,'D1'),pardsg,partec,dat),'NW5:TRVariables');
    blocked = TA; blocked.N(1) = "B";
    mustFail(@() build(blocked,TD,TL,TLv,TR,pardsg,partec,dat), ...
        'NW5:ReinforcementBlocked','J');
    unsupported = pardsg; unsupported.bounded_flow=0; unsupported.uniform_dem=0;
    mustFail(@() build(TA,TD,TL,TLv,TR,unsupported,partec,dat),'');

    % Optimum connu : seul l'arc nu doit etre equipe, avec D2 (D1 interdit).
    % Verifier le MILP reel quand le solveur est disponible.
    if exist('intlinprog','file')==2 && license('test','Optimization_Toolbox')
        options = optimoptions('intlinprog','Display','off');
        [z,cost,flag] = intlinprog(model.f,7:9,model.Aineq,model.bineq, ...
            model.Aeq,model.beq,model.lb,model.ub,options);
        assert(flag>0,'La resolution du MILP a echoue.');
        closeTo(cost,50);
        closeTo(z,[0;1;0;0;0;0;1;0;0]);
        assert(all(model.Aineq*z<=model.bineq+1e-8));
        fprintf('OK: optimum MILP connu, cout = 50.\n');
    else
        fprintf('SKIP: resolution MILP (Optimization Toolbox indisponible).\n');
    end
    fprintf('OK: build_reinf_model, bornes, couts, chemins et integrite.\n');
end

function [TA,TD,TL,TLv,TR,p,t,dat] = fixture()
    t = struct('lambda',2,'faucetcoef',1e-6);
    p = struct('vmax',1,'targetflow',.4,'secu',1.25, ...
        'bounded_flow',1,'uniform_dem',1);
    TD = table(["fictif";"D1";"D2"],[0;.05;.2],[0;10;30], ...
        [0;2;5],[Inf;1e6;1e4], ...
        'VariableNames',{'ID','diam','pressure','cost','resist'});
    TD.cond = TD.resist.^(-1/t.lambda);
    TA = table(["J";"B1";"B2"],[10;20;30],[-1000;-1500;-2000], ...
        [1;20;1],[0;2;2],[0;2;3],["N";"N";"B"], ...
        ["fictif";"D1";"fictif"],["fictif";"fictif";"D1"], ...
        'VariableNames',{'succ','lon','haut','pmax','statut','pressure','N','ID1','ID2'});
    TA.lon1 = [0;20;0];
    TA.lon2 = [0;0;30];
    TL = table(TA.succ,[2;.5;1],[1;1;0], ...
        'VariableNames',{'ID','B1','B2'});
    TLv = table(TA.succ,[10;1;.5],[3;.5;1], ...
        'VariableNames',{'ID','B1','B2'});
    existing = [Inf;1e6;1e6];
    candidate = (existing.^(-1/t.lambda)+TD.cond(2:3).').^(-t.lambda);
    TR = table(TA.succ,existing,candidate(:,1),candidate(:,2), ...
        'VariableNames',{'ID','rexist','D1','D2'});
    dat.M_path = sparse([1 1 1;0 1 0;0 0 1]);
end

function checkMandatorySpeedReinforcement()
    [TA,TD,TL,TLv,TR,p,t,dat] = fixture();
    TA.N(:) = "N";
    % D1 can carry only about 1.9635 L/s at vmax=1 m/s. The speed-control
    % flow is 2 L/s although its design-flow row in TL reaches only 1 L/s.
    TLv{2,2:end} = 2;
    m = build_reinf_model(TA,TD,TL,TLv,TR,p,t,dat);
    assert(isequal(m.lb(7:9),[1;1;0]));
    % The previous no-reinforcement choice on B1 still meets all hydraulic
    % inequalities: it is the speed test alone that now makes y(B1)=1.
    previousChoice = [0;1;0;0;0;0;1;0;0];
    assert(all(m.Aineq*previousChoice<=m.bineq+1e-8));
    closeTo(m.Aeq*previousChoice,m.beq);

    blocked = TA; blocked.N(2) = "B";
    mustFail(@() build_reinf_model(blocked,TD,TL,TLv,TR,p,t,dat), ...
        'NW5:ReinforcementBlocked','B1');

    % A short small-diameter second section controls the existing capacity,
    % even though most of the pipe is the much larger D2.
    split = TA;
    split.ID1(2) = "D2"; split.lon1(2) = 19.9;
    split.ID2(2) = "D1"; split.lon2(2) = .1;
    splitTR = TR;
    splitTR.rexist(2) = (19.9*TD.resist(3)+.1*TD.resist(2))/20;
    splitTR{2,{'D1','D2'}} = (splitTR.rexist(2)^(-1/t.lambda) + ...
        TD.cond(2:3).').^(-t.lambda);
    m = build_reinf_model(split,TD,TL,TLv,splitTR,p,t,dat);
    assert(m.lb(8)==1);

    % A fictitious zero-length section has zero diameter but does not limit
    % an otherwise sufficient existing D2 pipe.
    split.ID2(2) = "fictif"; split.lon2(2) = 0;
    split.lon1(2) = split.lon(2);
    splitTR.rexist(2) = TD.resist(3);
    splitTR{2,{'D1','D2'}} = (splitTR.rexist(2)^(-1/t.lambda) + ...
        TD.cond(2:3).').^(-t.lambda);
    m = build_reinf_model(split,TD,TL,TLv,splitTR,p,t,dat);
    assert(m.lb(8)==0);

    % A blocked arc remains valid when its existing pipe alone is sufficient.
    split.N(2) = "B";
    m = build_reinf_model(split,TD,TL,TLv,splitTR,p,t,dat);
    assert(m.lb(8)==0 && m.ub(8)==0);

    % Symmetric case: equipment carried entirely by lon2 must also trigger
    % mandatory reinforcement once its speed-control flow is excessive.
    TLv{3,2:end} = 2;
    m = build_reinf_model(TA,TD,TL,TLv,TR,p,t,dat);
    assert(m.lb(9)==1);
end

function closeTo(actual,expected)
    assert(isequal(size(actual),size(expected)),'Dimensions inattendues.');
    assert(all(abs(actual(:)-expected(:))<=1e-9*max(1,abs(expected(:)))), ...
        'Ecart numerique avec le resultat attendu.');
end

function mustFail(action,identifier,expectedText)
    if nargin<3, expectedText=''; end
    try
        action();
    catch exception
        if ~isempty(identifier)
            assert(strcmp(exception.identifier,identifier), ...
                'Erreur inattendue : %s',exception.message);
        end
        if ~isempty(expectedText)
            assert(contains(exception.message,expectedText), ...
                'Le diagnostic doit identifier la liaison : %s',expectedText);
        end
        return
    end
    error('test_build_reinf_model:MissingError','Une entree invalide a ete acceptee.');
end
