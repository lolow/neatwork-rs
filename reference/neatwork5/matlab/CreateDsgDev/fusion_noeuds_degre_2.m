function [TableArcsFusion, TableNoeudsFusion, ctx] = fusion_noeuds_degre_2(TA, TN, TD, ctx)
%FUSION This function concatenates the two arcs that are adjacent to the same 
% node of degree 2 into a single arc whose length is the sum of the two lengths.
% Input: tables of arcs, nodes and diameters
% Output: table of arcs and nodes after merger. Table of meerged arcs.
% Fusion is not allowed if the adjacent arcs to degree two node have access
% to the same set of allowed pipes. The concatenated arc is endow with the
% largest pmax of its two members.
% To be improved: The allowedpipes array should be an output.

% Si la fusion n'est pas demandée, on ressort de la fonction
    if ctx.useFusion == 0
        TableArcsFusion = TA ;
        TableNoeudsFusion = TN ;
        changed = 0;
        return
    end

    % % Calcul du degré
    % deg = zeros(height(TN), 1);
    % for i = 1:height(TN)
    %     deg(i) = sum(TA.pred == TN.noeud(i)) + ... 
    %     sum(TA.succ == TN.noeud(i));
    % end

    % === Maximal hydrostatic pressure on each arc ===
	% Height of the predecessor node
	Temp = join(TA, TN, 'LeftKeys','pred', 'RightKeys','noeud', 'RightVariables','haut');
	haut_pred = Temp.haut_TN;

	% Height of the successor node
	Tempi = join(TA, TN, 'LeftKeys','succ', 'RightKeys','noeud', 'RightVariables','haut');
	haut_succ = Tempi.haut_TN;

	% Record maximal hydrostatic pressure on each arc
	TA.pmax = -min(haut_pred, haut_succ);	
    hasTDpress  = ismember('pressure', TD.Properties.VariableNames);
    hasTApmax   = ismember('pmax', TA.Properties.VariableNames);
    
    if hasTDpress && hasTApmax
        allowedPipes = (TA.pmax <= (TD.pressure.'));  % broadcasting logique
	else
        error('Merging is impossible: TD.pressure is missing. Please check the input tables.');
    end

    allowedPipes = buildAllowedPipes(TA, TD);
   
    TableNoeudsFusion = TN ;
    TableArcsFusion = TA ;
%   TableArcsFusion.pressure = [];
    TableFusion = ctx.TableFusion ;

    modif = true;
    while modif
        modif = false;
        for i = 1:height(TableNoeudsFusion)
            if TableNoeudsFusion.degree(i) ~= 2 || TableNoeudsFusion.statut(i) == 0
                continue;
            end
            ni = TableNoeudsFusion.noeud(i);

            idx_pred = TableArcsFusion.succ == ni;
            idx_succ = TableArcsFusion.pred == ni;
            lonpred = TableArcsFusion.lon(idx_pred);
            lonsucc = TableArcsFusion.lon(idx_succ);
            
             if sum(idx_pred) + sum(idx_succ) ~= 2
                continue;
            end
            canFuse = isequal(allowedPipes(idx_pred, :), allowedPipes(idx_succ, :));
            if canFuse == false
                continue
            end

            % Récupération des deux arcs adjacents
            arc1 = TableArcsFusion(idx_pred,:);
            n1 = arc1.pred(1);
            l1 = arc1.lon;
            arc2 = TableArcsFusion(idx_succ, :);
            n2 = arc2.succ(1);
            l2 = arc2.lon;
            % Pression à supporter par le nouvel arc
            pmax = max(TableArcsFusion.pmax(idx_succ),TableArcsFusion.pmax(idx_pred));
            % pression à la borne en cas de mode irrigation
            
            % Nouveau lien entre n1 et n2
            % new_arc = table(n1, n2, l1 + l2, pmax,...
            %     'VariableNames', {'pred', 'succ', 'lon', 'pmax'});
            haut = TableArcsFusion.haut(idx_succ) ;
            statut = TableArcsFusion.statut(idx_succ) ;
            nbrob = TableArcsFusion.nbrob(idx_succ) ;
            pressure = TableArcsFusion.pressure(idx_succ) ;
            new_arc = table(n1, n2, l1 + l2, pmax, haut, statut, nbrob,pressure, ...
                'VariableNames', ...
                {'pred', 'succ', 'lon', 'pmax', 'haut', 'statut','nbrob','pressure'});

            % Supprimer anciens arcs et le noeud de degré 2
            idx_predsucc = logical(idx_pred +idx_succ);
            TableArcsFusion(idx_predsucc, :) = [];
            savePermission = allowedPipes(idx_pred, :);
            allowedPipes(idx_predsucc, :) = [];
            TableNoeudsFusion(i, :) = [];
            % deg(i) = []; % mettre à jour

            % Ajouter l’arc fusionné
            TableArcsFusion = [TableArcsFusion; new_arc];
            allowedPipes =[allowedPipes; savePermission];

            % Enregistrement de la fusion
            TableFusion = [TableFusion; {ni, n1,l1, n2,l2, l1 + l2}];

            modif = true;
            break;
        end
    end
    TableNoeudsFusion.degree = [] ;
    % Modify boolean if TableFusion is not empty
    changed = (height(TableFusion) > 0) ;
    ctx.changed = changed ;
    if changed
        ctx.TableFusion = TableFusion;
    end 
    TableNoeudsFusion = sortrows(TableNoeudsFusion,{'statut','noeud'});
    TableArcsFusion = sortrows(TableArcsFusion,{'statut','succ'});
end

function T = emptyTableFusion()
    T = cell2table(cell(0,6), ...
        'VariableNames', {'noeud_supprime','extremite1','long1', ...
                          'extremite2','long2','lon_total'});
end