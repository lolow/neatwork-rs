function param = promptParamDialog(param)
% Ouvre une boîte de dialogue avec l'utilisateur pour revoir les valeurs
% des paramètres de design

    fields = fieldnames(param);
    n = numel(fields);

    % Prépare les labels et les valeurs par défaut
    prompts = cell(n,1);
    defaults = cell(n,1);

    for i = 1:n
        fname = fields{i};
        prompts{i} = fname;
        val = param.(fname);
        if isnumeric(val)
            defaults{i} = num2str(val);
        else
            defaults{i} = char(string(val));
        end
    end

    % Ouvre une boîte de dialogue
    answer = inputdlg(prompts, 'Paramètres', 1, defaults);

    if ~isempty(answer)
        for i = 1:n
            if isnumeric(param.(fields{i}))
                newval = str2double(answer{i});
                if ~isnan(newval)
                    param.(fields{i}) = newval;
                end
            else
                param.(fields{i}) = answer{i};
            end
        end
    end
end
