function [dat,options] = complete_data(TA,TN, dat, param)
% Calcule la matrice d'oncidence si elle n'est pas présente dans le champ de
% la structure dat. Définit la structure des paramètres par défaut de
% l'algorithme IPM. Enrichit dat de champs vecteurs indicateurs des
% positions des noeuds et des arcs

    % ---- incidence matrix ------
    [M_incid, nodes_rows] = incidence_intermediaire(TA, TN) ;
    dat.M_incid = M_incid ;

    % ---- Sauvegarder le coût total d'investissement ----
    dat.totcost = sum(TA.totcost) ;

    
    %  --- Options pour le code IPM ---
    options.tolx = 1e-8 ; % Soit 1 millilitre par seconde
    options.tolpress = 1e-6 ; % Soit la pression de 1mm d'eau (1e-4 bar)
    options.tolpseudoA = 1e-10 ; % Tolerance de déclaration pseudo actif
    options.maxiter = 30 ; % Nombre maximum d'itérations pour la méthode de Newt
    options.tolcent = 1e-10 ; % paramètre pour la régularisation en y
    options.sigma   = 0.1;        % reduction factor for mu (kept as in your code)
    options.tau     = 2.5;        % exponent used in sigma_mu = min(1,(mu_aff/mu)^tau)*mu
    options.a_secur = 0.999;      % safeguard on final step

    
    % Générer les indicateurs
    % Noeuds Nter (terminaux), Ntra (transit), Nsrc (sources)
    dat.idNter = (TN.statut == 2);
    dat.setNter = TN.noeud(dat.idNter) ;
    dat.idNtra = (TN.statut == 1);
    dat.setNtra = TN.noeud(dat.idNtra) ;
    dat.idNsrc = (TN.statut == 0);
    dat.setNsrc = TN.noeud(dat.idNsrc) ;
    
    % Arcs Ater (terminaux), Atra (transit), Asrc (sources)
    dat.idAter = (TA.statut == 2) ;
    dat.setAter = TA.succ(dat.idAter) ;
    dat.idAtra = (TA.statut == 1) ;
    dat.setAtra = TA.succ(dat.idAtra) ;
    dat.idAsrc = (TA.statut == 0) ;
    dat.setAsrc = TA.succ(dat.idAsrc) ;
    dat.nbA = height(TA) ;
    
    % Dénombrement des diverses catégories.
    % Noeuds
    dat.nbNter = numel(dat.setNter) ;
    dat.nbNter = numel(dat.setNter) ;
    dat.nbNtra = numel(dat.setNtra) ;
    % Arcs
    dat.nbAter = numel(dat.setAter) ;
    dat.nbAtra = numel(dat.setAtra) ;
    dat.nbAsrc = numel(dat.setAsrc) ;
    
    % Composants de la fonction intégrale d'énergie
    % S'il y a plusieurs sources, elles sont 
    dat.gravit = TA.gravit ;
    % dat.gravit(dat.idAtra) = 0 ;
    % dat.gravit(dat.idAsrc) = TN.haut(dat.idNsrc) ;
    dat.beta = TA.beta;
    dat.alpha = TA.alpha;
    dat.alpha_ideal = TA.alpha_ideal;
    dat.alpha_com = TA.alpha_com;


end