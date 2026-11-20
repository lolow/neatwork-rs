function [TA, TN, TD, param] = completeTables(TA, TN, TD, param)
% Entrée : TA(pred,succ,lon1,lon2,ID1,ID2,...)  TN(noeud,haut,...)  TD(ID,Diameter,Cost)
% Sortie : TA enrichie (haut_succ, diam1/2, cost1/2, totcost). Typages 
% minimaux. La structure param est allégée (modérément)

    
    %  ==== 1- Ajout de la hauteur du successeur ======
    % TA.haut_succ = NaN(height(TA),1);
    % [tf, loc] = ismember(TA.succ, TN.noeud);
    % TA.haut_succ(tf) = TN.haut(loc(tf));

    TA.gravit = zeros(height(TA),1);
    source = TN.statut == 0;
    source_haut = TN.haut(source);
    [tf, loc] = ismember(TA.pred, TN.noeud(source));
    TA.gravit(tf) =  -source_haut(loc(tf));
    borne = TN.statut == 2;
    borne_haut = TN.haut(borne);
    [tf, loc] = ismember(TA.succ, TN.noeud(borne));
    TA.gravit(tf) = borne_haut(loc(tf));


    
    % ===== 2- Mappage Diameter/Cost pour ID1/ID2 si présents ======
    % TD doit fournir: TD.Diameter (double), TD.Cost (double), + (facultatif) TD.ID
    needTD = all(ismember({'diam','cost'}, TD.Properties.VariableNames));
    assert(needTD, 'TD doit contenir les colonnes ''diam'' et ''cost''.');
    
    % Harmoniser types côté TD
    if iscell(TD.ID), TD.ID = string(TD.ID); end
    if iscategorical(TD.ID), TD.ID = string(TD.ID); end
    
    %  Assurer TA.diam (bloquant si impossible)
    hasID1   = any(strcmp('ID1',   TA.Properties.VariableNames));
    hasDiam1 = any(strcmp('diam1', TA.Properties.VariableNames));
    hasID2   = any(strcmp('ID2',   TA.Properties.VariableNames));
    hasDiam2 = any(strcmp('diam2', TA.Properties.VariableNames));
    hasID = hasID1 & hasID2;
    hasDiam = hasDiam1 & hasDiam2;
    
    if ~hasDiam
        if hasID
            % ID -> Diameter via TD (join « left », ordre de TA préservé)
            [TA.diam1, TA.cost1] = map_diam_cost(TA.ID1, TA.lon1, TD, 10, 0);
            mask = TA.lon2 > 0;
            TA.cost2 = zeros(height(TA),1);
            TA.diam2 = zeros(height(TA),1);
            [TA.diam2, TA.cost2] = map_diam_cost(TA.ID2, TA.lon2, TD, 10, 0);
        else
            error(['Diamètres manquants: ni TA.diam présent, ni TA.ID permettant ' ...
                'de le reconstituer. Impossible de simuler les débits.']);
        end
    end
    
    % Vérifier qu'on a bien obtenu des diamètres utilisables
    if any(~isfinite([TA.diam1;TA.diam2]))
        error(['Diamètres manquants/invalides après normalisation. ' ...
            'Vérifier la cohérence TA.ID ↔ TD.ID/TD.diam.']);
    end
    
    % === 3- Coefficient beta par tronçon ===
    % S'assurer que la décomposition de chaque segment est compatible avec
    % sa longueur initiale
    lon = TA.lon1 + TA.lon2;
    tol_length = 1e-12;
    fautif = abs(lon - TA.lon) > tol_length * max(1, abs(TA.lon));
    if any(fautif)
       error(sprintf('L''arc ayant pour successeur ''%s'' n''a pas la bonne découpe\n',TA.succ(fautif))) 
    end
 
    TA.beta = zeros(height(TA),1);
    idlon1 = (TA.lon1 > 0);
    TA.beta(idlon1) = param.bescal * TA.lon1(idlon1) ./ TA.diam1(idlon1) .^param.nu;
    idlon2 = (TA.lon2 > 0);
    TA.beta(idlon2) = TA.beta(idlon2) + param.bescal * TA.lon2(idlon2) ...
        ./ TA.diam2(idlon2) .^param.nu;

    % === 4- Assurer TA.cost si absent (fallback: coût minimal par diamètre) ===
    hasCost=any(ismember({'cost1','cost2'}, TA.Properties.VariableNames));
    if ~hasCost
        % Construire un mapping Diameter -> min(Cost)
        % (si plusieurs lignes partagent le même diamètre, on prend le coût min)
        if max(groupcounts(TD.diam)) > 1
            disp('At least one diameter appears twice. The least cost is retained.')
        end
        [uD,~,g] = unique(TD.diam);
        minCost  = accumarray(g, TD.cost, [], @min);
        Map = containers.Map(num2cell(uD), num2cell(minCost));
    
        TA.cost1 = arrayfun(@(d) Map(d), TA.diam1);  % coût "le plus bas" par Ø
        TA.cost2 = zeros(height(TA),1);
        TA.cost2(TA.lon2>0) = arrayfun(@(d) Map(d), TA.diam2(TA.lon2>0));  % coût "le plus bas" par Ø
    end
    
    % --- Coût total par tronçon ---
    ac1 = ismember('cost1', TA.Properties.VariableNames);
    ac2 = ismember('cost2', TA.Properties.VariableNames);
    if ac1 && ac2
        TA.totcost = TA.lon1 .* TA.cost1 + TA.lon2 .* TA.cost2;
    end
    
    
    
    % ===== 5- Placer les bornes et les sources. Statut des arcs ======
    TA.statut = ones(height(TA),1);
    idter = (TN.statut == 2);
    [idpred, ~] = ismember(TA.pred,TN.noeud(idter));
    [idsucc, ~] = ismember(TA.succ,TN.noeud(idter));
    if any(idpred)
        error('Il y a des bornes parmi les prédécesseurs');
    else
        TA.statut(idsucc) = 2;
    end
    idsrc =  (TN.statut == 0);
    [idsucc, ~] = ismember(TA.succ,TN.noeud(idsrc));
    [idpred, ~] = ismember(TA.pred,TN.noeud(idsrc));
    if any(idsucc)
        error('Il y a des sources parmi les successeurs');
    else
        TA.statut(idpred) = 0;
    end
    
    
    % ==== 6- Calculer les paramètres alpha ====
    id_orifid   = ismember('orif_ideal',TN.Properties.VariableNames);
    id_orifcom  = ismember('orif_com',TN.Properties.VariableNames);
    id_ecart    = ismember('ecart',TA.Properties.VariableNames);
    if param.faucetcoef > 0
        alpha_faucet = 1 / param.faucetcoef ;
    else
        alpha_faucet = 0;
    end

    [~,loc] = ismember(TA.succ,TN.noeud);

    TA.alpha_ideal = zeros(height(TA),1);
    TA.alpha_com = zeros(height(TA),1);
    idBorne = (TA.statut == 2);
    if id_orifid
        cible = param.targetflow / 1000;
        TA.orif_ideal = TN.orif_ideal(loc) ;
        TA.alpha_ideal(idBorne) = (param.orifcoef./ TA.orif_ideal(idBorne)) .^4;
        TA.alpha_ideal(idBorne) = TA.alpha_ideal(idBorne) + alpha_faucet;
    end
    if id_orifcom
        TA.orif_com = TN.orif_com(loc) ;
        TA.alpha_com(idBorne) = (param.orifcoef./ TA.orif_com(idBorne)) .^4;
        TA.alpha_com(idBorne) = TA.alpha_com(idBorne) + alpha_faucet;
    end

    if id_ecart
        TA.alpha = TA.ecart / cible^2;
    else
        TA.alpha = TA.alpha_ideal;
    end

    % ====== Clean parameter fields ======
    keep = {'targetflow','opentaps','pressure','bounded_flow','uniform_dem','seuil','nu','lambda','faucetcoef'};
    paramS =  subset_struct(param,keep) ;
    
    
    % ======= Trier les tables pour avoir le bon ordre ======
    TA = sortrows(TA,{'statut','succ'}) ;
    TN = sortrows(TN,{'statut','noeud'}) ;
    end
    
    
    % =======================================================
    function [diam, cost] = map_diam_cost(IDcol, lenCol, TD, defaultDiam, defaultCost)
    % Remplit diam/cost à partir de TD(ID→Diameter/Cost), uniquement là où lenCol>0
	    n = numel(IDcol);
	    diam = defaultDiam * ones(n,1);
	    cost = defaultCost * ones(n,1);
    
	    if ~ismember('diam', TD.Properties.VariableNames), return; end
	    if ~ismember('cost', TD.Properties.VariableNames), return; end
    
	    idx = (lenCol > 0);
	    if any(idx)
	        [tf, loc] = ismember(IDcol(idx),TD.ID);
	        % ids = categorical(IDcol(idx));
	        % [tf, loc] = ismember(ids, categorical(TD.ID));
	        % valeurs connues
	        diam(idx) = defaultDiam;
			cost(idx) = defaultCost;
	        diam(idx) = replace_at(diam(idx), tf, TD.diam(loc(tf)));
			cost(idx) = replace_at(cost(idx), tf, TD.cost(loc(tf)));
	    end
    end
    
    function v = replace_at(v, mask, vals)
    % remplace v(mask) par vals en gardant la taille
	    tmp = v;
	    tmp(mask) = vals;
	    v = tmp;
    end