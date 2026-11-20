function [ok, report] = validate_tree(TA, TN, racine)
%VALIDATE_TREE  Vérifie que (pred,succ) définit un arbre dirigé (arcs orientés parent->enfant)
% Usage:
%   [ok,report] = validate_tree(TA,TN)
%   [ok,report] = validate_tree(TA,TN,racine)   % racine (char/string/categorical) optionnel
%
% Hypothèses: TA a les variables pred, succ ; TN a la variable noeud.

ok = false;
report = struct();

% Normalise en string pour simplifier
pred = string(TA.pred);
succ = string(TA.succ);
nodes = string(TN.noeud);

% 0) Sanity: membres de l'univers TN
missingPred = pred(~ismember(pred, nodes));
missingSucc = succ(~ismember(succ, nodes));
if ~isempty(missingPred) || ~isempty(missingSucc)
    report.error = "Arc(s) référencent des nœuds absents de TN";
    report.missingPred = unique(missingPred);
    report.missingSucc = unique(missingSucc);
    return
end

% 1) Pas de boucle triviale
if any(pred == succ)
    report.error = "Boucle triviale détectée (pred == succ pour au moins un arc).";
    report.selfLoops = find(pred == succ);
    return
end

% 2) Cardinalité arbre : |E| = |V|-1
if height(TA) ~= height(TN) - 1
    report.error = "Cardinalité invalide: un arbre doit avoir |TA| = |TN|-1.";
    report.edgeCount = height(TA);
    report.nodeCount = height(TN);
    % On continue quand même, mais c'est un indicateur fort.
end

% 3) Racine: le seul nœud qui n'apparaît jamais comme succ
if nargin < 3 || strlength(string(racine)) == 0
    racines = TN.noeud(TN.statut == 0);
    if numel(racines) ~= 1
        report.error = "Impossible d'inférer une racine unique via TN.statut.";
        report.racineCandidates = racines;
        return
    end
    racine = racines(1);
end

report.racine = racine;

% 4) Indegree: chaque nœud (sauf racine) doit apparaître EXACTEMENT une fois comme succ
[uniqSucc,~,ic] = unique(succ);
counts = accumarray(ic, 1);
dupSucc = uniqSucc(counts > 1);
missingSuccs = setdiff(setdiff(nodes, racine), succ); % nœuds hors racine jamais succ

if ~isempty(dupSucc) || ~isempty(missingSuccs)
    report.error = "Indegree invalide: doublons dans succ ou nœuds (hors racine) sans parent.";
    report.duplicatedSucc = dupSucc;          % ces nœuds ont indegree > 1
    report.nodesWithoutParent = missingSuccs; % indegree == 0 hors racine
    return
end

% 5) Connexité: tous les nœuds doivent être atteignables depuis racine (en négligeant l'orientation)
%    -> on fait un BFS en non orienté sur la liste d'arcs
adj = containers.Map('KeyType','char','ValueType','any');
% initialiser adjacency pour tous les nœuds
for k = 1:numel(nodes)
    adj(char(nodes(k))) = string.empty(1,0);
end
% remplir (non orienté)
for e = 1:numel(pred)
    a = char(pred(e)); b = char(succ(e));
    adj(a) = unique([adj(a), string(b)]);
    adj(b) = unique([adj(b), string(a)]);
end

visited = false(size(nodes));
[~, idxracine] = ismember(racine, nodes);
queue = idxracine;

while ~isempty(queue)
    i = queue(1); queue(1) = [];
    if visited(i), continue; end
    visited(i) = true;
    neigh = adj(char(nodes(i)));
    [~, idxNeigh] = ismember(neigh, nodes);
    idxNeigh = idxNeigh(idxNeigh>0 & ~visited(idxNeigh));
    queue = [queue, idxNeigh(:)']; %#ok<AGROW>
end

if ~all(visited)
    report.error = "Le graphe n'est pas connexe (tous les nœuds ne sont pas atteignables depuis la racine).";
    report.unreachedNodes = nodes(~visited);
    return
end

% Si on est arrivé ici, tout va bien
ok = true;
report.message = "OK: (pred,succ) définit un arbre dirigé (parent→enfant) connexe.";
end
