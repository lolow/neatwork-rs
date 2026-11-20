function [pardsg,partec] = extract_parameters(param)
%EXTRACT_PARAMETERS builds two parameters substructures out orf param
%  Missing fields are created and default values are assigned

    % -------- partec (constantes techniques) --------
    partec = struct();
    partec.orifcoef   = mustfield(param, 'orifcoef',   1.0);
    partec.faucetcoef = mustfield(param, 'faucetcoef', 1.0);
    partec.lambda     = mustfield(param, 'lambda',   1.7810);
    partec.nu         = mustfield(param, 'nu',       4.7810);
    partec.bescal     = mustfield(param, 'bescal', 8.9396e-04);

    partec = orderfields(partec);

    % -------- pardsg (paramètres design) ------------
    % NOTE (NW4 compatibility)
    % ------------------------
    % The parameter 'limitbudget' is kept only for backward compatibility with
    % the NW4 Java implementation. It is not used in the NW5 design algorithm.
    % This parameter may disappear in the future Python implementation.
    % The same for 'seuil' and 'watertemp'. Other parameters may be necessary
    % in the future concerning the formula for friction in pipes.
    pardsg = struct();
    pardsg.targetflow   = mustfield(param,'targetflow', 0.0);
    pardsg.opentaps     = mustfield(param,'opentaps',   1);
    pardsg.pump_pressure = mustfield(param,'pump_pressure',0); % m of water
    pardsg.pressure     = mustfield(param,'pressure',   2);
    pardsg.secu         = mustfield(param,'secu',   1.0);
    pardsg.vmax         = mustfield(param,'vmax',   1.5);
    pardsg.vmax_secu    = mustfield(param,'vmax_secu',0.95);
    pardsg.servicequal  = mustfield(param,'servicequal',0.65);
    pardsg.fusion       = mustfield(param,'fusion',      0);
    pardsg.split        = mustfield(param,'split',      0);
    pardsg.bounded_flow ...
        = mustfield(param,'bounded_flow', 0);
    pardsg.pipelength   = mustfield(param, 'pipelength',   6.0);
    pardsg.seuil        = mustfield(param,'seuil',      0.0);
    pardsg.limitbudget  = mustfield(param,'limitbudget',inf);
    pardsg.watertemp    = mustfield(param,'watertemp',  20.0);
    pardsg.uniform_dem  = mustfield(param,'uniform_dem',  1);
    pardsg.use_efficient_pipes = mustfield(param,'use_efficient_pipes',  0);
    pardsg.nScenLoad    = mustfield(param,'nScenLoad',200000);
    pardsg.use_cvar     = mustfield(param,'use_cvar',0); % Empirical or binomial

    pardsg=orderfields(pardsg);




end
