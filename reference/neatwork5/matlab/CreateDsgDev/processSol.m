    function [TA_out,TN_out,TCP,TAH,reinfcost,ctx,TS] = processSol(TA,TN,TD,TC,TO,TL,TR,dat,pardsg,partec,model,sol,ctx)
%PROCESSSOL Transforme la solution LP/MILP en reseau physique NW5.
%   Les cinq etapes de post-traitement portent sur l'ARBRE de depart :
%   1. Ajustement commercial des renforcements, apres suppression des
%      raccordements intermediaires (faite dans la section preliminaire).
%   2. Resistance equivalente des liaisons, avec ou sans arcs paralleles.
%   3. Pertes de charge, contraintes et marges de pression.
%   4. Orifices aux terminaux (cas NW4).
%   5. Construction de TA avec arcs paralleles, sans nouveau noeud.
%
%   Convention : TD.resist et TR.rexist sont des resistances LINEIQUES.
%   La resistance totale d'une liaison est TA.lon * resistance_lineique.
%   TL et dat.M_path decrivent l'arbre AVANT insertion des arcs paralleles.
%   Les calculs intermediaires sont conserves dans ctx.reinforcement ;
%   TA ne recoit pas de coefficient hydraulique de resistance.
%   TS records all post-adjustment head constraints at downstream nodes,
%   excluding the source. Heads and margins are in metres of water, using
%   the design model (including its safety factor and local equipment losses),
%   before final regulation. These are not hydraulic simulation results.

%   A. Verify input data and prepare data



    if nargin < 13 || isempty(ctx), ctx = struct(); end


    % ===== A. Verify input data and prepare data ======
   
    % ---- Check conformity ----
    status = integrity(TA,TN,TD,TC,TO,TL,dat,pardsg,partec,model,sol,ctx);
    assert(strcmp(status,'OK'), 'NW5:Input', 'Donnees non valides.');

    nbA = height(TA);
    TA.ID1 = string(TA.ID1); TA.ID2 = string(TA.ID2);
    TD.ID = string(TD.ID);
    [ok1,idx1] = ismember(TA.ID1,TD.ID);
    [ok2,idx2] = ismember(TA.ID2,TD.ID);
    assert(all(ok1 & ok2), 'NW5:UnknownPipe', ...
        'Assigned pipe in base network absent from catalog TD.');
    if ~ismember('diam1',TA.Properties.VariableNames)
        TA.diam1 = TD.diam(idx1);
    end
    if ~ismember('diam2',TA.Properties.VariableNames)
        TA.diam2 = TD.diam(idx2);
    end
    
    % --- Construct table of real pipes ---

    realPipe = TD.ID ~= "fictif";
    Td = TD(realPipe,:);                  % ordre des lignes de sol.x_mat
    nbD = height(Td);
    idsTR = string(TR.Properties.VariableNames(3:end));
    assert(isequal(Td.ID(:),idsTR(:)), 'NW5:PipeOrder', ...
        'Inconsistent pipe order between TD et TR.');
    assert(all(isfinite(Td.resist) & Td.resist > 0), 'NW5:Resistance', ...
        'TD.resist for real pipes must be finite and positive.');
    assert(ismember('rexist',TR.Properties.VariableNames), ...
        'NW5:Resistance', 'Missing TR.rexist.');
    assert(height(TR)==nbA && ...
           isequal(string(TR.ID(:)),string(TA.succ(:))), ...
        'NW5:PipeOrder', 'rows in TR and TA are not aligned.');
    rExist = TR.rexist(:); % Arc resistance in the base network

    % --- Arc tables for base network and for reinforcement ---
    % Teq and Tre are similar to TA. 
    % Teq captures the existing equipment, Tre captures the reinforcement.
    % Teq and Tre are used to construct TA_out, which contains both 
    % existing and reinforced arcs.

    varTA = {'pred','succ','lon','lon1','ID1','lon2','ID2',...
        'N','equip','diam1','diam2'};
    Teq = TA(:,varTA);
    Teq.N = string(Teq.N);
    Tre = Teq;
    Tre.N(Tre.N == "N") = "R";
    Tre(:,{'ID1','ID2'}) = array2table(repmat("fictif",height(Tre),2));
    Tre(:,{'lon1','lon2','diam1','diam2'}) = array2table(zeros(height(Tre),4));
    Tre(:,{'nature'}) = array2table(repmat("reinf",nbA,1));
    Teq(:,{'nature'}) = array2table(repmat("equip",nbA,1));

    % ========= B. Retrieve solution ========

    X = full(sol.x_mat);  % nbD x nbA, x already cleaned by solveModel
    y = full(sol.y(:));
	assert(all(y<1e-6 | y>1-1e-6),'Variable y is not binary')
    reinforced = y > 0.5;
    % setReinf = find(y > 0.5);
    equipped = TA.lon1 > 0 | TA.lon2 > 0;

    % Preserve the solver cost before reducing redundant pipe mixtures.
    costPipesOptim = sum(TA.lon .* (Td.cost' * X)');
    candidateR = table2array(removevars(TR,{'ID','rexist'}))';
    assert(isequal(size(candidateR),size(X)) && ...
           all(isfinite(candidateR(:)) & candidateR(:)>0), ...
        'NW5:Resistance','Resistances candidates invalides dans TR.');
    reducedArcs = find(sum(X>0,1)>2);
    for i = reducedArcs
        X(:,i) = reducePipeMixture(X(:,i),candidateR(:,i),Td.cost);
    end
    costPipesAfterReduction = sum(TA.lon .* (Td.cost' * X)');

    % ===== C. Assign iD's and lengths to Tre
 
    idXtran = X' > 0;
    for i = find(reinforced(:))'
        J = find(idXtran(i,:));
        assert(numel(J)>=1 && numel(J)<=2, ...
            'Nombre de tuyaux incorrect sur l''arc %d',i);
        [~,order] = sort(Td.diam(J),'descend');
        J = J(order);

        Tre.ID1(i) = Td.ID(J(1));
        Tre.diam1(i) = Td.diam(J(1));
        if numel(J)==2
            Tre.ID2(i) = Td.ID(J(2));
            Tre.diam2(i) = Td.diam(J(2));
        end
    end

    % ===== D. Optimal pipe cost before adjusting solution =====

    costFixed = model.f(nbD*nbA+ (1:nbA))' * y;

       
    % ======= E. Eliminate junctions to existing arcs ========
    % build_reinf_model assumes that the junction between two reinforcement
    % pipe sections is also connected to the existing pipe. The final network
    % does not introduce this intermediate network node: the reinforcement
    % remains a separate branch connected to the existing pipe only at the
    % link endpoints. The two reinforcement sections retain their series
    % connection, without an intermediate connection to the existing pipe.
    % Their length proportions are adjusted algebraically to preserve the
    % equivalent link resistance and hence the modeled headloss constraints.
    % This produces a feasible alternative whose cost may increase; global
    % cost optimality is no longer guaranteed after this adjustment.

    doubled = reinforced & equipped;
    assert(all(equipped | reinforced), 'NW5:DisconnectedArc', ...
        'Non equipped base arc should be reinforced.');
    assert(all(isfinite(rExist(equipped)) & rExist(equipped)>0) && ...
           all(isinf(rExist(~equipped))), 'NW5:Resistance', ...
        'TR.rexist does not match the base equipement.');

	% --- Change proportions on reinforced arcs ---
	% See technical note for the algebra
 		   
    splitDoubled = doubled & (sum(X>0,1)'==2);
    rTarget = sum(X(:,splitDoubled).*candidateR(:,splitDoubled),1)';
    arcs = find(splitDoubled);
    for k = 1:numel(arcs)
        i = arcs(k);
        ids = find(X(:,i)>0);
        rho = Td.resist(ids);
        deltaC = rTarget(k)^(-1/partec.lambda) ...
               - rExist(i)^(-1/partec.lambda);
        assert(deltaC>0, 'NW5:Junction', ...
            'Arc %d : resistance equivalent incompatible avec l''existant.',i);
        rhoTarget = deltaC^(-partec.lambda);
        if rho(1)==rho(2)
            assert(abs(rhoTarget-rho(1)) <= 1e-8*max(1,rho(1)), ...
                'NW5:Junction', 'Arc %d : resistance cible incoherente.',i);
            continue
        end
        eta1 = (rhoTarget-rho(2))/(rho(1)-rho(2));
        assert(eta1>=-1e-8 && eta1<=1+1e-8, ...
            'NW5:Junction', 'Arc %d : proportions inadmissibles.',i);
        eta1 = min(1,max(0,eta1));
        X(ids,i) = [eta1;1-eta1];
    end

    % --- Check whether resistance was altered ---
    rBefore = inf(nbA,1);
    rBefore(reinforced) = (Td.resist' * X(:,reinforced))';
    rEqBefore = parallelResistance(rExist,rBefore,partec.lambda);

    % rBefore = inf(nbA,1);
    % for i = find(reinforced(:))'
    %     rBefore(i) = sum(X(:,i).*Td.resist);
    % end
    % rEqBefore = parallelResistance(rExist,rBefore,partec.lambda);
    
    costPipesBeforeRounding = sum(TA.lon .* (Td.cost' * X)');
    if any(splitDoubled)
        relativeError = abs(rEqBefore(splitDoubled)-rTarget) ./ ...
            max(abs(rTarget),realmin);
        assert(all(relativeError < 1e-7), 'NW5:Junction', ...
            'La suppression du raccordement ne conserve pas la resistance.');
    end
    costPipesInter = sum(TA.lon .* (Td.cost' * X)');


    % ===== F. Adjust to commercial pipe length =====

    % --- Increase length of the pipe with the larger diameter ---
   
	% splitReinf = reinforced & (sum(X>0,1)'==2);

    for i = find(reinforced(:))'
        J = find(idXtran(i,:));
        % Put the largest diameter in the first place
        [~,order] = sort(Td.diam(J),'descend');
        J = J(order);
        L = TA.lon(i);
        % Round up L to the upper multiple of commercial pipe length
        if numel(J)==1
            Tre.lon1(i) = L;
        else
            L1 = L * X(J(1),i);
            if pardsg.pipelength > 0.1
                L1 = min(L, ceil(L1/pardsg.pipelength)*pardsg.pipelength);
            end
            Tre.lon1(i) = L1;
            Tre.lon2(i) = L-L1;
            % Update X
            X(J,i) = [L1; L-L1]/L;
        end
    end
    % Normalize lengths and IDs for fictive pipes
    Tre.ID1(Tre.lon1 == 0) = "fictif";
    Tre.ID2(Tre.lon2 == 0) = "fictif";
    Tre.diam1(Tre.lon1 == 0) = 0;
    Tre.diam2(Tre.lon2 == 0) = 0; 
    
    % --- Final cost after rounding ---
    costPipesFinal = sum(TA.lon .* (Td.cost' * X)');
 

    % ===== G. Total headloss and  margin ========

    % Create a table of hydraulic complements on links
    
    TAH = TA(:,{'pred','succ'});

    % --- Compute resistances --- 
	% Existing network: rEq = rExist
    % Reinforced arcs rEq = rNew.
    % Compound resistance on parallel arcs by summing conductances.

    rNew = inf(nbA,1);
    rNew(reinforced) = (Td.resist' * X(:,reinforced))';
    rEqNew = parallelResistance(rExist,rNew,partec.lambda);
    assert(all(isfinite(rEqNew) & rEqNew>0), 'NW5:Resistance', ...
        'A link has an infinite resistance.');
    % record resistance
    TAH.equivResist = rEqNew;
	
    % --- Design flow   ---
    Q = table2array(TL(:,2:end));               % L/s, arcs x bornes
    Qmax = full(max(Q,[],2))/1000;     % m3/s
    TAH.designFlow = Qmax*1000; % in l/s

    % Diameter selection checks vmax using the estimated total link flow
    % for each pipe, without distributing it among parallel pipes. This is
    % conservative provided that this estimate bounds each individual pipe
    % flow after reinforcement. The final hydraulic flow split and resulting
    % pipe velocities are not recomputed here.

    % Headlosses induced by the design flow 
    betaEq = TA.lon .* rEqNew;            % resistance TOTALE de la liaison
    connectionLoss = betaEq .* Qmax.^partec.lambda;
    pathLoss = dat.M_path' * connectionLoss;

    % --- Gravity pressure minus loss due to local equipment ---
    terminal = (TA.statut == 2);
    available = -TA.haut;
    available(terminal) = available(terminal)/max(pardsg.secu,eps);
    required = zeros(nbA,1);
    cible = pardsg.targetflow/1000;
    if pardsg.bounded_flow==1
        required(terminal) = TA.pressure(terminal);
    elseif pardsg.uniform_dem~=1
        % Extension homogene par borne : adapter ensemble marges et orifices,
        % avec la simulation Python (docs/forum/a_faire.md, section 2).
        error('NW5:Orifices', ...
              'Demandes heterogenes non bornees non traitees.');
    end
    if pardsg.uniform_dem == 1
        required(terminal) = required(terminal) + ...
            cible^2/max(partec.faucetcoef,eps);
    end

    % --- Residual pressure ---
    residualPressure = available - pathLoss;
    margin = residualPressure - required; 
    violation = max(0,-margin);
    constraintType = repmat("intrusion_prevention",nbA,1);
    constraintType(terminal) = "demand_satisfaction";
    TS = table(TA.succ, constraintType, required, residualPressure, margin, violation, ...
        'VariableNames', {'noeud','constraint_type','head_required', ...
                          'head_available','head_margin','head_deficit'});
    TS.Properties.VariableUnits = {'','','m','m','m','m'};
    TS.Properties.Description = ...
        'Post-adjustment design diagnostics before final regulation; simulation required.';
    if any(violation>1e-6)
        warning('NW5:PressureAfterRounding', ...
            ['%d contraintes hydrauliques depassees apres ajustement. ' ...
             'Violation maximale : %.6g m.'],nnz(violation>1e-6),max(violation));
    end

    % Controle de l'impact du seul arrondi (meme equipement, meme Qmax).
    % betaBefore = TA.lon .* rEqBefore;
    % marginBefore = available - required- ...
    %     dat.M_path'*(betaBefore.*Qmax.^partec.lambda);
    % 
    % betaNew = TA.lon .* rEqNew;
    % marginNew = available - required- ...
    %     dat.M_path'*(betaNew.*Qmax.^partec.lambda);

    TAH.pathLoss = pathLoss;
    TAH.margin = margin;


    % ===== H. Orifices to compensate margins (NW4) ======

    ideal = ones(nbA,1);
    commercial = ones(nbA,1);
    if pardsg.uniform_dem==1 && pardsg.bounded_flow==0
        use = terminal & margin>=0.1;
        ideal(use) = min(1,partec.orifcoef*cible^0.5 ...
                          .*margin(use).^(-0.25));
        diam = [TO.in_diam(:);1];       % 1 m = absence d'orifice
        [~,j] = min(abs(ideal(use).^(-4)-diam'.^(-4)),[],2);
        commercial(use) = diam(j);
    end


    % ===== J. Finalizing output =====

    % --- Table TN for nodes ---
    TN_out = TN;
    TN_out.orif_ideal = ones(height(TN),1);
    TN_out.orif_com   = ones(height(TN),1);
    
    % Match TA.succ and TN.noeud to create variables for orifices
    [found,bloc] = ismember(TA.succ,TN.noeud);
    assert(all(found), 'NW5:Nodes', ...
        'Un noeud successeur est absent de TN.');
    TN_out.orif_ideal(bloc) = ideal;
    TN_out.orif_com(bloc)   = commercial;
    TN_out = TN_out(:, ...
        {'noeud','haut','xdata','ydata','orif_ideal','orif_com','statut'});

    % --- Tables TA_out and TCP for pipes ---
    TA_out = [Teq(equipped,:); Tre(reinforced,:)];
    % Complement (not present in NW4)
    TCP = TA_out(:,{'pred','succ','nature','diam1','diam2'});
    TCP.nature(TCP.nature=="equip") = "old";
    TCP.nature(TCP.nature=="reinf") = "new";

    % ---- Reinforcement costs ----
    reinfcost = struct();
    reinfcost.costOptim = costPipesOptim;
    reinfcost.costInter = costPipesInter;
    reinfcost.costFinal = costPipesFinal;
    reinfcost.costFixed = costFixed;



    % ---- Extra information collected in structure ctx ===
    if ismember('degree',TN.Properties.VariableNames)
        [fp,ip] = ismember(string(TA_out.pred),string(TN.noeud));
        [fs,is] = ismember(string(TA_out.succ),string(TN.noeud));
        assert(all(fp & fs), 'NW5:Nodes', ...
            'Une extremite de conduite est absente de TN.');
        TN.degree = accumarray([ip;is],1,[height(TN),1]);
    end
    assert(height(TA_out)==nbA+nnz(doubled), 'NW5:Output', ...
        'Nombre de conduites physiques inattendu.');
    assert(all(abs(TA_out.lon1 + TA_out.lon2 - TA_out.lon) <= ...
               1e-8*max(1,TA_out.lon)), 'NW5:Output', ...
        'La somme des longueurs ne couvre pas la liaison.');
    valid1 = (TA_out.lon1==0 & TA_out.ID1=="fictif") | ...
             (TA_out.lon1>0  & TA_out.ID1~="fictif");
    valid2 = (TA_out.lon2==0 & TA_out.ID2=="fictif") | ...
             (TA_out.lon2>0  & TA_out.ID2~="fictif");
    assert(all(valid1 & valid2), 'NW5:Output', ...
        'Longueurs et ID des troncons incoherents.');

    ctx.nbsplit = 0;
    ctx.split = false;
    % costFixed = 0;
    % if ismember('fixedCost',TA_out.Properties.VariableNames)
    %     costFixed = sum(model.f(nbA*nbD+(1:nbA)).*double(reinforced));
    % end
    ctx.reinforcement = struct( ...
        'reducedMixtureArcs',reducedArcs(:), ...
        'costPipesAfterReduction',costPipesAfterReduction, ...
        'costPipesBeforeRounding',costPipesInter, ...
        'costPipesAfterRounding',costPipesFinal, ...
        'fixedCost',costFixed, ...
        'requiresSimulationGraphRebuild',true);

    if isfield(sol,'fval'), ctx.reinforcement.costLP = sol.fval; end
    if isfield(sol,'slack') && numel(sol.slack)==nbA
        ctx.reinforcement.slackLP = sol.slack(:);
        ctx.reinforcement.marginChange = margin-sol.slack(:);
    end
end

% -------- Local functions --------

function r = parallelResistance(rOld,rAdded,lambda)
% Resistance lineique equivalente ; inf signifie conduite absente.
    cOld = zeros(size(rOld));
    cNew = zeros(size(rAdded));
    okOld = isfinite(rOld);
    okNew = isfinite(rAdded);
    cOld(okOld) = rOld(okOld).^(-1/lambda);
    cNew(okNew) = rAdded(okNew).^(-1/lambda);
    assert(all(cOld+cNew>0), 'NW5:DisconnectedArc', ...
        'Liaison sans conduite physique.');
    r = (cOld+cNew).^(-lambda);
end

function [idx,len] = fitCommercialLengths(idx,len,L,pipelength)
% idx est classe par diametre DECROISSANT. Toujours arrondir vers le haut
% la longueur du plus gros tuyau ; completer avec l'autre diametre.
    assert(numel(idx)==numel(len) && numel(idx)>=1 && numel(idx)<=2, ...
        'NW5:Solution','Un ou deux tuyaux attendus par renforcement.');
    if numel(idx)==2 && pipelength>0
        tolLen = 1e-10*max(1,L);
        nUnits = max(0,ceil((len(1)-tolLen)/pipelength));
        len(1) = min(L,nUnits*pipelength);
        len(2) = L-len(1);
    else
        len = L;
    end
    keep = len>1e-10*max(1,L);
    idx = idx(keep);
    len = len(keep);
    if numel(idx)==1, len = L; end
end

function f = loadFactorForExport(Qm3,share,pardsg)
% Champ transitoire requis par data4DsgFile ; sans role hydraulique ici.
    if isfield(pardsg,'targetflow') && pardsg.targetflow>0
        f = Qm3*1000*share/pardsg.targetflow;
    else
        f = 0;
    end
end

function status = integrity(TA,TN,TD,TC,TO,TL,dat,pardsg,partec,model,sol,ctx) %#ok<INUSD>
% Controles d'integrite regroupes ; nettoyages numeriques dans solveModel.
    assert(isnumeric(pardsg.secu) && isscalar(pardsg.secu) && ...
       isreal(pardsg.secu) && isfinite(pardsg.secu) && ...
       pardsg.secu >= 1, ...
       'NW5:Parameters', ...
       'pardsg.secu must be a finite real scalar greater than or equal to 1.');
    n = height(TA);
    assert(n>0 && istable(TD) && istable(TL), 'NW5:Input', ...
        'TA, TD et TL doivent etre des tables non vides.');
    assert(isfield(sol,'x_mat') && isfield(sol,'y'), 'NW5:Solution', ...
        'La solution doit contenir x_mat et y.');
    if isfield(sol,'ok')
        assert(sol.ok,'NW5:Solution','Le solveur n''a pas certifie la solution.');
    end
    assert(isfield(dat,'M_path') && isequal(size(dat.M_path),[n n]), ...
        'NW5:Paths','dat.M_path ne correspond pas aux arcs initiaux.');
    assert(all(ismember({'lon','lon1','lon2','ID1','ID2','statut', ...
                         'haut','pred','succ'}, TA.Properties.VariableNames)), ...
        'NW5:Input','Description de TA incomplete.');
    assert(all(ismember({'ID','diam','cost','resist'},TD.Properties.VariableNames)), ...
        'NW5:Catalogue','TD doit contenir ID, diam, cost, resist.');
    ids = string(TD.ID);
    actual = ids~="fictif";
    assert(nnz(~actual)<=1 && any(actual), 'NW5:Catalogue', ...
        'Un seul tuyau fictif au maximum.');
    assert(all(isfinite(TD.diam(actual)) & TD.diam(actual)>0) && ...
           numel(unique(ids(actual)))==nnz(actual), 'NW5:Catalogue', ...
        'Catalogue des tuyaux reels invalide.');
    X = full(sol.x_mat);
    y = full(sol.y(:));
    assert(isequal(size(X),[nnz(actual),n]) && numel(y)==n, ...
        'NW5:Solution','Dimensions incorrectes de sol.x_mat ou sol.y.');
    tol = 1e-6;
    assert(all(isfinite(X(:))) && all(X(:)>=-tol) && ...
           all(isfinite(y)) && all(abs(y-round(y))<=tol) && ...
           all(y>=-tol & y<=1+tol), 'NW5:Solution', ...
        'Precision insuffisante dans les variables de decision.');
    assert(all(abs(sum(X,1)'-round(y))<=tol), 'NW5:Solution', ...
        'La somme des x doit etre egale a y.');
    assert(all(isfinite(TA.lon) & TA.lon>0 & ...
               isfinite(TA.lon1) & TA.lon1>=0 & ...
               isfinite(TA.lon2) & TA.lon2>=0), 'NW5:Length', ...
        'Longueurs invalides.');
    equipped = TA.lon1>0 | TA.lon2>0;
    assert(all(abs(TA.lon1(equipped)+TA.lon2(equipped)-TA.lon(equipped)) ...
               <= 1e-9*max(1,TA.lon(equipped))), 'NW5:Length', ...
        'Longueurs existantes incompatibles avec TA.lon.');
    assert(all(equipped | round(y)>0), 'NW5:DisconnectedArc', ...
        'Un arc nu doit etre renforce.');
    if ismember('N',TA.Properties.VariableNames)
        assert(~any(round(y)>0 & string(TA.N)=="B"), 'NW5:BlockedArc', ...
            'Un arc bloque est renforce.');
    end
    ids1 = string(TA.ID1); ids2 = string(TA.ID2);
    assert(all(ismember(ids1,ids)) && all(ismember(ids2,ids)), ...
        'NW5:UnknownPipe','Un ID de tuyau est absent de TD.');
    assert(~any((TA.lon1>0 & ids1=="fictif") | ...
                (TA.lon2>0 & ids2=="fictif") | ...
                (TA.lon1==0 & ids1~="fictif") | ...
                (TA.lon2==0 & ids2~="fictif")), ...
        'NW5:Equipment','Longueurs et ID non coherents.');
    Q = table2array(TL(:,2:end));
    assert(size(Q,1)==n && ~isempty(Q) && ...
           all(isfinite(Q(:)) & Q(:)>=0), 'NW5:Loads', ...
        'TL doit contenir des debits finis et positifs.');
    if pardsg.uniform_dem==1 && pardsg.bounded_flow==0
        assert(ismember('in_diam',TO.Properties.VariableNames) && ...
               all(isfinite(TO.in_diam) & TO.in_diam>0), ...
               'NW5:Orifices', 'Catalogue des orifices invalide.');
    end
    status = 'OK';
end

