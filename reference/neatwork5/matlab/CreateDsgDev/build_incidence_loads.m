function [TA, TL, dat] = build_incidence_loads(TA, TN, TC, pardsg,dat)
% BUILD_INCIDENCE_LOADS
% Computes estimated loads on arcs using the NW4 heuristic based on
% probabilistic demand aggregation. See Clement for details.
%
% PRECONDITION
% ------------
% dat.M_incid and dat.M_path must already exist and must have been
% constructed by build_graph_artifacts() after topology processing.
%
% Calcule TA.loadFactor à partir d'un arbre (pred->succ).
% - Entrées :
%     TA(pred, succ, lon, ...)   (categorical pour pred/succ)
%     TN(noeud, haut, nbrob, ...) (categorical pour noeud)
%     pardsg : used to compute load factors
% - Output :
%     TA (ajout du champ : loadFactor)
%     TL Donne pour chaque borne les charges sur les nœuds visités sur le
%     chemin jusqu'à la source. Renvoie zero si le nœud ne fait pas partie
%     du chemin.
%     dat.idxPred, dat.idxSucc, dat.parentArc (indices utiles)
%
% METHODS
% ------------
% BINOMIAL LAW: If all users are identical, the load factors are computed using a binomial law. 
% The binomial law is relevant to the design when par dsg.uniform_dem == 1, which means that all 
% users have the same attributes (demand, probability). The load is computed conditionnally on the % existence of at least one open faucet at the terminal node. 
% ------------
% MONTE CARLO SAMPLING: If users have different attributes, the load factors are computed using
% Monte Carlo sampling. 
%
%  ENHANCEMENTS (Not implemented yet)
% ------------
% - The function could use approximation laws (e.g., normal approximation) to speed up the 
% computation of load factors for large numbers of users. See Clement for details.
% - If param.dsg.servicequal is close to 1, the Monte Carlo sampling should rely on importance 
% sampling to reduce the number of scenarios needed to estimate the load factors accurately.This is almost mandatory when the loads are computed to control speed in the pipes.


assert(isfield(dat,'M_incid') && isfield(dat,'M_path'), ...
    'Graph matrices must be computed by build_graph_artifacts before calling build_incidence_loads.');

% Check class of variables 'noeud', 'pred', 'succ'
% assert(iscategorical(TN.noeud), 'TN.noeud must categorical (see parseTpoToTables).');
% assert(iscategorical(TA.pred)  && iscategorical(TA.succ), ...
%        'TA.pred/TA.succ must be categorical (see parseTpoToTables).');


    % ========== A- Record incidence and path matrices ===========
    
    nbArcs = height(TA);

    assert(isequal(size(dat.M_incid), [height(TN)-1, nbArcs]) && ...
           isequal(size(dat.M_path), [nbArcs, nbArcs]), ...
        'Graph matrix dimensions do not match TA and TN.');

    % Group customers by terminal.
    % borneU defines the column order used in TL.
    [G, borneU] = findgroups(TC.borne);
    nB = numel(borneU);

    % Match each customer terminal to its incoming arc.
    [found, terminalArcIdx] = ismember( ...
        string(borneU), string(TA.succ));

    if ~all(found)
        error('Some customer terminals have no incoming arc in TA.');
    end

    if ~all(TA.statut(terminalArcIdx) == 2)
        error('Some customer terminals correspond to non-terminal arcs.');
    end

    % Columns follow borneU order, independently of TA row order.
    Path2Term = dat.M_path(:, terminalArcIdx);
    
    % ========= B Compute load factors ========

    % If the demand at the terminal nodes is to be satisfied, the 
    % stochastic demand induces a stochastic load on the arcs of the tree. 
    % When all users are the same, the distribution of flows on the path 
    % from the source to the terminal node follows a binomial law for each 
    % arc. This law is relevant to the design conditionally on the 
    % existence of at least one open faucet at the terminal node. When 
    % users have different attributes (demand, probability), explicit 
    % calculations are no longer possible. The distributions are evaluated 
    % using Monte Carlo sampling.    
    % Optional pardsg.use_cvar = 1 replaces the interpolated VaR by CVaR:
    % empirical for Monte Carlo, exact for the binomial law.
    % Default: 0 (historical VaR), at pardsg.servicequal in both cases.

    useCvar = false;
    if isfield(pardsg,'use_cvar')
        assert(isscalar(pardsg.use_cvar) && ...
               ismember(pardsg.use_cvar,[0 1]), ...
            'NW5:LoadRiskMeasure','use_cvar must be 0 or 1.');
        useCvar = logical(pardsg.use_cvar);
    end
    qos = pardsg.servicequal; % Fraction in (0,1]
    assert(isscalar(qos) && isfinite(qos) && qos>0 && qos<=1, ...
        'NW5:LoadQuantile','servicequal must be in (0,1].');

    if pardsg.uniform_dem == 0
        % --- 1- Simulate client's demand ----
        if ~isfield(pardsg,'nScenLoad')
            pardsg.nScenLoad = 20000;
        end
        assert(isreal(TC.proba) && ...
            all(isfinite(TC.proba) & TC.proba >= 0 & TC.proba <= 1, 'all'), ...
            'NW5:Probabilities', ...
            'Probabilities in TC.proba must be finite real values between 0 and 1.');

        assert(isreal(TC.cible) && ...
            all(isfinite(TC.cible) & TC.cible >= 0, 'all'), ...
            'NW5:Demands', ...
            'Demands in TC.cible must be finite real nonnegative values.');
            
        nS = pardsg.nScenLoad;
        nC = height(TC) ;

        % Optional: for reproducible tests
        % rng(2,'twister'); % Uncomment for reproducible scenarios
        % Here, scenarios(i,j) = demand of client i in scenario j
        scenarios = (rand(nC,nS) < TC.proba) .* TC.cible;

        % --- 2- Total demand at terminal ----
        B2C= sparse(G, (1:nC)', 1, nB, nC);   % construit une matrice borne×client
        % demande cumulée à chaque borne
        demande_borne = B2C * scenarios;
        

        % --- 3- Flowload in the whole tree ----
        % Demand is indexed by the incoming arc of each terminal.
        demande_noeud = zeros(nbArcs, nS);
        demande_noeud(terminalArcIdx, :) = demande_borne;
        
        load_flow = dat.M_path * demande_noeud ;

        % --- 4- Define submatrices of path and flows  ----
        PathIdx = arrayfun(@(k) find(Path2Term(:,k)), 1:nB, 'UniformOutput', false);
        ScenIdx = arrayfun(@(k) find(demande_borne(k,:)>0), 1:nB, 'UniformOutput', false);

        % ---- 4- Statistics on the nodal demands ----
        moy_cond    = zeros(nbArcs, nB);
        q_cond      = zeros(nbArcs, nB);
        % q_cond1     = zeros(nbArcs, nB);
        % q_cond2     = zeros(nbArcs, nB);

        for k = 1:nB
            % Select scenarios with positive demand at terminal node k and
            % collect flows on the path from source to node k
            I = PathIdx{k};      % arcs du chemin k
            J = ScenIdx{k};      % scénarios actifs pour k
            Xk = load_flow(I,J); % sous-matrice compacte
            [r, m] = size(Xk);
            assert(m>0, 'NW5:NoActiveLoadScenarios', ...
                'Terminal %s has no active scenarios; check demand or increase nScenLoad.', ...
                string(borneU(k)));
            % Compute percentile and the value just below
            % p/100*m est le nombre moyen d'éléments en prendre en compte.
            % Les percentiles sont associés au deux arrondis de ce nombre,
            % supérieur et inférieur.
            Y = sort(Xk,2);
            idx1 = ceil(qos * m); % L'arrondi est le numémax(Y,[],repmat(qc1,1,m),2);ro d'odre dans Y(k,:)
            qc1 = Y(:,idx1); 
            if useCvar
                % Exact empirical tail mean, including fractional mass at
                % the quantile when demands are discrete or tied.
                if qos==1
                    q_cond(I,k) = Y(:,end); % CVaR limit: sampled maximum
                else
                    q_cond(I,k) = qc1 + ...
                        sum(max(Y-qc1,0),2)/((1-qos)*m);
                end
                continue
            end
            lt = sum(Y < qc1, 2);
            le = sum(Y <= qc1, 2);
            pc2 = lt/m;
            pc1 = le/m;
            qc2 = zeros(size(I));
            pos = lt > 0; 
            for i = 1:length(I)
                if lt(i) == 0
                    qc2(i) = qc1(i);
                else
                    qc2(i) = Y(i,lt(i));
                end
            end
                
            % Combiner qc1 et qc2 en fonction en proortion des
            % probabilités et de l'objectif param.servicequal
            theta = ones(size(pc1));
            qc3 = qc1;

            % Trouver les indices 
            idx3 = (pc1 > pc2) & (pc2 > 0);
            theta(idx3) = (qos - pc2(idx3)) ./ (pc1(idx3) - pc2(idx3));
            qc3(idx3)  = theta(idx3) .* qc1(idx3) + (1-theta(idx3)) .* qc2(idx3);

            % Reporter le résultat dans q_cond
            q_cond(I,k) = qc3;
          
        end
        TL = array2table(q_cond, 'VariableNames', string(borneU(:)).') ;
    end

    if pardsg.uniform_dem == 1
        % Clients have identical attributes (NeatWork4)

        % --- Nombre de dépendants par arc ---
        nbdep  = dat.M_path * TA.nbrob;
        % idTerm = (TA.statut == 2);
        % nbdep2 = full(sum(dat.M_path(:,idTerm), 2));
        TA.nbdep = nbdep;       
        qos = pardsg.servicequal;
        proba = pardsg.opentaps;
        assert(isscalar(proba) && isreal(proba) && isfinite(proba) && ...
               proba>0 && proba<=1, ...
            'NW5:LoadProbability', ...
            ['opentaps must be in (0,1]. With opentaps=0, no client ever opens; ' ...
             'there is no demand to size.']);

        % sous-ensemble pertinent
        idx1 = nbdep > 1;
        nbdep1 = nbdep(idx1) -1;

        % One faucet is known open; the remaining Z ~ Bin(nbdep-1, proba).
        if proba==1
            ld = 1+nbdep1; % All dependent clients are open, for VaR and CVaR.
        elseif useCvar
            if qos==1
                ld = 1+nbdep1; % Maximum supported load, not a sampled maximum
            else
                k = binoinv(qos,nbdep1,proba);
                % E[(Z-k)+] = n*p*P(Bin(n-1,p)>=k) - k*P(Z>k).
                % Upper tails avoid cancellation in 1-binocdf(...).
                excess = nbdep1.*proba.*binocdf(k-1,nbdep1-1,proba,'upper') ...
                    - k.*binocdf(k,nbdep1,proba,'upper');
                ld = 1+k+max(excess,0)/(1-qos);
            end
        else
            % Encadrement sur le sous-ensemble
            k  = binoinv(qos, nbdep1, proba);
            p2 = binocdf(k,   nbdep1, proba);
            p1 = binocdf(k-1, nbdep1, proba);

            % Approximation linéaire entre k et k-1. 
            % On part de k-1 et on ajoute l'écart entre qos et p1 relativement
            % à l'écart entre p2 et p1. Ces mesures se font pour un nombre de 
            % tirages nbdep - 1, qui est le nombre de robinets dépendants en
            % sus du robinet qu'on sait ouvert. La charge ld doit réintroduire
            % le robinet au'on sait ouvert. On aura k -1 + 1 = k pour la
            % charge.
            % Si k = 0, alors p1 = proba(X ≤ k-1<0) = 0; Donc p1>0 => k>=1.
            idx_aprox = (p2 > p1) & (p1 > 0) ;
            ld = ones(size(nbdep1));
            ld(idx_aprox) = k(idx_aprox)  + (qos - p1(idx_aprox)) ./ (p2(idx_aprox) - p1(idx_aprox));
            idx = (p2 == p1) & p1 > 0;  % indices du cas particulier
            ld(idx) = k(idx);  % on prend directement k
        end
        % Reporter sur les coordonnées pertinentes (idx1)
        loadFactor = ones(height(TA),1);
        loadFactor(idx1) = ld ;
        
        % Convert loadFactor in flows. Une borne de demande nulle n'est pas
        % supprimée automatiquement du réseau par le module de design. Elle 
        % reçoit un équipement minimal. Sa suppression éventuelle relève 
        % d'une décision explicite du concepteur.
        flow_eps = 1e-12;   
        flowFactor = max(loadFactor * pardsg.targetflow, flow_eps);

        % Report flowFactors on each path to terminal
        lf = Path2Term .* flowFactor ;
        TL = array2table(lf, 'VariableNames', string(borneU(:)).') ;
    end 
    TL.ID = string(TA.succ);
    TL = movevars(TL,'ID','Before',1);
end
