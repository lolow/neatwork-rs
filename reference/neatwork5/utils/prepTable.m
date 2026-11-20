function T = prepTable(T)
    % Convertit types non-JSON en représentations simples
    for v = string(T.Properties.VariableNames)
        x = T.(v);
        if iscategorical(x), T.(v) = string(x); end
        if isdatetime(x)
            % ISO 8601 (sans fuseau si non défini)
            fmt = 'uuuu-MM-dd''T''HH:mm:ss.SSS';
            T.(v) = string(x, fmt);
        end
        if isduration(x), T.(v) = seconds(x); end
        if isstring(x)
            xi = x; xi(ismissing(xi)) = ""; T.(v) = xi;  % pas de <missing> en JSON
        end
        % logical/numeric/cellstr passent en l'état
    end
end
