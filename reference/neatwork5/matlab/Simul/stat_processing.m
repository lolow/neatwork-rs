function resultats = stat_processing(itesim,simRes,param,dat,TableN,TableA)
% Traitement statistique des performances
% Input essentiels dans la structure simRes avec une colonne par simulation
% Elements avec autant de lignes que d'arcs
    % simRes.X = débits, simRes.iter = nombre d'itérations IPM, 
    % simRes.Ouv = arc suceptibles de porter un débit non-nul.
    % simRes.Charge = perte de charge par arc
% Eléments avec autant de lignes que d'arcs terminaux
    % simRes.Satis = ratio entre demande souhaitée à la borne et réalisée.
    % simRes.OuvT = sous-ensemble de simRes.Ouv concernant les terminaux.
% Eléments avec autant de lignes que d'arcs intermédiaires
    % simRes.U = variables duales (pression) aux nœuds extrémité



    X           = simRes.X;
    Iter        = simRes.Iter;
    Ouv         = simRes.Ouv;
    OuvT        = Ouv(dat.idAter,:);
    Charge      = simRes.Charge;
    U           = simRes.U;
    Satis       = simRes.Satis; 

    nbT     = dat.nbAter;
    nbB     = sum(dat.idAtra);
    nbS     = sum(dat.idAsrc);

    
    % %% Statistiques sur les débits
    % % Niveau de staisfaction pour les utilisateurs 
    % minSat = zeros(dat.nbAter,1) ;
    % maxSat = zeros(dat.nbAter,1) ;
    % moySat = zeros(dat.nbAter,1) ;
    % cVarSat20 = zeros(dat.nbAter,1) ;
    % cVarSat5 = zeros(dat.nbAter,1) ;
    % minFlow = zeros(dat.nbAter,1) ;
    % maxFlow = zeros(dat.nbAter,1) ;
    % moyFlow = zeros(dat.nbAter,1) ;
    % cVarFlowt20 = zeros(dat.nbAter,1) ;
    % cVarFlow5 = zeros(dat.nbAter,1) ;
    % for i = 1:dat.nbAter
    %     % j = dat.setAter(i) ;
    %     % a = logical(Ouv(j,:)) ;
    %     satis = X(i,OuvT(i,:)) ;
    %     x = Satis(i,OuvT(i,:)) ;
    %     if isempty(satis)
    %         minSat(i) = 0 ;
    %         maxSat(i) = 0 ;
    %         moySat(i) = 0 ;
    %         cVarSat5(i) = 0;
    %         cVarSat20(i) = 0 ;
    %     else
    %         minSat(i) = min(satis) ;
    %         maxSat(i) = max(satis) ;
    %         moySat(i) = mean(satis) ;
    %         varSat5 = (satis <= percentile(satis,5)) ;
    %         cVarSat5(i) = mean(satis(varSat5));
    %         varSat20 = (satis <= percentile(satis,20)) ;
    %         cVarSat20(i) = mean(satis(varSat20)) ;
    %     end
    % 
    % borne = TableN.noeud(dat.idNter) ;
    % TableSat = table(minSat,moySat,maxSat,cVarSat5,cVarSat20,'RowNames',borne);
    % Satborne = sprintf('\n Satisfaction aux bornes');
    % 

    %% Statistiques sur les satisfactions
    % Niveau de staisfaction pour les utilisateurs 
    minSat = zeros(dat.nbAter,1) ;
    maxSat = zeros(dat.nbAter,1) ;
    moySat = zeros(dat.nbAter,1) ;
    cVarSat20 = zeros(dat.nbAter,1) ;
    cVarSat5 = zeros(dat.nbAter,1) ;
    for i = 1:dat.nbAter
        % j = dat.setAter(i) ;
        % a = logical(Ouv(j,:)) ;
        satis = Satis(i,OuvT(i,:)) ;
        if isempty(satis)
            minSat(i) = 0 ;
            maxSat(i) = 0 ;
            moySat(i) = 0 ;
            cVarSat5(i) = 0;
            cVarSat20(i) = 0 ;
        else
            minSat(i) = min(satis) ;
            maxSat(i) = max(satis) ;
            moySat(i) = mean(satis) ;
            varSat5 = (satis <= percentile(satis,5)) ;
            cVarSat5(i) = mean(satis(varSat5));
            varSat20 = (satis <= percentile(satis,20)) ;
            cVarSat20(i) = mean(satis(varSat20)) ;
        end
    end
    
    borne = TableN.noeud(dat.idNter) ;
    TableSat = table(minSat,moySat,maxSat,cVarSat5,cVarSat20,'RowNames',borne);
    Satborne = sprintf('\n Satisfaction aux bornes');
    
    
    % Débits aux stations (Les débits sont < 0; on rapporte leur valeur absolue)
    % en litres par seconde
    stations = TableN.noeud(dat.idNsrc) ;
    Xst = X(dat.idAsrc,:) ;
    % nbS = sum(dat.idNsrc) ;
    cVarSt5 = zeros(nbS,1) ;
    cVarSt95 = zeros(nbS,1) ;
    for i=1:nbS
        varSt5 = (Xst(i,:) <= percentile(Xst(i,:),5)) ;
        cVarSt5(i) = mean(Xst(i,varSt5)) * 1000 ;
        varSt95 = (Xst(i,:) >= percentile(Xst(i,:),95)) ;
        cVarSt95(i) = mean(Xst(i,varSt95)) * 1000 ;
    end
    minSt = min(Xst,[],2) * 1000 ;
    moySt = mean(Xst,2) * 1000  ;
    maxSt = max(Xst,[],2) * 1000  ;
    xst =  sprintf('\n Débits aux stations') ;
    TableSt = table(minSt,moySt,maxSt,cVarSt5,cVarSt95,'RowNames',stations);
    
    
    % Vitesse dans les arcs intermédiaires
    idAinter = dat.idAtra | dat.idAsrc;
    xb = abs(X(idAinter,:));
    diam1 = TableA.diam1 ;
    surface = double(pi * diam1(idAinter).^2 /4) ;
    Vit = xb ./ surface ;
    minV = min(Vit,[],2) ;
    moyV = mean(Vit,2) ;
    maxV = max(Vit,[],2) ;
    fin = TableA.succ(idAinter) ;
    debut = TableA.pred(idAinter) ;
    TableResV = table(debut,fin,minV,moyV,maxV) ;
    vitesse = sprintf('\n Vitesses dans les arcs intermédiaires en m/s');
    res = sprintf(['Résultats \n    noeud    fréquence  min       moy    '...
        '   max       cVar20%%   coefvar ']);
    
    
    % Info sur les pressions
    
    % Pressions résiduelles aux bornes
    resP = zeros(nbT,3);
    chargeT = Charge(dat.idAter,:);
    for i = 1:nbT
        % j = dat.setAter(i) ;
        % % Calcul de la pression en j
        % a = logical(Ouv(j,:)) ;
        % x = X(j,a) ;
        x = X(i,OuvT(i,:)) ;
        if isempty(x)
            resP(i,1:3) = 0 ;
        else
            % c = Charge(j,a) ;
            % alpha = Alpha(j,a) ;
            % pression = -c;
            % - alpha.*x.^2;% - dat.gravit(j) ;
            pression = - chargeT(i,OuvT(i,:));
            resP(i,1) = min(pression) ;
            resP(i,2) = mean(pression) ;
            resP(i,3) = max(pression) ;
        end
    end
    press =  sprintf('\n Pression en amont de l''orifice en mètres');
    % Elements pour les statistiques de pression.
    minP = resP(:,1) ;
    moyP  = resP(:,2) ;
    maxP  = resP(:,3) ;
    TablePress = table(minP,moyP,maxP,'RowNames',borne);
    pborne = sprintf('\n Pressions en amont des orifices en mètres');
    
    % Pression résiduelles aux noeuds intermédiaires
    tnB = TableN.noeud(dat.idNtra) ;
    % prB = -repmat(dat.h(dat.idNtra),1,param.simulmax)+ U ;
    prB = - repmat(TableN.haut(dat.idNtra),1,itesim)+ U ;
    minPrB = min(prB,[],2) ;
    moyPrB = mean(prB,2) ;
    maxPrB = max(prB,[],2) ;
    TablePrB = table(minPrB,moyPrB,maxPrB,'RowNames',tnB);
    pressB =  sprintf(['\n Excès de pression aux nœuds intermédiares ' ...
        'pendant les écoulements']);
    % Info sur itérations
    resIter = sprintf(['\nIterations : min = %d, moyenne = %1.2f, ' ...
        'max = %d %1.2f'], min(Iter),mean(Iter),max(Iter));
    % resTemps = sprintf(['Temps : %1.2f secondes pour %d simulations'],temps,itesim);
    disp(Satborne)
    disp(TableSat)
    disp(xst)
    disp(TableSt)
    disp(vitesse)
    disp(TableResV)
    disp(pborne)
    disp(TablePress)
    disp(pressB)
    disp(TablePrB)
    disp(resIter)
    if isfield(simRes,'TempsIPM')
        disp(sprintf('Temps de calcul des débits %f',simRes.TempsIPM))
    end
    
    print = 1 ;
    if print
        filename = 'simul_debit.xlsx';
        TableSatWithRowNames = [table(TableSat.Properties.RowNames, ...
            'VariableNames', {'Bornes'}) TableSat];
        writetable(TableSatWithRowNames,filename,'Sheet',1,'Range','B2')
        writetable(TableResV,filename,'Sheet',3,'Range','B2')
        writetable(TableSt,filename,'Sheet',2,'Range','B2')
        writetable(TablePress,filename,'Sheet',4,'Range','B2')
        writetable(TablePrB,filename,'Sheet',5,'Range','B2')
    end
    
    louange = sprintf(['C''est-y pas beau ?']);
    disp(louange)
    
    % f8 = sum(cVarSat5 < 0.8);
    % disp(f8)
    % disp(TableSat.Row(cVarSat5 < 0.8))
    %
    % f5 = sum(cVarSat5 < 0.5);
    % disp(f5)
    % disp(TableSat.Row(cVarSat5 < 0.5))
    %
    %
    % disp(mean(cVarSat5)*100)
    % disp(mean(cVarSat5(cVarSat5 < 0.8))*100)
    % disp(mean(cVarSat5(cVarSat5 < 0.5))*100)
    %
    % disp(mean(moySat)*100)
    % disp(mean(moySat(cVarSat5 < 0.8))*100)
    % disp(mean(moySat(cVarSat5 < 0.5))*100)
    %
    % disp(std(moySat)/mean(moySat)*100)
    % disp(std(moySat(cVarSat5 < 0.8))/mean(moySat(cVarSat5 < 0.8))*100)
    % disp(std(moySat(cVarSat5 < 0.5))/mean(moySat(cVarSat5 < 0.5))*100)

end