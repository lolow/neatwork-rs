function [TA_oriented, TN_out, report] = peel_orient_tree(TN, TA)
% PEEL_ORIENT_TREE Préparation topologique pour la conception.
%   Regroupe temporairement les arcs par paire non orientée de nœuds, puis
%   vérifie que le graphe simplifié est connexe et sans cycle sur tout TN.
%   Si c'est un arbre, le pèle et oriente tous les arcs physiques depuis
%   la source unique (TN.statut == 0). Sinon, TA est rendu inchangé.
%   TN et l'ordre des lignes de TA sont conservés, sans fusion hydraulique.
%
%   report.is_tree       : arbre connexe après réduction des parallèles
%   report.has_parallel  : présence de plusieurs arcs pour une même paire
%   report.source_degree : nombre de voisins distincts de la source
%   report.is_connected, report.has_cycle : diagnostic du graphe simplifié
%   report.parallel_groups : cellules contenant les lignes de TA par groupe
%   report.iterations, report.layers : étapes du pelage
%   report.leftoverTA : arcs physiques non pelés (tout TA si non-arbre)
%   report.reason : diagnostic et éventuel besoin d'une source fictive
%
%   Le degré 1 de la source est une convention de conception, pas une
%   condition d'arborescence. L'appelant décide des suites du diagnostic.
%   Une source unique et des identifiants de nœuds non ambigus sont requis.

    % -------- 0) Préparations & validations légères
    TN_out = TN;                              % on ne réordonne ni TN ni TA
    n = height(TN_out);
    m = height(TA);

    if ~ismember('noeud', TN_out.Properties.VariableNames)
        error('TN doit contenir la variable "noeud".');
    end
    if ~ismember('statut', TN_out.Properties.VariableNames)
        error('TN doit contenir la variable "statut" (0 = source).');
    end
    if ~all(ismember({'pred','succ'}, TA.Properties.VariableNames))
        error('TA doit contenir les variables "pred" et "succ".');
    end

    % On compare par libellés : passer tout en string pour un mapping robuste
    nodeLabels = string(TN_out.noeud);
    predLabels = string(TA.pred);
    succLabels = string(TA.succ);

    % Source : ligne (row) puis code stable = index de ligne
    idsource = (TN_out.statut == 0);
    if nnz(idsource) ~= 1
        error('Il doit y avoir exactement une source avec TN.statut == 0 (trouvé: %d).', nnz(idsource));
    end
    srcRow = find(idsource, 1, 'first');   % index de LIGNE dans TN
    % (srcRow servira d'index de nœud stable dans tout l'algorithme)

    % -------- 1) Mapping TA → indices 1..n (positions dans TN)
    [tfp, pred_idx] = ismember(predLabels, nodeLabels);
    [tfs, succ_idx] = ismember(succLabels, nodeLabels);
    if ~all(tfp) || ~all(tfs)
        missPred = unique(predLabels(~tfp));
        missSucc = unique(succLabels(~tfs));
        error('Noeuds d''arc inconnus: pred manquants=%s | succ manquants=%s', ...
              strjoin(missPred,','), strjoin(missSucc,','));
    end

    % -------- 2) Réduction topologique (A-B et B-A dans le même groupe)
    [pairs, ~, arcGroup] = unique(sort([pred_idx, succ_idx], 2), 'rows');
    topoPred = pairs(:,1);
    topoSucc = pairs(:,2);
    mt = size(pairs, 1);
    groupSizes = accumarray(arcGroup, ones(m,1), [mt,1]);
    parallelIds = find(groupSizes > 1);
    parallelGroups = cell(numel(parallelIds), 1);
    for j = 1:numel(parallelIds)
        parallelGroups{j} = find(arcGroup == parallelIds(j));
    end

    % Les n nœuds sont explicitement inclus, même les nœuds isolés.
    G = graph(topoPred, topoSucc, [], n);
    components = conncomp(G);
    componentCount = max(components);
    is_connected = (componentCount == 1);
    % Valable aussi pour une forêt et les boucles sur un même nœud.
    has_cycle = mt > n - componentCount;
    is_tree = is_connected && ~has_cycle;
    sourceNeighbors = unique([topoSucc(topoPred == srcRow); ...
                              topoPred(topoSucc == srcRow)]);
    source_degree = nnz(sourceNeighbors ~= srcRow);

    % -------- 3) Pelage uniquement si l'orientation depuis la source est définie
    activeArcs = true(mt,1);
    activeNodes = true(n,1);
    peeled_layers = {};
    if is_tree
        while any(activeArcs)
            degCounts = degree_counts_simple(topoPred, topoSucc, ...
                activeArcs, activeNodes, n);
            leaves = find(activeNodes & (degCounts == 1));
            leaves(leaves == srcRow) = [];

            isPredLeaf = activeArcs & ismember(topoPred, leaves);
            isSuccLeaf = activeArcs & ismember(topoSucc, leaves);
            maskSwap = isPredLeaf;
            tmp = topoPred(maskSwap);
            topoPred(maskSwap) = topoSucc(maskSwap);
            topoSucc(maskSwap) = tmp;

            activeArcs(isPredLeaf | isSuccLeaf) = false;
            activeNodes(leaves) = false;
            peeled_layers{end+1} = TN_out.noeud(leaves); %#ok<AGROW>
        end
        % Chaque conduite physique reçoit l'orientation de son arc topologique.
        pred_idx = topoPred(arcGroup);
        succ_idx = topoSucc(arcGroup);
    end

    % -------- 6) Restitution : TA_oriented (sans réordonnancement)
    TA_oriented = TA;  % on préserve l'ordre et toutes les colonnes
    if is_tree
        if iscategorical(TA.pred)
            TA_oriented.pred = categorical(nodeLabels(pred_idx));
        else
            TA_oriented.pred = nodeLabels(pred_idx);
        end
        if iscategorical(TA.succ)
            TA_oriented.succ = categorical(nodeLabels(succ_idx));
        else
            TA_oriented.succ = nodeLabels(succ_idx);
        end
    end

    % Sous-réseau restant (utile si maille)
    leftoverTA = TA_oriented(activeArcs(arcGroup), :);

    % -------- 7) Rapport
    report = struct();
    report.iterations = numel(peeled_layers);
    report.layers = peeled_layers;
    report.leftoverTA = leftoverTA;
    report.is_tree = is_tree;
    report.is_connected = is_connected;
    report.has_cycle = has_cycle;
    report.has_parallel = ~isempty(parallelIds);
    report.parallel_groups = parallelGroups;
    report.source_degree = source_degree;
    if is_tree
        report.reason = 'Le graphe simplifié est un arbre connexe.';
    elseif ~is_connected && has_cycle
        report.reason = 'Le graphe simplifié est déconnecté et contient un cycle.';
    elseif ~is_connected
        report.reason = 'Le graphe simplifié est une forêt déconnectée.';
    else
        report.reason = 'Le graphe simplifié contient un cycle.';
    end
    if report.has_parallel
        report.reason = sprintf('%s Présence de %d groupe(s) d''arcs parallèles.', ...
            report.reason, numel(parallelGroups));
    end
    if source_degree ~= 1
        report.reason = sprintf(['%s Degré topologique de la source : %d. ' ...
            'Pour la conception, ajouter une source fictive reliée à ' ...
            'l''ancienne par un arc de longueur nulle.'], ...
            report.reason, source_degree);
    end
end

% ====== helper clair (boucles) : degré non orienté sur le sous-réseau ======
function degCounts = degree_counts_simple(pred_idx, succ_idx, activeArcs, activeNodes, n)
    degCounts = zeros(n,1);
    m = numel(activeArcs);
    for e = 1:m
        if ~activeArcs(e), continue; end
        p = pred_idx(e); s = succ_idx(e);
        if activeNodes(p), degCounts(p) = degCounts(p) + 1; end
        if activeNodes(s), degCounts(s) = degCounts(s) + 1; end
    end
end
