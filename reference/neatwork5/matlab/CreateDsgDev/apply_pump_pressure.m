function TN = apply_pump_pressure(TN, pardsg)
%APPLY_PUMP_PRESSURE Express downstream heights relative to source head.
% TN.haut must already be relative to the physical source elevation.
% Call once after parameter review and before enrichTablesReinf, which
% derives TA.haut and TA.pmax. pump_pressure is a head gain in metres of water.
% The source remains at zero in the internal hydraulic reference.
    pumpPressure = 0;
    if isfield(pardsg,'pump_pressure')
        pumpPressure = pardsg.pump_pressure;
    end
    validateattributes(pumpPressure, {'numeric'}, ...
        {'scalar','real','finite','nonnegative'}, mfilename,'pump_pressure');
    source = TN.statut == 0;
    assert(nnz(source)==1, 'NW5:Source', 'Une source unique est requise.');
    assert(TN.haut(source)==0, 'NW5:SourceReference', ...
        'Les hauteurs doivent deja etre relatives a la source.');
    TN.haut(~source) = TN.haut(~source) - pumpPressure;
end
