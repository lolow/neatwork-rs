function save_dsg_json(jsonFile,TN,TA,TB,TC,TPar,TD,TO,TFact,TAH,TCP,pardsg,partec,reinfcost,dat,TS)
  
if nargin < 16, TS = table(); end
P = struct();
    P.schema = "NW-json-v1";  % petit tag pour versionner ton format

    % 1) Tables -> JSON-friendly
    P.TN = table2struct(prepTable(TN));
    P.TA = table2struct(prepTable(TA));
    P.TB = table2struct(prepTable(TB));
    P.TC = table2struct(prepTable(TC));
    P.TD = table2struct(prepTable(TD));
    P.TO = table2struct(prepTable(TO));
    P.TFact = table2struct(prepTable(TFact));
    P.TAH = table2struct(prepTable(TAH));
    P.TCP = table2struct(prepTable(TCP));
    P.TS = table2struct(prepTable(TS));

    % 2) Structures / divers -> JSON-friendly (conversion récursive légère)
    P.TPar   = prepAny(TPar);
    P.pardsg = prepAny(pardsg);
    P.partec = prepAny(partec);
    P.reinfcost = prepAny(reinfcost);

    % dat.M_incid = [];
    % dat.M_path = [];
    % P.dat    = prepAny(dat);

    txt = jsonencode(P, 'PrettyPrint', true);
    fid = fopen(jsonFile,'w'); assert(fid>0, "Impossible d'ouvrir %s", jsonFile);
    fwrite(fid, txt);
    fclose(fid);
end

function T = prepTable(T)
% Convert categorical columns without changing table rows or variable names.
    for k = 1:width(T)
        name = T.Properties.VariableNames{k};
        if iscategorical(T.(name))
            T.(name) = string(T.(name));
        elseif issparse(T.(name))
            T.(name) = full(T.(name));
        end
    end
end

function value = prepAny(value)
% Prepare nested parameter structures and tables for JSON encoding.
    if istable(value)
        value = prepAny(table2struct(prepTable(value)));
    elseif isstruct(value)
        names = fieldnames(value);
        for i = 1:numel(value)
            for k = 1:numel(names)
                value(i).(names{k}) = prepAny(value(i).(names{k}));
            end
        end
    elseif iscell(value)
        value = cellfun(@prepAny,value,'UniformOutput',false);
    elseif iscategorical(value)
        value = string(value);
    elseif issparse(value)
        value = full(value);
    end
end
