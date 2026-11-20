function [A, nodes_rows] = incidence_intermediaire(TA, TN)
    % TA.pred, TA.succ, TN.noeud sont des string
    % Degré des noeuds

    allEnds = [TA.pred; TA.succ];
    a1 = ismember(TN.noeud, unique(allEnds));
    a2 = ismember(unique(allEnds),TN.noeud); 
    
    [grpIdx, uniqNodes] = findgroups(allEnds); % indices 1..K par nœud unique
    degByUniq = accumarray(grpIdx, 1);  % compte occurrences par indice
    % Allouer ces nombres aux noeuds
    deg = zeros(height(TN),1);
    [tf, loc] = ismember(TN.noeud, uniqNodes);
    deg(tf) = degByUniq(loc(tf));
    TN.degre = deg;
    
    keep = TN.degre >= 2;
    nodes_rows = TN.noeud(keep);   % lignes gardées
    m = numel(nodes_rows);
    n = height(TA);

    [isPredKept, rowPred] = ismember(TA.pred, nodes_rows);
    [isSuccKept, rowSucc] = ismember(TA.succ, nodes_rows);

    I = [rowPred(isPredKept); rowSucc(isSuccKept)];
    J = [find(isPredKept)   ; find(isSuccKept)];
    V = [-ones(nnz(isPredKept),1); ones(nnz(isSuccKept),1)];

    A = sparse(I, J, V, m, n);
end
