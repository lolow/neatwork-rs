function pardsg = review_design_parameters_reinf(pardsg, TC) ;
%REVIEW_DESIGN_PARAMETERS_REINF activates a prompt that allows the user to
% modify the majority of design parameters. The first exception is
% 'uniform_dem' that is inherited from the table of clients TC which cannot
% be changed. The dialog can allow heterogeneous unbounded demands with one
% client per terminal, but the model still rejects them. Extension deferred:
% see docs/forum/a_faire.md, section 2, and coordinate with Python simulation.
% The function includes a safeguard against a meaningless pipelength value.
% The AI's formulation is a bit superfluous.

     % Add the field 'allow_junction' if absent
    % if ~isfield(pardsg,'allow_junction')
    %     pardsg.allow_junction = 0;
    % end
    % pardsg = orderfields(pardsg) ;

    % Prescribed additional source head, in metres of water.
    if ~isfield(pardsg,'pump_pressure')
        pardsg.pump_pressure = 0;
    end

    requireBounded = pardsg.uniform_dem == 0 && ...
        numel(unique(TC.borne)) < height(TC);

    % Default values for unused options of pardsg

    par = pardsg ;
    par = rmfield(par, {'uniform_dem','fusion','limitbudget', ...
    'split','use_cvar','use_efficient_pipes','watertemp'});
    if requireBounded
        if pardsg.bounded_flow == 0
            warning('NW5:BoundedFlowRequired', ...
                ['bounded_flow changed from 0 to 1: uniform_dem=0 and ' ...
                 'multiple clients share a terminal. The current implementation ' ...
                 'requires bounded flows for this configuration.']);
        end
        par =  rmfield(par,'bounded_flow');
    end
    
    % Allow change values of fields (except uniform_dem)
    parnouv= promptParamDialog(par);

    % Record parameters after user prompt.
    champs = fieldnames(parnouv);
    for k = 1:numel(champs)
        pardsg.(champs{k}) = parnouv.(champs{k});
    end
    
    if requireBounded
        pardsg.bounded_flow = 1;
    end
    
    % update the design parameters according to the user's wishes
    parnouv.uniform_dem = pardsg.uniform_dem;
    if requireBounded
        parnouv.bounded_flow = 1;
    end
    % Add missing fields
    pardsg = parnouv ;
    pardsg.fusion = 0;
    pardsg.split = 0;
    pardsg.use_cvar = 0;
    pardsg.use_efficient_pipes = 0;
    pardsg.limitbudget = 1e8;
    pardsg.watertemp = 20;

    pardsg = orderfields(pardsg);


    assert(isnumeric(pardsg.secu) && isscalar(pardsg.secu) && ...
       isreal(pardsg.secu) && isfinite(pardsg.secu) && ...
       pardsg.secu >= 1, ...
       'NW5:Parameters', ...
       'pardsg.secu must be a finite real scalar greater than or equal to 1.');
    
    validateattributes(pardsg.pump_pressure, {'numeric'}, ...
        {'scalar','real','finite','nonnegative'}, mfilename,'pump_pressure');
    ppl = pardsg.pipelength;
    if ~(isscalar(ppl) && isfinite(ppl) && ppl >= 0.01)
        pardsg.pipelength = prompt_pipelength(ppl);
    end

end

function v = prompt_pipelength(current)
% Demande une longueur de tronçon ≥ 0.1.
% Si <0.1 / non-numérique / annulation → force v = 0.1 (et avertit).

    def = current;
    if ~(isscalar(def) && isfinite(def) && def>0),def = 0.01; end
    
    answ = inputdlg( ...
        {"Longueur 'pipelength' (doit être un nombre ≥ 0.01)"}, ...
        "Paramètre de design : pipelength", ...
        1, {num2str(def)} );
    
    if isempty(answ)
        v = 0.01;
        warning("pipelength non fourni : valeur par défaut 0.01 appliquée.");
        return
    end
    
    x = str2double(answ{1});
    if ~(isscalar(x) && isfinite(x) && x>=0.01)
        v = 0.01;
        warning("pipelength invalide (%.4g) : valeur par défaut 0.01 appliquée.", x);
    else
        v = x;
    end
    
end