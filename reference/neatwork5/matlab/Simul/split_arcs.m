function [TA2,TN2,mapOld2New] = split_arcs(TA,TN)
% Scinde chaque arc ayant lon2>0 en 2 arcs + 1 noeud intermédiaire
TA2 = TA; TN2 = TN;
has2 = TA2.lon2 > 0;
mapOld2New = containers.Map('KeyType','char','ValueType','any');

for i=find(has2).'
    pr = TA2.pred(i); su = TA2.succ(i);
    lon1 = TA2.lon1(i); lon2 = TA2.lon2(i); L = lon1+lon2; f = lon1/L;

    % coords & alt interpolées (suppose TN a x,y,haut)
    P = TN2(TN2.noeud==pr,{'x','y','haut'});
    S = TN2(TN2.noeud==su,{'x','y','haut'});
    xM = (1-f)*P.x + f*S.x; yM = (1-f)*P.y + f*S.y; hM = (1-f)*P.haut + f*S.haut;

    % créer ID nœud intermédiaire unique
    nid = categorical(string(pr)+"§"+string(su)+"§M"+string(i));
    TN2 = [TN2; table(nid,xM,yM,hM,'VariableNames',{'noeud','x','y','haut'})];

    % remplacer l’arc i par 2 arcs (copie attributs utiles)
    base = TA2(i,:);
    a1 = base; a1.succ = nid; a1.lon1 = lon1; a1.lon2 = 0; a1.ID2 = "";
    a2 = base; a2.pred = nid; a2.lon1 = lon2; a2.lon2 = 0; a2.ID1 = base.ID2; a2.ID2 = "";

    TA2(i,:) = a1;
    TA2 = [TA2; a2];

    mapOld2New(char(string(pr)+"-"+string(su))) = [a1.pred a1.succ; a2.pred a2.succ];
end

% Option : réordonner colonnes/IDs si besoin
end
