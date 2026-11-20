function filename = data4DsgFile(TA,TB,TN,TC,TD,TO,TL,TAH,TCP,pardsg,partec,reinfcost,dat,TS)
%DATA4DSGFILE  Prépare les tables pour export_neatwork_csv et écrit le .dsg
% Sortie : chemin complet du fichier .dsg écrit.
%
% Convention :
%  - TA doit contenir au moins: pred, succ, lon, lon1, ID1, lon2, ID2.
%  - TN: noeud, haut (altitude physique), xdata, ydata, statut,
%    orif_ideal, orif_com. Altitudes and coordinates are exported unchanged.
%  - TD: diam, cost, ... (on ajoute 'NN'='N' pour l’export)
%  - TO: in_diam, ... (on ajoute 5 colonnes 'N' pour l’export)
%
% Le fichier est nommé: <nom><version>_Secu<secu>.dsg dans matlab/TpoFiles/<nom>.

    arguments
        TA table
        TB table
        TN table
        TC table
        TD table
        TO table
        TL table
        TAH table
        TCP table
        pardsg struct
        partec struct
        reinfcost struct
        dat struct
        TS table = table()
    end

	nom = dat.nom;
	version = dat.version;
    assert(ismember(string(dat.output_type), ["dsg", "json"]), ...
        'NW5:OutputFormat', ...
        'Only DSG and JSON exports are supported.');
	
    % ----------- Localise la racine du projet ----------- %
    if exist('get_root','file')
        root = get_root();
    else
        here = fileparts(mfilename('fullpath'));
        root = ascend_to_repo(here); % fallback robuste
    end

    % --- Nom du fichier de sortie au format data.output_type --- %

    % filename = fullfile(root, 'out', char(nom), ...
    %     sprintf('%s%s_Secu%1.2f_fusion%1.0f_split%d.%s', ...
    %     nom, version, pardsg.secu, pardsg.fusion, pardsg.split,dat.output_type));
    suggested_name = sprintf('%s%s_Secu%1.2f_fusion%1.0f_split%d', ...
        nom, version, pardsg.secu, pardsg.fusion, pardsg.split);
    answer = inputdlg('Enter filename to save','Name File',[1 60], {suggested_name});
    if isempty(answer)
        filename = '';
        return
    end


    outdir = fullfile(root, 'out', char(nom));
    if ~isfolder(outdir)
        [ok, msg] = mkdir(outdir);
        assert(ok, 'NW5:OutputDirectory', ...
            'Impossible de creer le dossier %s : %s', outdir, msg);
    end

    fullName = sprintf('%s.%s', answer{1}, dat.output_type);
    filename = fullfile(outdir, fullName);

    % ========== TNodes ==========
    needTN = {'noeud','haut','xdata','ydata','orif_ideal','orif_com','statut'};
    assert(all(ismember(needTN, TN.Properties.VariableNames)), ...
        'TN doit contenir: %s', strjoin(needTN,', '));
    TNodes = TN(:, needTN);

    TN = TNodes;

    % Place 'statut' en dernière colonne (pour coller à l’export historique)
    vars = TNodes.Properties.VariableNames;
    vars = [setdiff(vars, {'statut'}, 'stable'), {'statut'}];
    TNodes = TNodes(:, vars);
    TNodes = sortrows(TNodes, {'statut','noeud'});

    % ========== Table des arcs TArcs ==========
    if ~ismember('N', TA.Properties.VariableNames)
        TA.N = repmat("N", height(TA), 1);
    end
    needTA = {'pred','succ','lon','lon1','ID1','lon2','ID2','N'};
    missTA = setdiff(needTA, TA.Properties.VariableNames);
    assert(isempty(missTA), ...
        'TA doit contenir: %s', strjoin(needTA, ', '));

    TArcs = TA(:, needTA);

    % ========== Table des paramètres TPar ===============
    TPar = vertcat(struct2tableKV(pardsg), struct2tableKV(partec));

    % ========== Table for reinforcment costs ===============
    TCost = struct2tableKV(reinfcost);

    % % ========== Flow factor and margin per connection ==========
    TFact = TAH(:,{'succ','designFlow','margin'});

    % ====== Table des diamères format dsg TDn (diamètres + NN) =====
    TD = TD(:,1:8);  % Caractéristiques persistantes uniquement
    TDn = TD;
    TDn.NN = repmat({'N'}, height(TDn), 1);

    % == Table des orifices TOn format dsg (orifices + 5 colonnes 'N') ==
    block = repmat({'N'}, height(TO), 5);
    Tnew  = array2table(block, 'VariableNames', {'col1','col2','col3','col4','col5'});
    TOn   = [TO, Tnew];
    % The five extra columns required if to be read by NW4. 

    % == Table des bornes s'assurer de la colonne de N pour le format dsg ===
    hasTBN = ismember('N',TB.Properties.VariableNames);
    if ~hasTBN
        block = repmat({'N'}, height(TB), 1);
        Tnew  = array2table(block, 'VariableNames', {'col1'});
        TBn   = [TB, Tnew];
    else
        TBn = TB;
    end


    % ========== Export ==========
    if strcmp(dat.output_type, 'dsg')
        % format dsg
        export_neatwork_csv(filename, TNodes, TArcs, TBn, TC, TPar, TFact, TAH, TCP, TDn, TOn,TCost,TS);
    elseif strcmp(dat.output_type, 'json')
        % format json
        save_dsg_json(filename,TN,TA,TB,TC,TPar,TD,TO,TFact,TAH,TCP,pardsg,partec,reinfcost,dat,TS);
    else
       disp('Format d''exportationn inconnu')
    end
end

% --------- helpers --------- %

function T = struct2tableKV(S)
    fn = fieldnames(S);
    val = struct2cell(S);
    % Convertit en double si possible, sinon string (pour garder l’info)
    vnum = cellfun(@(x) isnumeric(x) && isscalar(x), val);
    outv = cell(size(val));
    outv(vnum)  = val(vnum);
    outv(~vnum) = cellfun(@string, val(~vnum), 'UniformOutput', false);
    T = table(fn, vertcat(outv{:}), 'VariableNames', {'name','value'});
end

function root = ascend_to_repo(here)
    d = here;
    while true
        if exist(fullfile(d,'.git'),'dir') || exist(fullfile(d,'matlab'),'dir')
            root = d; return
        end
        nd = fileparts(d);
        assert(~strcmp(nd,d), 'Racine projet introuvable depuis %s', here);
        d = nd;
    end
end
