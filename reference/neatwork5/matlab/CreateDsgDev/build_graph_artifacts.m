function dat = build_graph_artifacts(TA, TN, dat)
% BUILD_GRAPH_ARTIFACTS
%
% Construct graph structures used by the design algorithms:
%   - reduced incidence matrix
%   - path-to-terminal matrix
%
% IMPORTANT ARCHITECTURAL RULE
% ----------------------------
% This function belongs to the topology-processing stage of the pipeline.
% It must be executed AFTER all topology modifications (fusion, split,
% orientation) and BEFORE any design algorithm is applied.
%
% These matrices represent objective properties of the network graph and
% are independent of the design method (heuristic NW4 loads, robust
% optimization, etc.).
%
% Therefore this computation must NOT be embedded inside design-specific
% functions such as build_incidence_loads().

% Calcule M_incid et M_path à partir d'un arbre (pred->succ).
% - Entrées :
%     TA(pred, succ, lon, ...)   (categorical pour pred/succ)
%     TN(noeud, haut, nbrob, ...) (categorical pour noeud)
%  
% - Output :
%     dat.M_incid ((|TN|-1) x |TA|) sparse
%         Rows: TN order, excluding the source.
%         Columns: TA order.
%     dat.M_path (|TA| x |TA|) sparse
%         Column j: path to TA.succ(j), with arcs in TA order.
%     dat.idArcT (|TA| x 1) logical
%         True for terminal arcs, in TA row order.
%     dat.Path2Term (|TA| x nnz(dat.idArcT)) sparse
%         Paths to terminal arcs, with rows and columns in TA order.

    % Number of rows in the 2 tables
    nbArcs = height(TA);
    nbNodes = height(TN);
    
    % Indices des prédécesseurs et successeurs
    [tfPred, predIdx] = ismember(TA.pred, TN.noeud);
    [tfSucc, succIdx] = ismember(TA.succ, TN.noeud);
    
    if ~all(tfPred)
        error("Certains prédécesseurs de TA sont absents de TN.noeud.");
    end
    
    if ~all(tfSucc)
        error("Certains successeurs de TA sont absents de TN.noeud.");
    end
    
    % ========== Incidence and path matrices ===========
    
    % Matrice d'incidence complète : noeuds x arcs
    Afull = sparse(predIdx, 1:nbArcs, -1, nbNodes, nbArcs) + ...
            sparse(succIdx, 1:nbArcs,  1, nbNodes, nbArcs);
    
    % Identification de la source :
    % elle n'apparaît jamais dans la liste des successeurs
    isSource = ~ismember(TN.noeud, TA.succ);
    sourceIdx = find(isSource);
    
    if numel(sourceIdx) ~= 1
        error("Le réseau doit avoir une source unique. %d candidates trouvées.", ...
              numel(sourceIdx));
    end
    
    % Suppression explicite de la ligne correspondant à la source
    nonSourceIdx = setdiff((1:nbNodes)', sourceIdx, 'stable');
    dat.M_incid = Afull(nonSourceIdx,:);
    
    % Correspondance entre l'indice d'un noeud dans TN
    % et sa ligne dans la matrice d'incidence réduite
    nodeToReducedRow = zeros(nbNodes,1);
    nodeToReducedRow(nonSourceIdx) = 1:numel(nonSourceIdx);
    
    % Ligne de la matrice réduite correspondant au successeur de chaque arc
    arcSuccRow = nodeToReducedRow(succIdx);
    
    % Second membre :
    % la colonne j représente une unité injectée au successeur de l'arc j
    B = sparse(arcSuccRow, 1:nbArcs, 1, nbArcs, nbArcs);
    
    % Calcul de la matrice des chemins
    % Dans une conversion en Python, il faut s'assurer de l'existence de la
    % division matricielle X \ Y au sens de Matlab.
    % La colonne j donne le chemin vers le successeur de l'arc j
    dat.M_path = dat.M_incid \ B;
    
    % Identify terminal arcs
    dat.idArcT = TA.statut == 2;
    
    % Submatrix of paths to terminal nodes
    dat.Path2Term = dat.M_path(:,dat.idArcT);

end
