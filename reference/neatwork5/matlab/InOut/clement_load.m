function TL = clement_load(TN,TL,TC,TB,pardsg,dat)
%CLEMENT_LOAD Implement Clement's heuristic to estimate loads
%   The present implementation is only relevant in cases where demand is 
%   uniform. The binomial distribution is approximated by the normal law.
%   The heuristic should apply only when the number of downstream faucets
%   (or hydrants) is larger or equal to a threshold (10?). This part is not
%   implemented.

arguments (Input)
    TN
    TL
    TC
    TB
    pardsg
    dat
end

arguments (Output)
    TL
end
if pardsg.uniform_demand = 0
    error('The current implementation does not handle non uniform demands')
end

chemins     = (table2array(TL) > 0);
nbprises    = TN.nbrob(TN.statut == 2);
nbaval      = chemins * nbprises;
p           = pardsg.opentaps ;
maxload     = pardsg.targetflow * nbaval; 
moy         = p * nbaval ;
var         = (1-p) * p * nbaval ;
load_Clem0  = moy + norminv(pardsg.servicequal) * sqrt(var);
load_Clem0  = min(load_Clem0,nbaval);
load_Clem   = load_Clem0 * pardsg.targetflow ;
load_NW4    = full(max(table2array(TL),[],2));

T               = table();T.noeud = TN.noeud(2:end);
T.nbaval        = nbaval;
T.load_Clem0    = load_Clem0;
TL{:,:}         = full(dat.Path2Term).*T.load_Clem0*pardsg.targetflow;

end