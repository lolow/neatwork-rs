function [demande_borne,nbClients,simRes] = genscen(TC,dat,param)
%GENSCEN est un petit utilitaire pour construire des scénarios d'activité
% aux bornes.
% Input: table TC des clients et nS nombres de simulations
% Output: matrice de la demande aux bornes et matrice d'indicateurs
% d'ouverture (demande >0) des bornes.

    assert(all(TC.proba >= 0 & TC.proba <= 1), ...
        'Some probabilities are not in [0,1]');
    
    % ==== Mettre à jour les demandes uniformes après revue des paramètres ====
    if param.uniform_dem == 1 
        TC.proba = param.opentaps * ones(height(TC),1);
        TC.cible = param.targetflow * ones(height(TC),1);
    end
    % Tests pour se garantir contre le cas pathologique sans scénario
    assert(any(TC.proba>0),'All probabilities are zero');

    % ==== Créer la marice des scénarios =====
    % Un scénario est défini par la liste des demandes cumulées en chaque
    % borne. L'ensemble des scénarios est stocké sous forme de matrice.


    % borneU = matrice (nC,nS) des scenarios
    [G, borneU] = findgroups(TC.borne);
    nB = numel(borneU);
    nC = height(TC) ;
    nS = param.nbsim;
    ATC = sparse(G, (1:nC)', 1, nB, nC); % construit une matrice borne × client
    demande_borne  = zeros(nB,nS) ;
    nbClients = zeros(nB,nS);
    nSc = 0;
    ouvert = zeros(nC,nS) ;


    % Tirages aléatoires répétés pour garantir des scénarios avec au moins 
    % un client actif (demande > 0). 
    iter = 0;
    maxIter = 50;
    % probabilité qu'il y ait au moins un client actif dans un scénario
    p_any = 1 - prod(1 - TC.proba); 
    % Si p_any >= 0.5, chaque itération donnera en moyenne la moitié de ce
    % qui est requis. Cela devrait suffire. Si le réseau est testé en
    % période de charge p_any est proche de 1
    if p_any < 0.5
        warning('Very low expected number of valid scenarios in current draw.');
    end
    nSc = 0;
    iter = 0;
    maxIter = 20;   % normalement on n'en approchera jamais

    while nSc < nS
        iter = iter + 1;
        assert(iter <= maxIter, ...
            'Scenario generation exceeded maximum iterations.');

        nRemain = nS - nSc;

        % nombre de colonnes à tirer pour obtenir ~nRemain scénarios utiles
        nDraw = ceil(1.1 * nRemain / p_any);   % facteur de sécurité 10%
        nDraw = max(nDraw, nRemain);           % au minimum nRemain

        ouv = (rand(nC, nDraw) < TC.proba);
        keep = any(ouv,1);
        ouv  = ouv(:,keep);

        nbKeep = size(ouv,2);
        nbTake = min(nbKeep, nRemain);

        if nbTake > 0
            ajout = nSc + (1:nbTake);
            ouvert(:,ajout) = ouv(:,1:nbTake);
            nSc = nSc + nbTake;
        end
    end

    scenarios = ouvert .* TC.cible ;
    demande_borne = ATC * scenarios;
    nbClients  = ATC * ouvert;

    borne_ouverte = demande_borne > 0;

    % --- Structure d'archivage des résultats de la simulation ---
    simRes = struct();
    simRes.X = zeros(dat.nbA,param.nbsim) ; % débits
    simRes.Satis = zeros(dat.nbAter,param.nbsim) ; % satisfaction de clients dpp
    simRes.Ouv = false(dat.nbA,param.nbsim) ; % robinets ouverts (0 ou 1)
    simRes.Charge = zeros(dat.nbA,param.nbsim) ; % pertes de charge sur chaque arc
    simRes.Alpha = zeros(dat.nbAter,param.nbsim) ; % Friction usager (pour le calcul de la pression aux bornes)
    simRes.Iter = zeros(param.nbsim,1) ;
    simRes.U = zeros(dat.nbNtra,param.nbsim) ;
    simRes.dem_tot = zeros(1,param.nbsim) ;

end
