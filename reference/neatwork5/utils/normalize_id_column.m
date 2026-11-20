function ids = normalize_id_column(x, varargin)
% NORMALIZE_ID_COLUMN  Force une colonne d'identifiants en string propre.
% - Accepte: string, char array (N-by-M), cellstr, categorical, numeric.
% - Nettoie: trim espaces, option upper/lower, remplace <missing>.
% Options: 'Case' = 'as-is'|'upper'|'lower' (default 'as-is')
%          'MissingPolicy' = 'error'|'drop'|'fill' (default 'error')
%          'FillWith' = "" (si MissingPolicy='fill')

    p = inputParser;
    addParameter(p,'Case','as-is');
    addParameter(p,'MissingPolicy','error');
    addParameter(p,'FillWith',"");
    parse(p,varargin{:});
    opt = p.Results;

    % 1) Convertir en string de façon robuste
    if isstring(x)
        ids = x;
    elseif iscellstr(x) || (iscell(x) && all(cellfun(@ischar,x)))
        ids = string(x);
    elseif ischar(x)                     % char N-by-M -> cellstr -> string
        ids = string(cellstr(x));
    elseif iscategorical(x)
        ids = string(x);                 % udefs -> <missing>
    elseif isnumeric(x)
        ids = string(x);                 % élément par élément
    else
        error('Type non supporté pour IDs: %s', class(x));
    end

    % 2) Trim espaces
    ids = strtrim(ids);

    % 3) Normalisation de casse (optionnelle)
    switch lower(opt.Case)
        case 'upper', ids = upper(ids);
        case 'lower', ids = lower(ids);
        case 'as-is'
        otherwise, error('Case doit être as-is|upper|lower');
    end

    % 4) Gestion des manquants
    miss = ismissing(ids);  % vrai pour "" ou <missing>
    if any(miss)
        switch lower(opt.MissingPolicy)
            case 'error'
                error('IDs contiennent des valeurs manquantes (%d).', nnz(miss));
            case 'drop'
                ids = ids(~miss);
            case 'fill'
                ids(miss) = opt.FillWith;
            otherwise
                error('MissingPolicy doit être error|drop|fill');
        end
    end
end
