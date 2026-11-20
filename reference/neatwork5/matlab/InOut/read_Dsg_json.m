function S = read_Dsg_json(filename, opts)
%READ_DSG_JSON  Charge un design NeatWork depuis un .json et reconstruit les tables.
% S = read_dsg_json(file)
% S = read_dsg_json(file, opts)
%
% Retourne une struct S contenant, si présents dans le JSON :
%   - tables d'export : TNodes, TArcs, TPar, TFact, TDn, TOn
%   - tables moteur   : TA, TN, TD, TO
%   - structs         : pardsg, partec, dat
%
% Notes :
% - Convertit automatiquement les tableaux d'objets JSON -> tables MATLAB.
% - Recoerce en categorical les colonnes d'identifiants (pred, succ, noeud, ID*).
% - Tolère l'absence de certaines clés (elles ne seront pas dans S).
%
% opts.categoricalVars : cellstr de noms de colonnes à forcer en categorical
%                        (par défaut : {'pred','succ','noeud','ID','ID1','ID2'})
%
% Exemple :
%   S = read_dsg_json(fullfile(get_root,'out','Toy','Toy.json'));
%   TA = S.TA;  TN = S.TN;

    arguments
        filename (1,1) string
        opts.categoricalVars cell = {'pred','succ','noeud','ID','ID1','ID2'}
    end

    txt = fileread(filename);
    J = jsondecode(txt);
    S = struct();

    % --- helpers locaux ---
    toTable = @(x) objarray_to_table(x);
    recat   = @(T) coerce_categoricals(T, opts.categoricalVars);

    % --- Liste des clés potentielles (tables) ---
    tableKeys = {'TNodes','TArcs','TClients','TPar','TFact','TDn','TOn', ...
                 'TA','TN','TB','TC','TD','TO'};

    % --- Copie des champs "simples" (structs) si présents ---
    simpleKeys = {'pardsg','partec','dat','meta'};
    for k = simpleKeys
        key = k{1};
        if isfield(J, key)
            S.(key) = J.(key);
        end
    end

    % --- Reconstruit les tables si présentes ---
    for k = tableKeys
        key = k{1};
        if ~isfield(J, key), continue; end

        val = J.(key);

        % Si c'est déjà un struct-array JSON d'objets, faire table
        if isstruct(val) || (iscell(val) && ~isempty(val) && isstruct(val{1}))
            T = toTable(val);
        elseif istable(val)
            T = val;
        else
            % cas rare : tableau hétérogène -> tenter conversion
            try
                T = struct2table(val);
            catch
                warning('Impossible de convertir la clé %s en table. Stockée brute.', key);
                S.(key) = val;
                continue
            end
        end

        % Recategorise quelques colonnes nominales usuelles
        T = recat(T);

        % Harmonise 'loadfactor' vs 'loadFactor' si besoin
        if ismember(key, {'TA','TFact'})
            v = T.Properties.VariableNames;
            i1 = find(strcmpi(v,'loadfactor'),1,'first');
            if ~isempty(i1), T.Properties.VariableNames{i1} = 'loadFactor'; end
        end

        S.(key) = T;
    end
end

% ---------- helpers ----------

function T = objarray_to_table(x)
% Convertit un array/cell-array d'objets JSON en table MATLAB propre.
    if istable(x), T = x; return; end
    if iscell(x)
        % JSON -> cell d'objets (struct). Empile en struct-array
        x = [x{:}];
    end
    if isstruct(x)
        % struct-array -> table
        T = struct2table(x, 'AsArray', true);
    else
        error('Format JSON inattendu pour table (%s).', class(x));
    end

    % Déplie les colonnes qui seraient elles-mêmes des struct scalaires
    for i = 1:width(T)
        vi = T.(i);
        if isstruct(vi) && numel(vi)==height(T)
            try
                sub = struct2table(vi, 'AsArray', true);
                % préfixer les noms pour éviter collisions
                sub = renamevars(sub, sub.Properties.VariableNames, ...
                    strcat(T.Properties.VariableNames{i}, "_", sub.Properties.VariableNames));
                T = [T(:,1:i-1) sub T(:,i+1:end)];
            catch
                % laisse tel quel si pas convertible proprement
            end
        end
    end
end

function T = coerce_categoricals(T, names)
% Force en categorical les colonnes nominales si elles existent.
    if ~istable(T), return; end
    for j = 1:numel(names)
        v = names{j};
        if ismember(v, T.Properties.VariableNames)
            col = T.(v);
            % sur tables MATLAB, cellstr / string -> categorical
            if iscellstr(col) || (iscell(col) && all(cellfun(@(c) ischar(c) || isstring(c), col)))
                T.(v) = categorical(string(col));
            elseif isstring(col)
                T.(v) = categorical(col);
            elseif ischar(col)
                T.(v) = categorical(string(col));
            end
        end
    end
end
