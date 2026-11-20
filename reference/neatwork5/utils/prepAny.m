function y = prepAny(x)
    % Conversion récursive très simple pour struct/cell/sparse/datetime/categorical
    if istable(x)
        y = table2struct(prepTable(x));
    elseif isstruct(x)
        y = struct();
        f = string(fieldnames(x));
        for k = 1:numel(f)
            y.(f(k)) = prepAny(x.(f(k)));
        end
    elseif iscell(x)
        y = cellfun(@prepAny, x, 'UniformOutput', false);
    elseif issparse(x)
        [i,j,v] = find(x);
        y = struct('type','sparse','n',size(x,1),'m',size(x,2),'i',i,'j',j,'v',v);
    elseif iscategorical(x)
        y = string(x);
    elseif isdatetime(x)
        y = string(x, 'uuuu-MM-dd''T''HH:mm:ss.SSS');
    elseif isduration(x)
        y = seconds(x);
    else
        y = x; % numeric, logical, string, char…
    end
end