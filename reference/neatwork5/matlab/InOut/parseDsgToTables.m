function [TN, TA, TC, TB, TP, TO, TD, param] = parseDsgToTables(dat, lines)
%PARSEDSGTOTABLES  Parse le .dsg en tables TN,TA,TP,TD,TO + param
% Usage:
%   [TN,TA,TP,TD,TO,param] = parseTpoToTables(dat, lines)   
%
% Requiert dat.nom (char ou string) et dat.nomv.

    % -------- root robuste (indépendant du Current Folder) --------
    if exist('get_root','file')
        root = get_root();                                % .../NW_Git
    else
        here = fileparts(mfilename('fullpath'));          % .../matlab/CreateDsg/ReadTpo
        root = fileparts(fileparts(fileparts(here)));     % remonte à .../NW_Gitnom
    end
    
    % -------- lecture des lignes si besoin --------
    
    dsgFile = dat.adresse2fiche ;
    assert(exist(dsgFile,'file')==2, 'Fichier introuvable : %s', dsgFile);
    fid = fopen(dsgFile,'r');  assert(fid~=-1, 'Ouverture impossible : %s', dsgFile);
    C = onCleanup(@() fclose(fid));
    R = textscan(fid, '%s', 'Delimiter','\n', 'Whitespace','');
    lines = R{1};
    
    % assurer un vecteur de chaînes "string"
    lines = string(lines(:));
    
    
    % =========== Clients (TC) =============
    % Lire la section si elle existe. Sinon envoyer un message d'erreur pour correction.
    sectionName = 'Clients' ;
    [block,~,~]  = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    if hadSection
        clientBlob  = char(join(block, newline));
        clientData = textscan(clientBlob, '%s %s %f %f','Delimiter', ',');
        TC = table(clientData{1}, clientData{2}, clientData{3},...
            clientData{4},...
            'VariableNames', {'client','borne','proba', 'cible'}) ;
    else
    	info = sprintf('Le fichier ne contient pas de section Clients.\nMettre à jour le fichier NW4 avec l''utilitaire maj_NW4_NW5.m');
        error(['Messag long:' newline ...
            'Le fichier ne contient pas de section Clients.'  newline ...
            'Mettre à jour le fichier NW4 avec l''utilitaire.']) ;
    end

    % ================== Arcs (TA) ==================
    sectionName = 'Pipes';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    pipeBlob  = char(join(block, newline));
    pipeData = textscan(pipeBlob, '%s %s %f %f %s %f %s %s', 'Delimiter', ',');
    TA = table(pipeData{1}, pipeData{2}, pipeData{3}, pipeData{4}, ...
        pipeData{5}, pipeData{6}, pipeData{7}, pipeData{8}, 'VariableNames', ...
        {'pred','succ','lon', 'lon1', 'ID1', 'lon2', 'ID2', 'N'});
    TA = sortrows(TA, {'succ','pred'});

    % ================== NOEUDS (TN) ==================
    sectionName = 'Nodes';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    nodeBlob  = char(join(block, newline));
    nodeData  = textscan(nodeBlob, '%s %f %f %f %f %f %d', 'Delimiter', ',');    
    TN = table(nodeData{1}, nodeData{2}, nodeData{3}, nodeData{4}, ...
        nodeData{5}, nodeData{6}, nodeData{7}, 'VariableNames', ...
        {'noeud','haut','xdata','ydata','orif_ideal','orif_com', 'statut'});
    TN = sortrows(TN, {'statut','noeud'});

    % ================== Diameters (TD) ==================
    sectionName = 'Diameters';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    diamBlob  = char(join(block, newline));
    diamData = textscan(diamBlob, '%s %s %s %f %f %f %d %f %s', 'Delimiter', ',');
    TD = table(diamData{1}, diamData{2}, diamData{3}, diamData{4}, ...
         diamData{5}, diamData{6}, diamData{7}, diamData{8}, 'VariableNames', ...
         {'ID','Nominal','SDR', 'diam', 'cost', 'Pmax', 'type', 'Roughness'});

    % % ================== Orifices (TO) ==================
    sectionName = 'Orifices';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    orifBlob  = char(join(block, newline));
    orifData = textscan(orifBlob, '%f %s %s %s %s %s', 'Delimiter', ',');
    TO = table(orifData{1}, 'VariableNames', {'in_diam'});

    % ============ PARAMÈTRES LUS (TP) =========
    sectionName = 'Default properties';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);   
    paramBlob  = char(join(block, newline));
    paramData = textscan(paramBlob, '%s %f', 'Delimiter', ',');
    TP = table(paramData{1}, paramData{2}, 'VariableNames', {'property','value'});
    if ~ismember('pressure',TP.property)
        TP = [TP; {"pressure",  2}]; % default value 
    end
    
    % struct param à partir de TP
    param = cell2struct(num2cell(TP.value), TP.property);

    % =========== Read or build section Borne (TB) ===========
    sectionName = 'Borne';
    [block,~,~]  = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    % Identifier les terminaux qui sont les bornes
    maskTerm = (TN.statut == 2);
    terminal = TN.noeud(maskTerm);
    if hadSection
        borneBlob  = char(join(block, newline));
        borneData = textscan(borneBlob, '%s %f %s','Delimiter', ',');
        TB = table(borneData{1}, borneData{2}, borneData{3},...
            'VariableNames', {'borne','pressure','N'}) ;
        % S'assurer que les bornes correspondent bien aux nœuds terminaux
        borne = unique(TB.borne);
        hasSameSize = numel(borne) == numel(terminal);
        areEqual = isequal(sort(borne),sort(terminal));
        if ~areEqual
            error('Les bornes ne correspondent pas aux noeuds terminaux')
        end
    else
        % Build missing table TB
        borne = terminal;
        rows = numel(borne);        
        pressure = ones(rows,1) * param.pressure;
        TB = table(borne,pressure);
    end
    TB = sortrows(TB,"borne","ascend");

    
    % ==== Types conseillés pour les jointures ===
    TN.noeud    = categorical(string(TN.noeud));
    TA.pred     = categorical(string(TA.pred));
    TA.succ     = categorical(string(TA.succ));
    TD.ID       = categorical(string(TD.ID));
    % TO.ID       = categorical(string(TO.ID));
    TC.client   = categorical(string(TC.client));
    TC.borne    = categorical(string(TC.borne));

    % The client section is required; it supplies counts and uniform demand.
    [TN, param] = apply_client_data(TN, TC, param);

end
% 
% % ---------- utilitaires interne ----------
% function v = mustfield(s, fld, default)
%     if isfield(s, fld) && ~isempty(s.(fld))
%         v = s.(fld);
%     else
%         v = default;
%     end
% end



