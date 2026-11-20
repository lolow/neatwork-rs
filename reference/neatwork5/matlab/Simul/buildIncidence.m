function A = buildIncidence(TA,TN)
% Incidence orientée (lignes: arcs, colonnes: noeuds)
m = height(TA); n = height(TN);
A = spalloc(m,n,2*m);
% map noeud -> index
[~,loc] = ismember(TN.noeud, TA.succ); %#ok<ASGLU> % trivial, mais garde l’idée
idx = containers.Map(cellstr(string(TN.noeud)), num2cell(1:n));
for i=1:m
    A(i, idx(char(string(TA.pred(i))))) = -1;
    A(i, idx(char(string(TA.succ(i))))) =  1;
end
end
