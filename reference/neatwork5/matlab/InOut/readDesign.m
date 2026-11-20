function [TA,TN,TC,TB,TD,TO,TP,param,dat,pardsg,partec] = readDesign(nom,version,secu,fusion,split,type,lieu)
% readDesign va rechercher le ou les fichier de données d'un design et 
% diverses tables et structures nécessaires pour la simulation
%  
% (Input)
    % nom,version,secu,fusion,split,type sont les constituants de
    % l'adresse du fichier
% Output
    % TA table des arcs, TN table des noeuds, TC table des clients, TD
    % tables de tuyaux commerciaux, TP table des paramètres.
    % pardsg et partec : paramètres de conception et techniques avec défauts.
% La fonction considère 4 types de fichier de stockage des des données du
% design. Le fichiers se trouvent dans le dossier examples et/ou dans le
% dossier out. Le premier dossier contient les design test pour calibrer le
% code ; le deuxième héberge les designs générés par l'utilisateur dans le
% module design.

    % Créer le nom du fichier
    if isempty(secu)
        ind_secu = "";
    else
        ind_secu = sprintf('_Secu%1.2f',secu) ;
    end
    if isempty(fusion)
       ind_fusion = "";
    else
        ind_fusion = sprintf('_fusion%d',fusion) ;
    end
    if isempty(split)
        ind_split = "";
    else
        ind_split = sprintf('_split%d',split) ;
    end
    
    root = get_root();  % suppose que ça renvoie .../NW_Git

    % Le nom et l'adresse du fichier recherché et de son dossier diffèrent
    % en fonction du type. On commence par donner la formule pour tous les
    % types autres que txt, soit dsg, mat et json. Les formules pour txt
    % sont précisées là où se trouve la commande de lecture du fichier.
    if ~strcmp(type,'txt')
        nom2fiche = sprintf('%s%s%s%s%s.%s',...
            nom,version,ind_secu,ind_fusion,ind_split,type);
        % Le dossier par défaut est celui des travaux en cours
        adresse2fiche = fullfile(root, lieu, nom, nom2fiche);
        assert(isfile(adresse2fiche), 'fichier introuvable : %s', adresse2fiche);
        dat.adresse2dossier = "";
        dat.adresse2fiche = adresse2fiche ;
    end

    dat.nom = nom; 
    dat.version = version;
    dat.secu = secu;
    dat.fusion = fusion;
    dat.split = split;
    dat.type = type;
    
    % ==== Récupérer les données (tables, dat et param) ===
    if strcmp(type,'mat') 
        S = load(adresse2fiche);
        TA=S.TA; TN=S.TN; TC=S.TC; TP=S.TPar; TD=S.TD; TO=S.TO; 
        pardsg=S.pardsg; partec=S.partec;
        fn1 = fieldnames(pardsg);
        fn2 = fieldnames(partec);
        val1 = struct2cell(pardsg);
        val2 = struct2cell(partec);
        param = cell2struct([val1; val2], [fn1; fn2], 1);
        clear S
    elseif strcmp(type,'dsg') 
        lines = read_Dsg_dsg(adresse2fiche) ;
        % Break the bloc lines into sections
        [TN, TA, TC, TB, TP, TO, TD, param] = parseDsgToTables(dat,lines) ;
    elseif strcmp(type,'json')
    
        S = read_Dsg_json(adresse2fiche);

        TA = S.TA;
        TN = S.TN;
        TC = S.TC;
        TB = S.TB;
        TD = S.TD;
        TO = S.TO;
        TP = S.TPar;
        pardsg = S.pardsg;
        partec = S.partec;
        clear S;

        % Preserve dat metadata initialized above; dat is not loaded from JSON.

        % Création de la structure unique param 
        name = TP.name; value=TP.value;
        if ~iscell(value), value = num2cell(value); end
        param = cell2struct(value, name, 1);

        % Not strictly necessary, but checks consistency between param built from
        % TP and the merged pardsg/partec structures (partec takes precedence).
        parami = pardsg;
        fieldstec = fieldnames(partec);
        for i = 1:length(fieldstec)
            monchamp = fieldstec{i};
            parami.(monchamp) = partec.(monchamp);
        end
        if ~isequal(param,parami)
            disp('param et parami ne sont pas identiques')
        end
    elseif  strcmp(type,'txt')
        [Tables,dat,param] = LireTablesDsgTxt(nom, version);
        % [Tables,dat,param] = LireTablesDsgTxt(nom, version);
        % Insérer ici une fonction qui teste si les tables ont les bons
        % noms.
        TA = Tables.DsgArcs;
        TN = Tables.DsgNodes;
        TC = Tables.DsgClients;
        TD = Tables.DsgDbase ;
        TP = Tables.DsgParam;
        % renommer les variables de TN
        TN.demande = [];
        old = TN.Properties.VariableNames;
        new =   {'noeud','haut','xdata','ydata','orif_ideal','orif_com', 'statut'};
        TN = renamevars(TN,old,new);

    else
        error('this type of file is not taken into consideration')
    end

    % Supprimer du nom2fiche le suffixe (.dsg, .mat ou .json)
    if ~strcmp(type,'txt')
        if strcmp(type,'json'),fin=5;else fin=4; end
        dat.nom2fiche = nom2fiche(1:end-fin);
    end
    
    % Make sure all data are clean. The helper guarantees clean keys.
    pred = string(TA.pred);
    TA.pred = normalize_id_column(pred, 'Case','as-is');
    succ = string(TA.succ);
    TA.succ = normalize_id_column(succ, 'Case','as-is');
    if ismember('ID1', TA.Properties.VariableNames)
        TA.ID1 = normalize_id_column(string(TA.ID1), 'Case','as-is');
    end
    if ismember('ID2', TA.Properties.VariableNames)
        TA.ID2 = normalize_id_column(string(TA.ID2), 'Case','as-is');
    end  
    noeud = string(TN.noeud);
    TN.noeud = normalize_id_column(noeud, 'Case','as-is');
    borne = string(TC.borne);
    TC.borne = normalize_id_column(borne, 'Case','as-is');
    client = string(TC.client);
    TC.client = normalize_id_column(client, 'Case','as-is');
    ID = string(TD.ID);
    TD.ID = normalize_id_column(ID, 'Case','as-is');

    % Preserve format defaults, then let the cleaned client data take priority.
    if strcmp(type,'dsg') || strcmp(type,'json')
        if ~isfield(param,'bounded_flow') || isempty(param.bounded_flow)
            param.bounded_flow = 1;
        end
    end
    [TN, param] = apply_client_data(TN, TC, param);

    % Keep other fields of param (notably simulation settings).
    [pardsg, partec] = extract_parameters(param);

end
