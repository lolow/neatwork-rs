function ctx = buildContext(pardsg)
% buildContext  Initialise la structure de contexte pour CreateDsg / Fusion
%
% Input :
%   pardsg : struct des paramètres design (doit contenir le champ 'fusion')
%
% Output :
%   ctx    : struct contexte d'exécution
%              - useFusion   : booléen, option demandée par l'utilisateur
%              - changed     : booléen, indique si des fusions ont été faites
%              - TableFusion : table des fusions (vide par défaut)
%              - log         : cell array pour des messages éventuels

    % -- sécurité : champ 'fusion' doit exister
    if ~isfield(pardsg,'fusion')
        error('pardsg doit contenir le champ "fusion" (0 ou 1).');
    end

    % -- initialisation
    ctx = struct();
    ctx.useFusion   = logical(pardsg.fusion);   % interrupteur demandé par l’utilisateur
    ctx.changed     = false;                    % mis à jour par fusion_noeuds_degre_2
    ctx.TableFusion = emptyTableFusion();       % table vide standard
    ctx.log         = {};                       % log ou diagnostics optionnels
    ctx.split    = logical(pardsg.split);    % partage des arcs avec 2 diamètres
end

function T = emptyTableFusion()
% emptyTableFusion  Crée une table vide avec la bonne structure
    T = cell2table(cell(0,6), ...
        'VariableNames', {'noeud_supprime','extremite1','long1', ...
                          'extremite2','long2','lon_total'});
end