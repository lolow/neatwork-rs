function param = review_simul_parmeters(param) ;
% review_design_parmeters activates a prompt that allows the user to
% modify the majority of design parameters. The first exception is
% 'uniform_dem' that is inherited from the table of clients TC which cannot
% be changed. Moreover, if pardsg.uniform_dem = 0, the sole instance
% handled at this stage of the code's evolution is the default value
% pardsg.bounded_flow = 1.
% The function includes a safeguard against a meaningless pipelength value.
% The AI's formulation is a bit superfluous.

    
    param.nbsim = 100; % default value
    param.orif_com = 0; % default value. Ideal orifices will be used
    % Fields to be edited
    
    if param.uniform_dem == 0
        wanted = ["nbsim","pressure"] ;
        param.bounded_flow = 1;
        % wanted = [wanted,"pressure"];
    else
        wanted = ["nbsim","bounded_flow","opentaps","targetflow","orif_com"];
    end

    wanted = string(wanted(:)).';
    par = struct();
    for f = wanted
        if isfield(param, f), par.(f) = param.(f); end
    end

    parnouv= promptParamDialog(par);

    f = fieldnames(parnouv);
    for k=1:numel(f)
        if isfield(param, f{k})
            param.(f{k}) = parnouv.(f{k});
        end
    end

    % If param.uniform=0 and param.bounded_flow =1, need to review pressure
    if param.uniform_dem == 1 && param.bounded_flow == 1;
        wanted = "pressure";
        par = struct();
        par.pressure = param.pressure;
        parnouv= promptParamDialog(par);
        param.pressure = parnouv.pressure;
    end

end


% 
%     par = pardsg ;
%     par = rmfield(par,'uniform_dem');
%     if pardsg.uniform_dem == 0
%         par =  rmfield(par,'bounded_flow');
%     end
% 
%     % Allow change values of fields (except uniform_dem)
%     parnouv= promptParamDialog(par);
% 
%     % update the design parameters according to the user's wishes
%     parnouv.uniform_dem = pardsg.uniform_dem;
%     if pardsg.uniform_dem == 0
%         parnouv.bounded_flow = 1;
%     end
%     pardsg = parnouv ;
% 
%     pardsg.pipelength = prompt_pipelength(pardsg.pipelength);
% 
% end
% 
% function v = prompt_pipelength(current)
% % Demande une longueur de tronçon > 0.
% % Si 0 / non-numérique / annulation → force v = 0.1 (et avertit).
% 
%     def = current;
%     if ~(isscalar(def) && isfinite(def) && def>0), def = 0.1; end
% 
%     answ = inputdlg( ...
%         {"Longueur 'pipelength' (doit être > 0)"}, ...
%         "Paramètre de design : pipelength", ...
%         1, {num2str(def)} );
% 
%     if isempty(answ)
%         v = 0.1;
%         warning("pipelength non fourni : valeur par défaut 0.1 appliquée.");
%     end
% 
%     x = str2double(answ{1});
%     if ~(isscalar(x) && isfinite(x) && x>0)
%         v = 0.1;
%         warning("pipelength invalide (%.4g) : valeur par défaut 0.1 appliquée.", x);
%     else
%         v = x;
%     end
