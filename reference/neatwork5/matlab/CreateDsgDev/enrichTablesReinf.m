function [TA, TB, TN, TD] = enrichTablesReinf(TA, TB, TN, TD, pardsg, source_height)
    %ENRICHTABLES  Ajoute aux arcs les infos utiles et dérivées.
    % INPUT : TA(pred,succ,lon,...)  TN(noeud,haut,...)  TD(...). TB(borne,pressure) 
    %           TN.haut and source_height are physical elevations in metres.
    %           pardsg : design parameters, including source pump_pressure.
    % OUTPUT : New variables for TA (pmax, maximal hydrostatic pressure on each link; 
    %          pressure is the require pressure at each terminal link, i.e. adjacent 
    %          to a terminal node or borne)
    %          TA.haut : downstream elevation minus the source hydraulic head.
    %          TN.haut remains unchanged; degree records node connectivity.
    
    % Check class of variables 'noeud', 'pred', 'succ'
    assert(iscategorical(TN.noeud), 'TN.noeud must categorical (Dee parseTpoToTables).');
    assert(iscategorical(TA.pred)  && iscategorical(TA.succ), ...
           'TA.pred/TA.succ must be categorical (see parseTpoToTables).');
    
    % Types conseillés pour les jointures
    TN.noeud = categorical(string(TN.noeud));
    TA.pred  = categorical(string(TA.pred));
    TA.succ  = categorical(string(TA.succ));
    TB.borne = categorical(string(TB.borne));
    
    % === Calcul de la pression maximale sur chaque arc ===
    Hsource = source_height + pardsg.pump_pressure;
    
    % Hauteur du prédécesseur
    [indic,rang] = ismember(TA.pred,TN.noeud) ;
    haut_pred = NaN(height(TA),1);
    haut_pred(indic) = TN.haut(rang(indic));

    % Hauteur du successeur
    [indic,rang] = ismember(TA.succ,TN.noeud) ;
    haut_succ = NaN(height(TA),1);
    haut_succ(indic) = TN.haut(rang(indic));
    
    TA.pmax = Hsource - min(haut_pred, haut_succ);
    
    % === Calcul du degré des noeuds ===
    % Degré   
    deg = zeros(height(TN), 1);
    for i = 1:height(TN)
        deg(i) = sum(TA.pred == TN.noeud(i)) + sum(TA.succ == TN.noeud(i));
    end
    TN.degree = deg ;
    
    % === Add the variables haut,statut and nrob to TA === 
    TNsucc = renamevars(TN, 'noeud', 'succ');
    TA = join(TA,TNsucc(:,{'succ','haut'}),'Keys','succ');
    TA.haut = TA.haut - Hsource;
    TA = join(TA,TNsucc(:, {'succ', 'statut'}), 'Keys', 'succ');
    TA = join(TA,TNsucc(:, {'succ', 'nbrob'}), 'Keys', 'succ');

    % ==== Align pressure at terminals if uniform_demand =====
    if pardsg.uniform_dem == 1
        TB.pressure = pardsg.pressure * ones(height(TB),1);
    end

    % === Add the variable pression to TA === 
    % Read pressure at junction (borne) in TB and assign it to the arcs in TA

    [indic,rang] = ismember(TA.succ,TB.borne) ;
    pressure = zeros(height(TA),1);
    pressure(indic) = TB.pressure(rang(indic));
    TA.pressure = pressure ;

    % ===== Give statut=0 to the arc adjacent to the source =====
    % src = TN.noeud(TN.statut == 0);
    % a = ismember(TA.pred,src) ;
    TA.statut(a) = 0;
    
    % ==== Reorder TA just like TN (done in parseTpoToTable) ====
    TA = sortrows(TA,{'statut','succ'});
    
end

