function AllowedPipes = buildAllowedPipes(TA, TD)
    % BUILdALLOWEDPIPES : builds the matrix (arc × diameter) of allowed diameters
    %
    % Inputs:
    %   TA : table of links (arcs), must contain the column 'pmax'
    %   TD : table of pipes, must contain the columns 'pressure' and 'diam'.
    %
    % Output:
    %   AllowedPipes : Logical matrix (height(TA) × height(TD))

    if ~ismember('pmax', TA.Properties.VariableNames)
        error('TA doit contenir la colonne pmax');
    end
    if ~ismember('pressure', TD.Properties.VariableNames)
        error('TD doit contenir la colonne pressure');
    end

    AllowedPipes = (TA.pmax <= TD.pressure.');  % broadcasting logique
end