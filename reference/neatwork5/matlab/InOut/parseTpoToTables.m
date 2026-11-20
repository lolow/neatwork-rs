function [TN, TA, TC, TB, TP, TD, TO, partec, pardsg] = parseTpoToTables(dat)
%PARSETOPOTABLES  Décortique le .tpo en tables TN,TA,TP,TD,TO + params
% Les tables TN,TA et TP sont lues dans le fichier xxx.tpo. Les tables TD
% et TO sont lues dans deux fichiers diameters.db et orifices.db. La table
% des paramètres est complétée. Les éléments nouveaux reçoivent des valeurs
% par défaut.
% Les tables TC et TB peuvent être soit des sections du fichier xxx.tpo
% préparés pour NW5, soit construites à partir des éléments déjà récupérés
% dans les sections !Nodes, !Pipes et !Default properties du fichier NW4.

% Usage:
%   [TN,TA,TC,TP,TD,TO,partec,pardsg] = parseTpoToTables(dat, lines)   
%
% Requiert dat.nom (char ou string) et dat.nomv.


    % ---- Lecture du fichier source .tpo ---
    if exist('get_root','file')
        root = get_root();                           
    else
        here = fileparts(mfilename('fullpath'));     % .../matlab/CreateDsg/
        root = fileparts(fileparts(fileparts(here)));% remonte à .../NW_Git
    end

    nom  = dat.nom;
    nomv = dat.nomv;
    filename = fullfile(root, 'examples', 'tpo_examples', nom, [nomv '.tpo']);
    
    if ~exist(filename, 'file')
        error('Fichier introuvable : %s', filename);
    end
    assert(exist(filename,'file')==2, 'Introuvable: %s', filename);
    fid = fopen(filename, 'r');
    assert(fid~=-1, 'Impossible d''ouvrir : %s', filename);
    lines = textscan(fid, '%s', 'Delimiter', '\n', 'Whitespace', '');
    fclose(fid);
    
    lines = string(lines{1})  ;

    % -------- repérage des sections --------
    idxNodes   = find(startsWith(lines, '!Nodes'),   1);
    idxPipes   = find(startsWith(lines, '!Pipes'),   1);
    idxDefault = find(startsWith(lines, '!Default properties'), 1);

    if isempty(idxNodes) || isempty(idxPipes) || isempty(idxDefault)
        error('Sections manquantes dans le .tpo (attendu: !Nodes, !Pipes, !Default properties).');
    end
    if ~(idxNodes < idxPipes && idxPipes < idxDefault)
        error('Ordre des sections inattendu dans le .tpo.');
    end

    % ================== NOEUDS (TN) ==================
    sectionName = 'Nodes';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    nodeBlob  = char(join(block, newline));
    nodeData  = textscan(nodeBlob, '%s %f %f %f %f %d', 'Delimiter', ',');    
    TN = table(nodeData{1}, nodeData{2}, nodeData{3}, nodeData{4}, nodeData{5}, nodeData{6}, ...
        'VariableNames', {'noeud','haut','xdata','ydata','nbrob','statut'});
    TN = sortrows(TN, {'statut','noeud'});

        % ================== Arcs (TA) ==================
    sectionName = 'Pipes';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    pipeBlob  = char(join(block, newline));
    pipeData  = textscan(pipeBlob, '%s %s %f', 'Delimiter', ',');
    TA = table(pipeData{1}, pipeData{2}, pipeData{3}, ...
        'VariableNames', {'pred','succ','lon'});
    TA = sortrows(TA, {'succ','pred'});    
    % Zero-length arcs are interpreted as control devices (valves, orifices).
    % Since the current hydraulic model assumes a geometric pipe segment,
    % we replace zero lengths by a minimal artificial length.
    % Future versions may treat such elements explicitly.
    lon_eps = 0.01;   % minimal artificial length (m)
    TA.lon = max(TA.lon, lon_eps);



        % ============ PARAMÈTRES LUS (TP) =========
    sectionName = 'Default properties';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);   
    paramBlob  = char(join(block, newline));
    paramData = textscan(paramBlob, '%s %f', 'Delimiter', ',');
    TP = table(paramData{1}, paramData{2}, 'VariableNames', {'property','value'});
    TP = sortrows(TP,'property');

    % struct param à partir de TP
    param = cell2struct(num2cell(TP.value), TP.property);
    % Shared defaults for topology and design inputs.
    [pardsg, partec] = extract_parameters(param);

    % ======= DIAMÈTRES (TD) in the file or separately =========
    % Retrieve as a section if exists.
    sectionName = 'Diameters';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    if hadSection
        diamBlob  = char(join(block, newline));
        diamData = textscan(diamBlob, '%s %s %f %f %f %f %d %f', 'Delimiter', ',');
        TD = table(diamData{1}, diamData{2}, diamData{3}, diamData{4}, ...
            diamData{5}, diamData{6}, diamData{7}, diamData{8}, 'VariableNames', ...
            {'ID','Nominal','SDR','diam','cost','pressure','type','Roughness'});
    else
        diaFile = fullfile(root, 'examples', 'tpo_examples', nom, sprintf('DiamBase%s.db', nom));
        assert(exist(diaFile,'file')==2, 'Fichier introuvable : %s', diaFile);
        fid = fopen(diaFile,'r');  assert(fid~=-1, 'Ouverture impossible : %s', diaFile);
        C = onCleanup(@() fclose(fid));
        R = textscan(fid, '%s', 'Delimiter', '\n', 'Whitespace', '');
        diamLines = string(R{1});
        diamBlob  = char(join(diamLines, newline));
        diamData  = textscan(diamBlob, '%s %s %f %f %f %f %f', 'Delimiter', ',');
        TD = table(diamData{1}, diamData{2}, diamData{3}, diamData{4}, ...
            diamData{5}, diamData{6}, diamData{7}, ...
            'VariableNames', {'Nominal','SDR','diam','cost','pressure','type','Roughness'});
        TD = sortrows(TD, 'diam');
        
        % ID D001, D002,... en première colonne (categorical)
        n  = height(TD);
        ID = categorical("D" + compose("%03d", (1:n)'));
        TD = addvars(TD, ID, 'Before', 1, 'NewVariableNames', "ID");
    end
    
    % ================ ORIFICES (TO) =================
    sectionName = 'Orifices';
    [block, firstData, lastData] = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    if hadSection
        orifBlob  = char(join(block, newline));
        orifData = textscan(orifBlob, '%f %s', 'Delimiter', ',');
        TO = table(orifData{1},orifData{2}, 'VariableNames', {'in_diam','N'});
        TO.N = [];
    else
        oriFile = fullfile(root, 'examples', 'tpo_examples', nom, 'orifices.db');
        assert(exist(oriFile,'file')==2, 'Fichier introuvable : %s', oriFile);
        TO = readtable(oriFile, 'FileType','text', 'ReadVariableNames', false);
        TO.Properties.VariableNames = {'in_diam'};
    end

    % Types utilisés pour les jointures
    TN.noeud = categorical(string(TN.noeud));
    TA.pred  = categorical(string(TA.pred));
    TA.succ  = categorical(string(TA.succ));
    TD.ID  = categorical(string(TD.ID));
    % TO.ID  = categorical(string(TO.ID));

    % =========== Read or build section Client (TC) ===========
    sectionName = 'Client';
    [block,~,~]  = getSectionBlock(lines, sectionName);
    hadSection = ~isempty(block);
    if hadSection
        clientBlob  = char(join(block, newline));
        clientData = textscan(clientBlob, '%s %s %f %f','Delimiter', ',');
        TC = table(clientData{1}, clientData{2}, clientData{3}, clientData{4},...
            'VariableNames', {'client','borne','proba','cible'}) ;
    else
        % Build missing table TC
        maskTerm = (TN.statut == 2);
        nbrob    = double(TN.nbrob);
        nbrob(~isfinite(nbrob)) = 0;

        rows = sum(nbrob(maskTerm));
        TC = table('Size',[rows 4], ...
            'VariableTypes', {'string','string','double','double'}, ...
            'VariableNames', {'client','borne','proba','cible'});

        g = 0; 
        % compteur global pour IDs C1, C2, ...
        for i = 1:height(TN)
            if TN.statut(i) ~= 2 || nbrob(i) <= 0, continue; end
            b = string(TN.noeud(i));        % borne (ex. "B1")
            for k = 1:nbrob(i)
                g = g + 1;
                TC.client(g)        = "C" + string(k) + b;  % ex. C4B2
                TC.borne(g)     = b;
                TC.proba(g)     = pardsg.opentaps;
                TC.cible(g)     = pardsg.targetflow;
            end
        end
    end
    TC = sortrows(TC,{'borne','client'},'ascend');
    [TN, pardsg] = apply_client_data(TN, TC, pardsg);

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
        pressure = ones(rows,1) * pardsg.pressure;
        TB = table(borne,pressure);
    end
    TB = sortrows(TB,"borne","ascend");
end
% 
% % ---------- utilitaire interne ----------
% function v = mustfield(s, fld, default)
% %MUSFIELDS checks wheter the field fld belongs to the structure s. If not, 
% %the field is created and is assigned the value default
%     if isfield(s, fld) && ~isempty(s.(fld))
%         v = s.(fld);
%     else
%         v = default;
%     end
% end
