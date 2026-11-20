function [TC, hadSection] = read_or_build_TC(lines, sectionName, TN, param)
    % Lecture de la section !Client dans un .dsg
    % Colonnes attendues (dans cet ordre) : ID, nodeID, proba, cible
    %
    % Requiert: getSectionBlock(lines, "Client")

    if nargin < 2 || strlength(sectionName)==0, sectionName = "Client"; end
    

    [block,~,~]  = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    if hadSection
        clientBlob  = char(join(block, newline));
        clientData = textscan(clientBlob, '%s %s %f %f %f','Delimiter', ',');
        TC = table(clientData{1}, clientData{2}, clientData{3},...
            clientData{4}, clientData{5},...
            'VariableNames', {'client','borne','proba', 'cible', 'ecart'}) ;
        return
    end

    % Build missing table TC
    % TC = makeTC_from_TN(TN, param);

    % Build missing table TC
    maskTerm = (TN.statut == 2);
    nbrob    = double(TN.nbrob);
    nbrob(~isfinite(nbrob)) = 0;
    
    rows = sum(nbrob(maskTerm));
    TC = table('Size',[rows 6], ...
        'VariableTypes', {'string','string','double','double','double'}, ...
        'VariableNames', {'client','borne','proba','cible','pressure'});
    
    g = 0; % compteur global pour IDs C1, C2, ...
    for i = 1:height(TN)
        if TN.statut(i) ~= 2 || nbrob(i) <= 0, continue; end
        b = string(TN.noeud(i));        % borne (ex. "B1")
        for k = 1:nbrob(i)
            g = g + 1;
            TC.client(g)        = "C" + string(k) + b;  % ex. C4B2
            TC.borne(g)     = b;
            TC.proba(g)     = param.opentaps;
            TC.cible(g)     = param.targetflow;
            TC.pressure(g)  = param.pressure ;
        end
    end
    
    % — Normalisation de types —
    TC.client    = string(TC.client);
    TC.borne = string(TC.borne);
    
    % — 1) Unicité stricte des clients —
    if numel(unique(TC.client)) < height(TC)
        error("Doublons dans TC.ID (chaque client doit être unique).");
    end
    
    % — 2) Une (et une seule) borne par client → ici: non vide
    if any(strlength(TC.borne) == 0)
        error("Chaque client doit avoir exactement une 'borne' non vide.");
    end
    
    % — 3) La borne doit être un nœud terminal (statut==2) —
    noeudsTerm = string(TN.noeud(maskTerm));
    
    bad = ~ismember(TC.borne, noeudsTerm);
    if any(bad)
        error("Borne(s) non terminales ou inconnues: %s", strjoin(unique(TC.borne(bad)), ", "));
    end
end
