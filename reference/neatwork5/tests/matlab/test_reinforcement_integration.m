%% 
function test_reinforcement_integration
% TEST_REINFORCEMENT_INTEGRATION Chaine de conception sur un arbre a 3 arcs.
% Depuis la racine : setup; test_reinforcement_integration
% Necessite Optimization Toolbox. Ni fichier externe, ni simulation aleatoire.
% Les coefficients simplifies permettent un calcul hydraulique a la main.
    assert(license('test','Optimization_Toolbox'), ...
        'Optimization Toolbox est necessaire pour ce test.');
    for source_height = [0 100]
      for permuted = [false true]
        [TA,TB,TN,TD,TC,p,t] = fixture();
        TN.haut = TN.haut + source_height;
        if permuted
            TA = TA([3 1 2],:);
            TN = TN([4 2 1 3],:);
            TD = TD([2 1],:);
            TB = TB([2 1],:);
            TC = TC([2 1],:);
        end
        physicalHeights = TN.haut;
        physicalX = TN.xdata;
        physicalY = TN.ydata;
        [TA,TD] = prepareReinforcementTables(TA,TD,t);
        [TA,TB,TN,TD] = enrichTablesReinf(TA,TB,TN,TD,p,source_height);
        validate_existing_pressure(TA,TD);
        closeTo(TN.haut,physicalHeights);
        closeTo(TN.xdata,physicalX);
        closeTo(TN.ydata,physicalY);
        closeTo(TA.haut,[-50;-200;-15]);
        assert(isequal(string(TA.succ),["J";"B1";"B2"]));
        closeTo(TA.pmax,[50;200;50]);
        closeTo(TA.pressure,[0;1;1]);
        closeTo(TB.pressure,[1;1]);
        [~,nodeOrder] = ismember(["S";"J";"B1";"B2"],string(TN.noeud));
        closeTo(TN.degree(nodeOrder),[1;3;1;1]);
        TR = equivResist(TA,TN,TD,t);
        dat = build_graph_artifacts(TA,TN,struct());
        closeTo(full(dat.M_path),[1 1 1;0 1 0;0 0 1]);
        [TA,TL,dat] = build_incidence_loads(TA,TN,TC,p,dat);
        speed = p; speed.servicequal = p.vmax_secu;
        [~,TLv,dat] = build_incidence_loads(TA,TN,TC,speed,dat);
        TLv{:,2:end} = max(TLv{:,2:end},TL{:,2:end});
        % Deux robinets ouverts : 2 L/s sur le tronc, 1 sur chaque branche.
        closeTo(TL{:,{'B1','B2'}},[2 2;1 0;0 1]);
        assert(isequal(TL.ID,string(TA.succ)));
        model = build_reinf_model(TA,TD,TL,TLv,TR,p,t,dat);
        sol = solveModel(model);
        % Le tronc nu exige D2 par la vitesse. B1 garde son existant ;
        % B2 doit etre renforce, et le candidat le moins cher D1 suffit.
        closeTo(sol.y,[1;0;1]);
        closeTo(sol.x_mat,[0 0 1;1 0 0]);
        closeTo(sol.fval,90);
        assert(all(sol.slack>=-1e-6));
        TO = table(.005,'VariableNames',{'in_diam'});
        [physical,nodes,TCP,TAH,cost,ctx,TS] = processSol( ...
            TA,TN,TD,TC,TO,TL,TR,dat,p,t,model,sol,struct());
        assert(height(physical)==4 && height(nodes)==4);
        closeTo(nodes.haut,physicalHeights);
        closeTo(nodes.xdata,physicalX);
        closeTo(nodes.ydata,physicalY);
        assert(nnz(TCP.nature=="old")==2 && nnz(TCP.nature=="new")==2);
        assert(isequal(string(TAH.succ),["J";"B1";"B2"]));
        closeTo(TAH.equivResist,[62500;1e6;250000]);
        % Pertes : tronc = 10*62500*.002^2 = 2.5 m ;
        % branches = 20 et 5 m. Perte locale = 1 + .001^2/1e-4.
        closeTo(TAH.pathLoss,[2.5;22.5;7.5]);
        closeTo(TAH.margin,[47.5;176.49;6.49]);
        closeTo(TS.head_margin,TAH.margin);
        closeTo(TS.head_deficit,zeros(3,1));
        closeTo(TAH.margin,sol.slack);
        closeTo(cost.costOptim,90);
        closeTo(cost.costFinal,90);
        closeTo(cost.costFixed,0);
        closeTo(ctx.reinforcement.costPipesAfterRounding,90);
        assert(all(abs(physical.lon1+physical.lon2-physical.lon)<1e-9));
        assert(isequal(sort(string(nodes.noeud)),sort(string(TN.noeud))));
        % Verifier les conduites physiques et recalculer le cout par le
        % catalogue, independamment du vecteur objectif et du post-traitement.
        addedCost = 0;
        for a = 1:height(physical)
            [ok,ids] = ismember([physical.ID1(a);physical.ID2(a)],TD.ID);
            assert(all(ok));
            if TCP.nature(a)=="new"
                addedCost = addedCost + physical.lon1(a)*TD.cost(ids(1)) ...
                    + physical.lon2(a)*TD.cost(ids(2));
            end
        end
        closeTo(addedCost,90);
        assert(nnz(string(physical.succ)=="J")==1);
        assert(nnz(string(physical.succ)=="B1")==1);
        assert(nnz(string(physical.succ)=="B2")==2);
      end
    end
    fprintf('OK: integration renforcement, entrees permutees et altitudes physiques preservees.\n');
end

function [TA,TB,TN,TD,TC,p,t] = fixture()
    TN = table(categorical(["S";"J";"B1";"B2"]), ...
        [0;-50;-200;-15],[0;1;2;2],[0;0;1;-1],[0;1;2;2],[0;0;1;1], ...
        'VariableNames',{'noeud','haut','xdata','ydata','statut','nbrob'});
    TA = table(categorical(["S";"J";"J"]), ...
        categorical(["J";"B1";"B2"]),[10;20;20],[0;20;20], ...
        ["fictif";"D1";"D1"],zeros(3,1),repmat("fictif",3,1), ...
        'VariableNames',{'pred','succ','lon','lon1','ID1','lon2','ID2'});
    TB = table(categorical(["B1";"B2"]),[9;9], ...
        'VariableNames',{'borne','pressure'});
    TD = table(["D1";"D2"],[.1;.2],[2;5],[1000;1000], ...
        ["petit";"grand"],["SDR11";"SDR11"],[0;0], ...
        'VariableNames',{'ID','diam','cost','pressure','Nominal','SDR','Roughness'});
    TC = table(categorical(["B1";"B2"]),[1;1],[1;1], ...
        'VariableNames',{'borne','proba','cible'});
    p = struct('uniform_dem',1,'bounded_flow',1,'pressure',1, ...
        'targetflow',1,'opentaps',1,'servicequal',.95,'vmax_secu',.99, ...
        'use_cvar',1,'vmax',.2,'secu',1,'pipelength',6,'pump_pressure',0);
    t = struct('lambda',2,'nu',4,'bescal',100, ...
        'faucetcoef',1e-4,'orifcoef',.001);
end

function closeTo(actual,expected)
    assert(isequal(size(actual),size(expected)),'Dimensions inattendues.');
    assert(all(abs(actual(:)-expected(:))<=1e-8*max(1,abs(expected(:)))), ...
        'Ecart avec le resultat attendu.');
end
