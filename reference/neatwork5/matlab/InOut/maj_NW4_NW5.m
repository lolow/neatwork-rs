% Mise à jour d'un fichier .dsg généré par NeatWork4 dans le format .dsg
% pour NeatWork5.
% Il manque au fichier *_NW4.dsg la table des clients. Lorsqu'il y a
% plusieurs clients à une même borne, l'information est donnée de manière
% implicite dans NW4: de nœuds terminaux fictifs sont rattachés à la borne
% qui les dessert par des tuyaux fictifs de longueur 1. 
% Dans *_NW5 ces informations sont données explicitement dans la table des
% clients. Il n'y a plus de nœuds fictifs pour les muulti-clients à une
% même borne.
%
% Lire le fichier issu supposément de NeatWork4. Tester si la section TC
% est présente. Si c'est le cas, ce n'est pas un fichier NW4. Sinon.
%
% 1. Charger la section Nodes. S'il y a des noeuds fictifs (repérés par la
% présence de '_a',  '_b', '_c', etc. on supprime ces nœuds pour ne garder
% que le nœud parent qui redevient terminal. Le nœud parent hérite des
% attributs de ses nœuds enfants (orifices, statut)
% 2. Charger la section !Pipes et supprimer tous les arcs fictifs joignant
% les nœuds fictifs.
% 3. Charger toutes les autres sections (Default properties, Diameters,
% Orifices, loadfactor) sous forme de tables.
% 4. Créer la table des Clients.
% 5. Enregistrer toutes les tables pour reconstituer un fichier *_NW5.dsg


clear
format compact

if exist('get_root','file')
    root = get_root();                                % .../NW_Git
else
    here = fileparts(mfilename('fullpath'));          % .../matlab/CreateDsg/ReadTpo
    root = fileparts(fileparts(fileparts(here)));     % remonte à .../NW_Gitnom
end
nom = 'Toy' ;
version = '2b_NW4';
nom2fiche = [nom,version,'.dsg']; % '_NW4.dsg'
mafiche = fullfile(root, 'examples/dsg_examples', nom, nom2fiche);

fid = fopen(mafiche,'r');  assert(fid~=-1, 'Ouverture impossible : %s', mafiche);
C = onCleanup(@() fclose(fid));
R = textscan(fid, '%s', 'Delimiter','\n', 'Whitespace','');
lines = R{1};

% assurer un vecteur de chaînes "string"
lines = string(lines(:));

% Confirmer l'absence de la section !Clients    hdr = "!" + sectionName;
hdr = '!Clients' ;
idxStart = find(startsWith(strtrim(lines), hdr), 1, 'first');
if ~isempty(idxStart)
    disp('Ce fichier contient unes section Clients. Ce n''est pas un fichier NW4.')
    return
end

% ================== NOEUDS (TN) ==================
sectionName = 'Nodes';
[block, ~, ~] = getSectionBlock(lines, sectionName);
nodeBlob  = char(join(block, newline));
nodeData  = textscan(nodeBlob, '%s %f %f %f %f %f %f', 'Delimiter', ',');
TN = table(nodeData{1}, nodeData{2}, nodeData{3}, nodeData{4}, ...
    nodeData{5}, nodeData{6}, nodeData{7}, 'VariableNames', ...
    {'noeud','haut','xdata','ydata','orifid','orifcom', 'statut'});
TN = sortrows(TN, {'statut','noeud'});


% ============ Eliminer les nœuds terminaux fictifs ===========

blk = strtrim(string(block));
parts = split(blk,",") ; 
parts = strip(parts) ;
assert(size(parts,2) > 1, "Bloc NW4 invalide: aucune colonne après le nom (pas de virgule ?)");

% Nom des nœuds dans l'actuel TN
name  = parts(:,1);

base  = regexprep(name, '_.*$', '');   % garde P2 à la place de P2_a, etc.
tail  = parts(:,2:end); % colonnes "après" le nom
ncol  = size(parts,2); 
% Forcer les éléments de ubase à garder l'ordre d'apparition dans base
% (unique réordonne les éléments par ordre croissant)
[ubase,~,ix] = unique(base,'stable');

tailRep = strings(numel(ubase), max(ncol-1,0));
for k = 1:numel(ubase)
    rows = find(ix==k);
    rowsUnd = rows(contains(name(rows), "_"));
    if ~isempty(rowsUnd)
        % les éléments '*_a', '*_b', etc. sont des nœuds fictifs tous
        % identiques. On choisit le premier pour les représenter tous
        pick = rowsUnd(1);
    else
        pick = rows(1);
    end
    tailRep(k,:) = tail(pick,:);
end

% Assemblage final : une seule ligne par base, queue conservée
merged = [ubase, tailRep];
% insertion dans TN
varNames = TN.Properties.VariableNames;     % garde les noms de colonnes
TN = array2table(merged, 'VariableNames', varNames);   % taille = size(merge)

    % ================== Arcs (TA) ==================
sectionName = 'Pipes';
[block, ~, ~] = getSectionBlock(lines, sectionName);
pipeBlob  = char(join(block, newline));
pipeData = textscan(pipeBlob, '%s %s %f %f %s %f %s %s', 'Delimiter', ',');
TA = table(pipeData{1}, pipeData{2}, pipeData{3}, pipeData{4}, ...
     pipeData{5}, pipeData{6}, pipeData{7}, pipeData{8}, 'VariableNames', ...
     {'pred','succ','lon', 'lon1', 'ID1', 'lon2', 'ID2', 'N'});

TA(contains(string(TA.succ), "_"), :) = [];


    % ================== LoadFactor TF ==================
sectionName = 'loadfactor';
[block, ~, ~] = getSectionBlock(lines, sectionName);
pipeBlob  = char(join(block, newline));
pipeData = textscan(pipeBlob, '%s %s %f %f %s %f %s %s', 'Delimiter', ',');
TF = table(pipeData{1}, pipeData{2}, pipeData{3}, 'VariableNames', ...
     {'node','lodfactor','null'});
TF(contains(string(TF.node), "_"), :) = [];

% ================== Diameters (TD) ==================
sectionName = 'Diameters';
[block, ~, ~] = getSectionBlock(lines, sectionName);
diamBlob  = char(join(block, newline));
diamData = textscan(diamBlob, '%s %s %s %f %f %f %d %f %s', 'Delimiter', ',');
TD = table(diamData{1}, diamData{2}, diamData{3}, diamData{4}, ...
     diamData{5}, diamData{6}, diamData{7}, diamData{8}, 'VariableNames', ...
     {'ID','Nominal','SDR', 'diam', 'cost', 'Pmax', 'Type', 'Roughness'});

% ================== Orifices (TO) ==================
sectionName = 'Orifices';
[block, ~, ~] = getSectionBlock(lines, sectionName);
orifBlob  = char(join(block, newline));
orifData = textscan(orifBlob, '%f %s %s %s %s %s', 'Delimiter', ',');
TO = table(orifData{1}, 'VariableNames', {'Diameter'});

% ============ PARAMÈTRES LUS (TP) =========
sectionName = 'Default properties';
[block, firstData, lastData] = getSectionBlock(lines, sectionName);
paramBlob  = char(join(block, newline));
paramData = textscan(paramBlob, '%s %f', 'Delimiter', ',');
TP = table(paramData{1}, paramData{2}, 'VariableNames', {'property','value'});

% struct param à partir de TP
param = cell2struct(num2cell(TP.value), TP.property);

if ~isfield(param, 'pressure') || isempty(param.pressure)
    param.pressure = 0 ; % default value.
end

if ~isfield(param, 'bounded_flow') || isempty(param.bounded_flow)
    param.bounded_flow = 0 ; % default value.
end

if ~isfield(param, 'uniform_dem') || isempty(param.uniform_dem)
    param.uniform_dem = 1 ; % default value from NW4.
end

f = fieldnames(param);          % cellstr, triés
vals = zeros(numel(f),1);       % supposé scalar double

for k = 1:numel(f)
    vals(k) = param.(f{k});
end

TP = table(categorical(string(f)), vals, ...
           'VariableNames', {'property','value'});


% =========== BUILD TABLE TC ===============
% TC = makeTC_from_TN(TN, param);

% Build missing table TC
nbrob = accumarray(ix,1);
nbrob = nbrob - (nbrob > 1);
borne = ubase;
nbrob = string(nbrob);

maskTerm = strcmp(TN.statut,"2");
nbrob    = double(nbrob);
nbrob(~isfinite(nbrob)) = 0;

rows = sum(nbrob(maskTerm));
% TC = table('Size',[rows 5], ...
%     'VariableTypes', {'string','string','double','double','double'}, ...
%     'VariableNames', {'client','borne','proba','cible','pressure'});
TC = table('Size',[rows 4], ...
    'VariableTypes', {'string','string','double','double'}, ...
    'VariableNames', {'client','borne','proba','cible'});

g = 0; % compteur global pour IDs C1, C2, ...
TN.statut=double(TN.statut);
for i = 1:height(TN)
    if TN.statut(i) ~= 2 || nbrob(i) <= 0, continue; end
    b = string(TN.noeud(i));        % borne (ex. "B1")
    for k = 1:nbrob(i)
        g = g + 1;
        TC.client(g)        = "C" + string(k) + b;  % ex. C4B2
        TC.borne(g)     = b;
        TC.proba(g)     = param.opentaps;
        TC.cible(g)     = param.targetflow;
        % TC.pressure(g)  = param.pressure ;
    end
end

% — Normalisation de types —
TC.client    = string(TC.client);
TC.borne = string(TC.borne);

% — 1) Unicité stricte des clients —
if numel(unique(TC.client)) < height(TC)
    error("Doublons dans TC.ID (chaque client doit être unique).");
end

% — 2) Une (et une seule) borne par client → ici: non vide
if any(strlength(TC.borne) == 0)
    error("Chaque client doit avoir exactement une 'borne' non vide.");
end

% — 3) La borne doit être un nœud terminal (statut==2) —
noeudsTerm = string(TN.noeud(maskTerm));

bad = ~ismember(TC.borne, noeudsTerm);
if any(bad)
    error("Borne(s) non terminales ou inconnues: %s", strjoin(unique(TC.borne(bad)), ", "));
end
TC = sortrows(TC,{'borne','client'}) ;

% ============== Exportation ================
nomNW5 = [nom,version,'_NW5.dsg'];
ficheNW5 = fullfile(root, 'out', nom, nomNW5);
if exist(ficheNW5,'file')==2
    delete(ficheNW5);
end

fid = fopen(ficheNW5,'w');
assert(fid>0, "Impossible d'ouvrir %s en écriture.", ficheNW5)
fclose(fid);

write_tbl(ficheNW5, 'Nodes', TN);
write_tbl(ficheNW5, 'Pipes', TA);
write_tbl(ficheNW5, 'Clients', TC);
write_tbl(ficheNW5, 'Default properties', TP);
write_tbl(ficheNW5, 'loadfactor', TF);
write_tbl(ficheNW5, 'Diameters', TD);
write_tbl(ficheNW5, 'Orifices', TO);

% helper -----------------------------------------
function write_tbl(outfile, secName, T)
% On ouvre et ferme le fichier pour y ajouter l'en-tête puis le corps de la
% section
    % 1) en-tête
    fid = fopen(outfile, 'a');  assert(fid>0);
    fprintf(fid, '!%s\n', secName);
    fclose(fid);

    % 2) section: table -> tmp, puis append dans outfile
    tmp = [tempname() '.txt'];
    writetable(T, tmp, 'FileType','text', 'Delimiter', ',', 'WriteVariableNames', false);
    fid = fopen(outfile,'a');  
    fprintf(fid, '%s', fileread(tmp));  
    fclose(fid);
    delete(tmp);
end
