function [Tables,dat,param] = LireTablesDsgTxtcopie(nom, version)
% Lire des tables en format txt. Ce format n'est pas destiné à perdurer. Il
% a été utilisé lors des tests de développement de l'optim robuste pour le
% design et de la méthode IPM pour la simulation.
% A partir du nom du projet (réseau) et de la version du design, la
% fonction récupère 5 fichiers .txt décrivant les propriétés des noeuds,
% des arcs et des clients ainsi que celles des paramètres (design et
% simulation) et des caractéristiques des tuyaux
% En sortie, la fonction renvoie une structure Tables dont les éléments
% sont les tables issues des fichiers .txt.
% La fonction contient 3 fonctions d'appui (helpers). Leur but est de
% déterminer le type de délimteur utilisé dans le fichier .txt et insérer
% cette information dans la fonction Matlab readtable( ,ops) via la
% structure d'options ops.

    root = get_root();  % Renvoie  à /NW_Git%  

    % --- Nom et adresse pour retrouver les fichiers ---
    % Le dossier au nom2dossier = 'nom' peut contenir plusieurs design du
    % même réseau.
    nom2fiche = sprintf('%s%sTxt',nom,version);
    nom2dossier = sprintf('%sTxt',nom);
    adresse2dossier = fullfile(root, 'examples', nom2dossier);
    assert(isfolder(adresse2dossier), 'Dossier introuvable : %s', adresse2dossier);

    % Stocker les informations
    dat.adresse2fiche = "";
    dat.adresse2dossier = adresse2dossier;
    dat.nom2fiche = nom2fiche ;
    dat.nom = nom;
    dat.version = version;
    dat.secu = "";
    dat.fusion = "";
    dat.split = "";
    dat.type = 'txt';

    % Passer en revue DsgArcs, DsgClients,DsgNodes,DsgDbase,DsgParam
    fiches = {'DsgArcs', 'DsgClients','DsgNodes','DsgDbase','DsgParam'};

    % Colonnes textuelles par fiche (ajuste selon tes schémas)
    colReq = struct( ...
        'DsgArcs',    ["pred","succ","lon1","diam1","ID1","lon2","diam2","ID2"], ...
        'DsgClients', ["borne","client","proba","cible"], ...
        'DsgNodes',   ["noeud","xdata","ydata","orif_ideal","orif_com","statut"], ...
        'DsgDbase',   ["ID","diam","cost"], ...
        'DsgParam',   ["property","value"] );
    textColsMap = struct( ...
        'DsgArcs',    ["pred","succ","ID1","ID2"], ...
        'DsgClients', ["borne","client"], ...
        'DsgNodes',   ["noeud"], ...
        'DsgDbase',   ["ID"], ...
        'DsgParam',   ["property"] );


    Tables = struct();  % Structure pour stocker les 5 tables
    
    for i = 1:numel(fiches)
        % --- Lecture de la table pour la fiche courante ---
        % adresse du fichier
        base = fiches{i};
        nom2fiche = sprintf('%s%s%s.txt', nom, version, base);
        fn = fullfile(adresse2dossier, nom2fiche);
        assert(exist(fn,'file')==2, 'Fichier introuvable : %s', fn);

        % Lire la table
        delim = pick_delimiter(fn);             % '\t' ou ','
        opts = detectImportOptions(fn, 'FileType','text', 'Delimiter',delim,...
            'TextType','string');

        T = read_txt_section(fn, base, textColsMap);
        Tables.(base) = T;


        % Lire la table
        delim = pick_delimiter(fn);             % '\t' ou ','
        opts = detectImportOptions(fn, 'FileType','text', 'Delimiter',delim,...
            'TextType','string');

        % --- Tester si la table comporte les bonnes colonnes ---

        % Traiter le cas particulier de la table des Arcs
        if strcmp(base,'DsgArcs')
            % Lire la table en s'assurant des bons formats
            % wantText = ["pred","succ","ID1","ID2"];
            % present  = intersect(string(opts.VariableNames), wantText, 'stable');
            % if ~isempty(present)
            %     opts = setvartype(opts, cellstr(present), 'string');
            % end
            % 
            % T = readtable(fn,opts) ;
            vn = string(T.Properties.VariableNames);

            % paires
            pairDiamOK = all(ismember(["diam1","diam2"], vn));
            pairIDOK   = all(ismember(["ID1","ID2"],   vn));
            if ~(pairDiamOK || pairIDOK)
                error('DsgArcs: il faut (diam1,diam2) ou (ID1,ID2) dans %s.', nom2fiche);
            end
            
            % colonnes de base (hors paires)
            baseMust = ["pred","succ","lon","lon1","lon2"];   % ajuste à ton schéma
            missingBase = setdiff(baseMust, vn, 'stable');
            if ~isempty(missingBase)
                error('DsgArcs: colonnes requises manquantes dans %s: %s', ...
                      nom2fiche, strjoin(missingBase, ', '));
            end
        else
            T = readtable(fn,opts) ;
            vn = string(T.Properties.VariableNames);
            presence = all(ismember(colReq.(base),vn));
            if ~presence
                missing = setdiff(colReq.(base),vn,'stable');
                error('%s: colonnes requises manquantes dans %s: %s', ...
                    colReq.(base), nom2fiche, strjoin(missing, ', '));
            end
        end
        % Nettoyer la table crée de possibles colonnes éritées de NW4
        if any(strcmp('N', T.Properties.VariableNames))
            T.N = [];
        end
        % Enregistrer la table dans la structure Tables
        Tables.(base) = T;
    end

    % --- Créer la structure param depuis la table TP ---

    TP = Tables.DsgParam ; 



    param = struct();

    % Garder le type dans param tel que dans TP (TP.value peut avoir des
    % valeurs double ou string
    for i = 1:height(TP)
        key = string(TP.property(i));
        param.(key) = TP.value(i);  
    end

    % Tester si les clients sont uniformes et forcer la valeur du paramètre
    TC = Tables.DsgClients ;
    nc = numel(unique(TC.cible));
    np = numel(unique(TC.proba));
    if (nc >1) || (np > 1)
        param.uniform_dem = 0;
        param.bounded_flow = 1 ;
    else
        param.uniform_dem = 1;
    end



    % --- Table des nœuds. Renommer les éléments de la variable statut ---
    s = Tables.DsgNodes.statut;
    u = unique(s);
    
    if numel(u) < 3
        error(['DsgNodes.statut doit contenir au moins 3 catégories : ' ...
               'source, nœuds intermédiaires, nœuds terminaux. ' ...
               'Trouvé: %s'], mat2str(u(:).'));
    end
    
    if ~all(u == [0;1;2])
        % Normalisation min/med/max → 1/2/0
        Tables.DsgNodes.statut(s == min(u)) = 1;             % intermédiaires
        Tables.DsgNodes.statut(s > min(u) & s < max(u)) = 2; % terminaux
        Tables.DsgNodes.statut(s == max(u)) = 0;             % sources
    end
end

function T = read_txt_section(fn, base, textColsMap)
% Lecture texte générique: détecte le délimiteur, lit en string,
% et force les colonnes textuelles prévues par le schéma de la section.

    delim = pick_delimiter(fn); % ton helper existant

    opts = detectImportOptions(fn, ...
        'FileType','text', ...
        'Delimiter',delim, ...
        'TextType','string');      % tout texte = string (pas cellstr)

    % Forcer les colonnes textuelles prévues pour cette section (si présentes)
    vn = string(opts.VariableNames);
    if isfield(textColsMap, base)
        wantText = textColsMap.(base);
        force    = intersect(vn, wantText, 'stable');
        if ~isempty(force)
            opts = setvartype(opts, cellstr(force), 'string');
        end
    end
    T = readtable(fn, opts);
end


function delim = pick_delimiter(fn)
% Heuristique ultra-simple : regarde la 1ère ligne, choisit celui qui "gagne".
    fid = fopen(fn,'r');  assert(fid>0, 'Impossible d''ouvrir: %s', fn);
    first = fgetl(fid);   fclose(fid);
    if ~ischar(first); first = ''; end
    nt = count(first, sprintf('\t'));
    nc = count(first, ',');
    if nt==0 && nc==0
        % fallback: tente tab d'abord (cas colonnes uniques)
        delim = '\t';
    else
        delim = iff(nt >= nc, '\t', ',');   % priorise celui qui apparaît le plus
    end
end


function y = iff(cond, a, b)
if cond, y=a; else, y=b; end
end