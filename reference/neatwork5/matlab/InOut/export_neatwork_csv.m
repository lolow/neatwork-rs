function export_neatwork_csv(filename, TN, TA, TB, TC, TP, TF, TAH, TCP, TD, TO, TCost, TS)
% Ecrit un CSV au format NeatWork sans entête.
% Les sections sont !Nodes et !Pipes.
% Chaque ligne contient les valeurs séparées par des virgules et terminées par ",N".

    % 1) !Nodes
    write_section(filename, '!Nodes', TN, 'w');

    % 2) !Pipes
    write_section(filename, '!Pipes', TA, 'a');   

    % 3) !Default Properties
    write_section(filename, '!Default properties', TP, 'a');
    
    % 4) !loadfactor
    write_section(filename, '!loadfactor', TF, 'a');

    % 5) !Diameters
    write_section(filename, '!Diameters', TD, 'a');

    % 6) !Orifices
    write_section(filename, '!Orifices', TO, 'a');

    % 7) !Clients
    write_section(filename, '!Clients', TC, 'a');  

    % 8) !Borne
    write_section(filename, '!Borne', TB, 'a');

    % 9) !Hydraulics
    write_section(filename, '!Hydraulics', TAH, 'a');

    % 10) !ComplementTA
     write_section(filename, '!ComplementTA', TCP, 'a');

     % 11) !Costs
     write_section(filename, '!Costs', TCost, 'a');

     % Constraint satisfaction diagnostics (optional for legacy callers).
     if nargin >= 13 && ~isempty(TS)
         write_section(filename, '!ConstraintSatisfaction', TS, 'a');
     end




end

function write_section(filename, section_name, T, mode)
    fid = fopen(filename, mode);
    if fid < 0
        error('Impossible d''ouvrir le fichier: %s', filename);
    end

    fprintf(fid, '%s\n', section_name);

    % Convertir la table en cellules de chaînes
    C = table_to_string_cells(T);

    % Écrire chaque ligne + colonne finale 'N'
    nrows = size(C,1);
    for i = 1:nrows
        % line = strjoin(C(i,:), ',');
        % fprintf(fid, '%s,N\n', line);
        line = strjoin(C(i,:), ',');
        fprintf(fid, '%s\n', line);
    end

    fclose(fid);
end

function C = table_to_string_cells(T)
    Tlocal = T;
    for k = 1:width(Tlocal)
        col = Tlocal.(k);
        if isstring(col) || iscategorical(col)
            Tlocal.(k) = cellstr(col);
        elseif ischar(col)
            Tlocal.(k) = repmat({col}, height(Tlocal), 1);
        elseif isnumeric(col)
            Tlocal.(k) = arrayfun(@(x) num2str(x, '%.15g'), col, 'UniformOutput', false);
        else
            Tlocal.(k) = cellstr(string(col));
        end
    end
    C = table2cell(Tlocal);
end
