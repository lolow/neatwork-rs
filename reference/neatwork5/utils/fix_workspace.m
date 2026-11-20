function fix_workspace()
%FIX_WORKSPACE  Tentative de récupération d'un Workspace MATLAB invisible.
%
%  Cette fonction applique une série d'actions connues pour "réveiller"
%  l'interface graphique (Workspace, Layout) lorsque :
%   - la fenêtre Workspace est vide
%   - whos montre pourtant des variables
%   - le desktop MATLAB est dans un état incohérent
%
%  Actions effectuées :
%   1) restoredefaultpath
%   2) rehash toolboxcache
%   3) tentative de restauration du layout standard
%
%  Usage :
%     fix_workspace
%
%  (fonction volontairement sans arguments)

    restoredefaultpath
    rehash toolboxcache

    % Tentative de rafraîchissement du desktop (non critique si échec)
    try
        desktop = com.mathworks.mde.desk.MLDesktop.getInstance;
        desktop.restoreLayout('Two Column');
    catch
        % rien : dépend de la version / état du desktop Java
    end

    disp('[fix_workspace] Workspace refresh attempted.');
    disp('[fix_workspace] restoredefaultpath + rehash toolboxcache executed.');
end
