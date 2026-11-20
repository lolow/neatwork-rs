function [TN, param] = apply_client_data(TN, TC, param)
%APPLY_CLIENT_DATA Make client data authoritative when importing a network.
% Rebuild node counts independently of the node-table format. In particular,
% DSG orifice columns must never be interpreted as client counts.
% Call before parameter review: reviewed parameters may then update TC.
    nodeIds = strtrim(string(TN.noeud));
    clientNodes = strtrim(string(TC.borne));
    [found, nodeRows] = ismember(clientNodes, nodeIds);
    assert(all(found), 'NW5:ClientNode', ...
        'Client terminals are absent from TN: %s', ...
        strjoin(unique(clientNodes(~found)), ', '));

    TN.nbrob = zeros(height(TN),1);
    if ~isempty(TC)
        TN.nbrob = accumarray(nodeRows,1,[height(TN),1]);
    end

    % A stored uniform_dem flag cannot override the clients that were read.
    param.uniform_dem = double(numel(unique(TC.proba))==1 && ...
                               numel(unique(TC.cible))==1);
    if param.uniform_dem
        param.opentaps = TC.proba(1);
        param.targetflow = TC.cible(1);
    elseif numel(unique(clientNodes)) < height(TC)
        if isfield(param,'bounded_flow') && param.bounded_flow==0
            warning('NW5:BoundedFlowRequired', ...
                ['bounded_flow changed from 0 to 1: uniform_dem=0 and ' ...
                 'multiple clients share a terminal. The current implementation ' ...
                 'requires bounded flows for this configuration.']);
        end
        param.bounded_flow = 1;
    end
end
