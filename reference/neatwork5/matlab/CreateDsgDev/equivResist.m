function [TR] = equivResist(TA,TN,TD,partec)
%EQUIVRESIST This function calculates the equivalent resistance of link 
% equipped with a reinforcement pipe in parallel. The equivalent resistance is 
% calculated for each candidate reinforcement pipe in TD, and the result is 
% returned as a table TR with the same number of rows as TA and the same 
% number of columns as TD (one column per candidate pipe).
% The formula used is:
%   R_equiv = (C_exist + C_candidate)^(-lambda)
% where C is the conductance C = R^(-1/lambda). The concept of conductance in 
% electricity is used here with the paramter 1 < lambda < 2 instead of 
% lambda = 1. The parameter lambda is provided in the input struct partec.


    arguments (Input)
        TA
        TN
        TD
        partec
    end
    
    arguments (Output)
        TR
    end
    
    % ==== 1- Vérification d'intégrité ====
    % TD doit fournir: TD.Diameter (double)
    needTD = all(ismember({'diam'}, TD.Properties.VariableNames));
    assert(needTD, 'TD doit contenir la colonne ''diam''.');
    % TA doit fournir les colonnes d'identifiants des tuyaux
    hasID1   = any(strcmp('ID1',   TA.Properties.VariableNames));
    hasID2   = any(strcmp('ID2',   TA.Properties.VariableNames));
    hasID = hasID1 & hasID2;
    assert(hasID, 'Identifications manquantes dans ID1 et/ou ID2')
    % Les longueurs des deux sous-segments de l'arc doivent être égales à
    % la longueur totale
    equipped = TA.lon1 > 0 | TA.lon2 > 0;
    ecart = TA.lon1(equipped)+ TA.lon2(equipped) - TA.lon(equipped);
    assert(all(abs(ecart) > 1e-7), ...
    'NW5:EquipmentLength', ...
    'Pour tout arc equipé, lon1 + lon2 doit être égal a lon.');


    % isLon = TA.lon1 + TA.lon2 == TA.lon ;
    % assert(all(isLon),'Split length don''t sum to total length');


    % ===== 2- Mappage Diameter pour ID1/ID2  ======
    % Harmoniser types côté TD
    if iscell(TD.ID), TD.ID = string(TD.ID); end
    if iscategorical(TD.ID), TD.ID = string(TD.ID); end
    
    %  Assurer TA.diam (bloquant si impossible)
    hasDiam1 = any(strcmp('diam1', TA.Properties.VariableNames));
    hasDiam2 = any(strcmp('diam2', TA.Properties.VariableNames));
    hasDiam = hasDiam1 & hasDiam2;

    if ~hasDiam
        % ID -> Diameter via TD (join « left », ordre de TA préservé)
        TA.diam1 = zeros(height(TA),1);
        TA.diam1 = map_diam(TA.ID1, TA.lon1, TD, 0);
        TA.diam2 = zeros(height(TA),1);
        TA.diam2 = map_diam(TA.ID2, TA.lon2, TD, 0);
    end
    
    % Vérifier qu'on a bien obtenu des diamètres utilisables
    if any(~isfinite([TA.diam1;TA.diam2]))
        error(['Diamètres manquants/invalides après normalisation. ' ...
            'Vérifier la cohérence TA.ID ↔ TD.ID/TD.diam.']);
    end

    
    % ==== 3- Equivalent resistance: existing + reinforcement ====

    nA = height(TA);
    nD = height(TD);

    % --- Resistance of the existing pipe(s) ---
    % ID -> position in TD
    [tf1, loc1] = ismember(TA.ID1, TD.ID);
    [tf2, loc2] = ismember(TA.ID2, TD.ID);

    use1 = TA.lon1 > 0;
    use2 = TA.lon2 > 0;

    assert(all(tf1(use1)), ...
        'Unknown pipe ID in TA.ID1 where lon1 > 0');
    assert(all(tf2(use2)), ...
        'Unknown pipe ID in TA.ID2 where lon2 > 0');

    % Fictitious pipe
    IDfictif = TD.ID(strcmp(TD.Nominal,'fictif'));

    real1 = use1 & (TA.ID1 ~= IDfictif);
    real2 = use2 & (TA.ID2 ~= IDfictif);

    % Is there a real existing pipe on the arc?
    isEquipped = real1 | real2;

    % Existing resistance coefficient over the complete arc
    rExist = zeros(nA,1);

    rExist(real1) = rExist(real1) + ...
        TA.lon1(real1) .* TD.resist(loc1(real1));

    rExist(real2) = rExist(real2) + ...
        TA.lon2(real2) .* TD.resist(loc2(real2));

    rExist(isEquipped) = rExist(isEquipped) ./ TA.lon(isEquipped);

    % Existing equivalent conductance
    cExist = zeros(nA,1);
    cExist(isEquipped) = rExist(isEquipped).^(-1/partec.lambda);

    % --- Existing pipe in parallel with every candidate pipe ---
    condEquiv = cExist + TD.cond(:).';

    TR = condEquiv.^(-partec.lambda);

    % For an unequipped arc, impose exact equality with TD.resist
    TR(~isEquipped,:) = repmat(TD.resist(:).',sum(~isEquipped),1);

    % Faire une table de TR
    varNames = TD.ID';
    varNames(1) = "rexist";
    TR = array2table(TR, 'VariableNames', varNames);
    TR = addvars(TR, TA.succ, 'Before', 1, 'NewVariableNames', 'ID');


end
    % Helpers
    % =======================================================
    function [diam] = map_diam(IDcol, lenCol, TD, defaultDiam)
    % Remplit diam à partir de TD(ID→Diameter), uniquement là où lenCol>0
	    n = numel(IDcol);
	    diam = defaultDiam * ones(n,1);
    
%	    if ~ismember('diam', TD.Properties.VariableNames), return; end
    
	    idx = (lenCol > 0);
	    if any(idx)
	        [tf, loc] = ismember(IDcol(idx),TD.ID);
	        % ids = categorical(IDcol(idx));
	        % [tf, loc] = ismember(ids, categorical(TD.ID));
	        % valeurs connues
	        diam(idx) = defaultDiam;
	        diam(idx) = replace_at(diam(idx), tf, TD.diam(loc(tf)));
	    end
    end
    
    function v = replace_at(v, mask, vals)
    % remplace v(mask) par vals en gardant la taille
	    tmp = v;
	    tmp(mask) = vals;
	    v = tmp;
    end