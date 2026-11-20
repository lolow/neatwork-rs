function [TN, TA, TB] = validate_and_align_Tables(TN, TA, TB)
% Objectif: Nettoyer et valider TN/TA immédiatement après parseTpoToTable.
% Hypothèses de nommage: TN.noeud (categorical), TA.pred/TA.succ (categorical).
% Effets:
%   - retire catégories fantômes
%   - interdit nœuds inconnus
%   - vérifie unicité des nœuds TN
%   - vérifie égalité ensembliste entre nœuds(TA) et nœuds(TN)
%   - aligne les catégories de TA sur celles de TN
%   - avertit en présence de boucles sur un même nœud

    % 0) Assurer le type
    if ~iscategorical(TN.noeud), TN.noeud = categorical(TN.noeud); end
    if ~iscategorical(TA.pred),  TA.pred  = categorical(TA.pred);  end
    if ~iscategorical(TA.succ),  TA.succ  = categorical(TA.succ);  end
    if ~iscategorical(TB.borne), TB.borne = categorical(TB.borne);  end
    
    % 1) Enlever les niveaux fantômes PARTOUT (important avant les comparaisons)
    TN.noeud = removecats(TN.noeud);
    TA.pred  = removecats(TA.pred);
    TA.succ  = removecats(TA.succ);
    TB.borne  = removecats(TB.borne);

    % Les catégories seules masquent les identifiants répétés dans TN.
    nodeIds = categories(TN.noeud);
    duplicateIds = nodeIds(countcats(TN.noeud) > 1);
    if ~isempty(duplicateIds)
        error('NW:DuplicateNodeIds', 'Identifiants de nœuds répétés dans TN.noeud : {%s}', ...
            strjoin(string(duplicateIds), ', '));
    end
    
    % 2) Référentiel EXACT des nœuds (sans unique, qui réordonne)
    idsTN   = categories(TN.noeud);  % cellstr des niveaux utilisés côté TN
    idsPred = categories(TA.pred);
    idsSucc = categories(TA.succ);
    idsBorne = categories(TB.borne);
    
    % 3) Contrôles d'ensemble (double inclusion) avec messages clairs
    badPred = setdiff(idsPred, idsTN);
    badSucc = setdiff(idsSucc, idsTN);
    badBorneN = setdiff(idsBorne, idsTN);
    % Avant orientation, une borne peut figurer dans pred ou dans succ.
    % peel_orient_tree placera ensuite les feuilles hors source dans succ.
    badBorneA = setdiff(idsBorne, union(idsPred, idsSucc));
    if ~isempty(badPred) || ~isempty(badSucc)
        error("Noeuds inconnus dans TA: pred={%s} succ={%s}", ...
              strjoin(string(badPred),","), strjoin(string(badSucc),","));
    end
    missingInTA = setdiff(idsTN, union(idsPred,idsSucc));
    if ~isempty(missingInTA)
        error("Noeuds présents dans TN mais absents de TA: {%s}", ...
              strjoin(string(missingInTA),","));
    end
    
    if ~isempty(badBorneA) || ~isempty(badBorneN)
        error("Bornes de TB absentes de TN={%s} ou de TA (pred/succ)={%s}", ...
            strjoin(string(badBorneN),","), strjoin(string(badBorneA),","));
    end
    
    % 4) Alignement d'ORDRE seulement (ensembles déjà identiques)
    orderPred = idsTN(ismember(idsTN, idsPred)); % permutation de idsPred
    TA.pred = reordercats(TA.pred, orderPred);
    orderSucc = idsTN(ismember(idsTN, idsSucc)); % permutation de idsSucc
    TA.succ = reordercats(TA.succ, orderSucc);

    % 5) Avertir en présence de boucles sur un même nœud
    hasSelf = any(TA.pred == TA.succ);
    if hasSelf
        i = find(TA.pred == TA.succ, 1);
        warning('Arc boucle (self-loop) détecté sur %s (ligne %d).', string(TA.pred(i)), i);
    end
end
