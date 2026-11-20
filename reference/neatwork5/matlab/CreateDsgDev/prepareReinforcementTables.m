function [TA, TD] = prepareReinforcementTables(TA, TD, partec)
%PREPAREREINFORCEMENTTABLES Prepare les tables pour le renforcement.

%
%   Cette fonction preserve les ID des
%   tuyaux reels. Elle ajoute au catalogue un tuyau d'ID "fictif" et
%   verifie la coherence de l'equipement existant.
%
%   Conventions :
%     - lon1/lon2 > 0 : le segment possede un tuyau reel ;
%     - lon1/lon2 = 0 : l'ID du segment est "fictif" ;
%     - un arc equipe couvre toute sa longueur (lon1 + lon2 = lon) ;
%     - un arc bloque (N = "B") est necessairement equipe.
%
% --- 1. Catalogue des tuyaux reels --------------------------------------
varsTD = string(TD.Properties.VariableNames);

% Compatibilite avec les anciens catalogues.
if ismember("Pmax", varsTD)
    assert(~ismember("pressure", varsTD), 'NW5:Catalogue', ...
        'TD contient a la fois Pmax et pressure.');
    TD = renamevars(TD, "Pmax", "pressure");
    varsTD = string(TD.Properties.VariableNames);
end

requiredTD = ["ID", "diam", "cost", "pressure", "Nominal", "SDR","Roughness"];
assert(all(ismember(requiredTD, varsTD)), 'NW5:Catalogue', ...
    'Le catalogue TD ne contient pas toutes les variables requises.');
assert(height(TD) > 0, 'NW5:Catalogue', ...
    'Le catalogue TD est vide.');

TD.ID = string(TD.ID);
assert(all(~ismissing(TD.ID)) && all(strlength(TD.ID) > 0), ...
    'NW5:Catalogue', 'TD contient un ID vide ou manquant.');
% Supprimer le tuyau fictif s'il est présent. On le reconstruit après.
TD(TD.ID == "fictif",:) = [];
% Vérifier que le catalogue des tuyaux réels
assert(~isempty(TD), 'Le catalogue ne contient pas de tuyaux réels')
% Unicité des tuyaux réels
assert(numel(unique(TD.ID)) == height(TD), 'NW5:Catalogue', ...
    'ID de tuyaux non uniques.');
assert(all(isfinite(TD.diam) & TD.diam > 0), 'NW5:Catalogue', ...
    'Les tuyaux réels doivent avoir un diamètre strictement positif et fini.');

% Resistance lineaire et conductance des tuyaux reels.
TD.resist = partec.bescal * TD.diam.^(-partec.nu);
TD.cond   = TD.resist.^(-1/partec.lambda);

% --- 2. Ajout du tuyau fictif --------------------------------------
% Les colonnes Nominal et SDR conservent leur type initial.
% Comme dans la version fournie, elles doivent accepter "fictif".

row = TD(1,:);
row.ID       = "fictif";
row.Nominal  = "fictif";
row.SDR      = "fictif";
row.diam     = 0;
row.cost     = 0;
row.pressure = 1000;
row.resist   = inf;
row.cond     = 0;
row.Roughness     = 0;


TD = [TD; row];
TD = sortrows(TD, 'diam');




% --- 3. Description de l'equipement existant ----------------------------
varsTA = string(TA.Properties.VariableNames);
assert(ismember("lon", varsTA), 'NW5:MissingEquipment', ...
    'La variable TA.lon est absente.');

fields = ["lon1", "ID1", "lon2", "ID2"];
present = ismember(fields, varsTA);

if ~any(present)
    % Reseau vierge : tous les arcs sont initialement non equipes.
    TA.lon1 = zeros(height(TA), 1);
    TA.ID1  = repmat("fictif", height(TA), 1);
    TA.lon2 = zeros(height(TA), 1);
    TA.ID2  = repmat("fictif", height(TA), 1);
else
    assert(all(present), 'NW5:MissingEquipment', ...
        'Description incomplete de l''equipement : lon1, ID1, lon2, ID2 requis.');
    TA.ID1 = string(TA.ID1);
    TA.ID2 = string(TA.ID2);
end

% Les longueurs doivent etre definies, finies et non negatives.
assert(all(isfinite(TA.lon)  & TA.lon  >= 0) && ...
       all(isfinite(TA.lon1) & TA.lon1 >= 0) && ...
       all(isfinite(TA.lon2) & TA.lon2 >= 0), ...
       'NW5:EquipmentLength', ...
       'Les longueurs lon, lon1 et lon2 doivent etre finies et non negatives.');

% Un segment de longueur positive doit avoir un tuyau reel ;
% un segment de longueur nulle doit avoir l'ID "fictif".
assert(~any(TA.lon1 > 0 & TA.ID1 == "fictif"), ...
    'NW5:EquipmentID', ...
    'Une longueur positive sur le 1er segment exige un tuyau réel.');
assert(~any(TA.lon2 > 0 & TA.ID2 == "fictif"), ...
    'NW5:EquipmentID', ...
    'Une longueur positive sur le 2e segment exige un tuyau réel.');

bad1 = TA.lon1 == 0 & TA.ID1 ~= "fictif";
assert(~any(bad1), 'NW5:EquipmentID', ...
    ['Un 1er segment de longueur nulle ne peut porter un tuyau reel. ' ...
     'Verifier ID1 ou lon1.']);

bad2 = TA.lon2 == 0 & TA.ID2 ~= "fictif";
assert(~any(bad2), 'NW5:EquipmentID', ...
    ['Un 2e segment de longueur nulle ne peut porter un tuyau reel. ' ...
     'Verifier ID2 ou lon2.']);

% Tous les ID doivent exister dans le catalogue, tuyau fictif inclus.
assert(all(ismember(TA.ID1, TD.ID)), 'NW5:UnknownPipe', ...
    'ID1 contient un identifiant absent de TD.');
assert(all(ismember(TA.ID2, TD.ID)), 'NW5:UnknownPipe', ...
    'ID2 contient un identifiant absent de TD.');

% --- 4. Statut des arcs et coherence globale ----------------------------
if ~ismember("N", varsTA)
    TA.N = repmat("N", height(TA), 1);
end
TA.N = string(TA.N);
assert(all(ismember(TA.N, ["N", "B", "R"])), ...
    'NW5:EquipmentStatus', ...
    'TA.N doit valoir "N" (nouveau), "B" (bloque) ou "R" (renforce).');

equipped = TA.lon1 > 0 | TA.lon2 > 0;
ecart = TA.lon1(equipped)+ TA.lon2(equipped) - TA.lon(equipped);
assert(all(abs(ecart) > 1e-7), ...
    'NW5:EquipmentLength', ...
    'Pour tout arc equipe, lon1 + lon2 doit etre egal a lon.');

blocked = TA.N == "B";
assert(~any(blocked & ~equipped), 'NW5:EquipmentStatus', ...
    'Un arc sans conduite reelle ne peut etre bloque.');

TA.equip = equipped;
end