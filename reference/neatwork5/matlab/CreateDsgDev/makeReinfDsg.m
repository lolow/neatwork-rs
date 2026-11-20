% % Renforcer un design à partir d'un fichier .dsg. Le module peut
% s'appliquer à un fichier .tpo en considérant que les arcs nus sont
% équipés d'un tuyau inopérant (diamètre intérieur quasi nul).

% TN.haut preserves physical elevations throughout design and export.
% enrichTablesReinf expresses TA.haut relative to the source head.
% pump_pressure is a prescribed source head gain in metres of water.

% Donner le nom et la version du réseau.
% Le fichier tpo se trouve dans le dossier NW-Git/examples/'nom'/
% Le résultat est stocké dans le dossier NW-Git/out/'nom'/ sous le nomv qui
% est le mom du réseau complété par la version et suivi de _Secu avec la
% valeur du coefficient de sécurité (>= 1).
% Les infos sur les runs relatifs à l'example sont stockés dans 
% NW-Git/out/'nomv'/run
% MakeDesign traite plusieurs configurations de design définis par les 
% demandes des clients et leur comportement.
% A- Dans NeatWork4, les clients sont tous identiques (débit demandé et
% probabilité d'ouverture). Il y a une perte de charge au robinet et pas de
% limitation de débits. Il peut y avoir un ou plusieurs client par borne.
% pardsg.uniform_dem = 1 et pardsg.bounded_flow = 0;
% B- NeatWork4+. Le débits sont bornés à la valeur demandée. Le débit
% sont assurés si la pression à la borne est au moins égale à la pression
% de consigne pardsg.pressure. Par ailleurs pardsg.uniform_dem = 1 et 
% pardsg.bounded_flow = 1;
% C- NeatWork5 (irrigation). Les clients sont diversifiés et les débits
% sont limités par les valeurs demandées. Le cas C généralise le cas B
% pardsg.uniform_dem = 0 et pardsg.bounded_flow = 1;
% D- C'est le cas C, sans contrainte sur les débits
% (pardsg.bounded_flow = 0). Ce cas peut être traité s'il n'y a qu'un client
% par borne. Il suffit d'introduire des arcs fictifs reliant une borne
% commune à chacun des clients attachés à cette borne. En plus d'une
% modificaiton de la topologie, il faut faire un petit ajustement dans le
% code pour tenir compte de la non-uniformité des clients. A faire si
% le besoin se fait sentir.
 
clear 
format compact

% ========= Choose file =========
% General information for tpo or dsg input files
nom = 'Toy';  % ou 'Red 1', 'Toy', 'ToyXlx'
version = "2b"; % 4design, 3b, 5,2d,'DevReinf'
input_type = 'tpo';
if ~ismember(string(input_type), ["tpo", "dsg"])
    error(['The file must have suffix .tpo or .dsg. ' ...
        'Support for suffix .json should be added in future developments.']);
end
output_type = 'json' ; % dsg or json

% Complement if output is a dsg file
if strcmp(input_type,'dsg')
    secu    = [] ; % Put [] if not specified. Else number >= 1
    fusion  = [] ; % Put [] if not specified. Else 0 or 1
    split   = [] ;
    % lieu    = 'examples/dsg_examples'; % nom du dossier parent. Par défaut out. Pour débogage 'examples'.
    lieu    = 'examples/dsg_examples'; % nom du dossier parent. Par défaut out. Pour débogage 'examples'.
end

% ========= Read the file and generate tables =========
if strcmp(input_type,'tpo')
    % Compose adresses to retrieve the .tpo file and to print the  .dsg file
    nomv = sprintf('%s%s',nom,version) ;

    % Put into structure dat
    dat.nom = nom;
    dat.version = version ;
    dat.nomv = nomv ;
    dat.output_type = output_type ;

    % --- Read the tpo file and generate my Tables ---
    [TN, TA, TC, TB, TP, TD, TO, partec, pardsg] = parseTpoToTables(dat);

elseif  strcmp(input_type,'dsg')
    % --- Read the dsg file and generate my Tables ---
    [TA,TN,TC,TB,TD,TO,TP,param,dat,pardsg,partec] = readDesign(nom,version,secu,fusion,split,input_type,lieu);
    dat.output_type = output_type ;

    % Make sure that the arc to be reinforced has only one type of pipe 
    % [TN_out, TA_out] = split_arcs_with_2_pipes(TN, TA, partec);
end

% Store the physical source elevation without changing the node table.
source = TN.statut == 0;
assert(nnz(source)==1, 'NW5:Source', 'Une source unique est requise.');
dat.source_height = TN.haut(source);

% ======== Complete tables TA and TD (new) ========
[TA,TD] = prepareReinforcementTables(TA,TD,partec);

% Change variable name
if ismember('Pmax', TD.Properties.VariableNames)
    TD.Properties.VariableNames{'Pmax'} = 'pressure';
end

% === Check validity of TA and TN ===
% Utilitaire dans InOut. Test de conformité.
[TN, TA] = validate_and_align_Tables(TN, TA, TB) ;

% === Check whether network is a tree. Reorient from root to leaves ===
[TA, TN, rapport] = peel_orient_tree(TN, TA);

if ~rapport.is_tree || rapport.has_cycle || rapport.has_parallel || rapport.source_degree ~= 1
    error('NW5:InvalidDesignTopology', 'Conception interrompue : %s', rapport.reason);
end

% Future extension (if needed): reduce existing parallel pipes to a virtual
% tree by replacing each parallel group with one equivalent-resistance arc.
% Keep a mapping to the physical pipes for output reconstruction. Equivalent
% resistance could be stored directly (as in TR.rexist); alternatively, use
% synthetic TD entries excluded from reinforcement choices. Leave this design
% decision to future developers; parallel networks remain outside current scope.

% === Review parameters values ===
pardsg = review_design_parameters_reinf(pardsg, TC) ;

% After review, the chosen uniform settings update the client table.
if pardsg.uniform_dem == 1
    TC.proba = ones(height(TC), 1) * pardsg.opentaps;
    TC.cible = ones(height(TC), 1) * pardsg.targetflow;
end

% === Warning if Case D is selected ===
% Can be omitted. To remember in a production version
% classCase = classify_case(TA,TN,TC,pardsg);

% === Enrich tables ===
% Transfer node information to arcs.
[TA, TB, TN, TD] = enrichTablesReinf(TA, TB, TN, TD, pardsg, dat.source_height);

% === Check the existing pressure classes before estimating any flows ===
validate_existing_pressure(TA, TD);

% === Create table of equivalent resistances ===
TR = equivResist(TA,TN,TD,partec);

% === Build a context function ===
% Capture the user's fusion option. Can be extended to other options
 ctx = buildContext(pardsg) ;

% === Merge arcs that are adjacent to the same node of degree 2 ===
% Structure ctx pour passer l'info sur l'éventuelle fusion
% ADAPTER POUR TENIR COMPTE DE L'EXISTANT. ON PEUT FUSIONNER SEULEMENT SI
% LES 2 TRONÇONS ONT LE MÊME EQUIPEMENT.
% [TA, TN, ctx] = fusion_noeuds_degre_2

% Fusion and splitting — deferred design decision: These operations should 
% be handled by the application interface, with explicit user interaction. 
% Their expected behavior remains to be clarified with field colleagues. 
% During the Python port, do not infer that these operations are 
% implemented merely because related options or metadata exist. 
% Revisit their scope and integration once the workflow has been agreed upon.

% === Graph construction (topology-dependent, algorithm-independent) ===
dat = build_graph_artifacts(TA, TN, dat);

% === Design heuristic (NW4-style load estimate) ===
% Load for the safety factor parsdg.sevicequal
[TA, TL, dat] = build_incidence_loads(TA, TN, TC, pardsg, dat);
% Load for the safety factor parsdg.vmax_secu 
par = pardsg;
par.servicequal = pardsg.vmax_secu ;
[~, TLv, dat] = build_incidence_loads(TA, TN, TC, par, dat);
% Speed constraints must use loads at least as large as the design loads.
TLv{:,2:end} = max(TLv{:,2:end}, TL{:,2:end});


% === Build MILP model ===
model = build_reinf_model(TA, TD, TL, TLv, TR, pardsg, partec, dat);

% === Solve MILP model ===
displayMode = "final";
sol = solveModel(model, displayMode);

% === ProcessSolution ===
[TA,TN,TCP,TAH,reinfcost,ctx,TS] = processSol(TA,TN,TD,TC,TO,TL,TR,dat,pardsg,partec,model,sol,ctx);

% === Write the result file in the appropriate format ===
filename = data4DsgFile(TA,TB,TN,TC,TD,TO,TL,TAH,TCP,pardsg,partec,reinfcost,dat,TS);
if isempty(filename)
    disp('Filemane is empty. No export. Results ares saved in the workspace.');
    return
end

% === Trace operations ===
meta.schemaVersion = "NW_design_meta_v1";
meta.network       = string(dat.nom);
meta.version       = string(dat.version);
meta.date          = string(datetime('now','Format','yyyy-MM-dd''T''HH:mm:ss''Z'''));
meta.useFusion     = logical(ctx.useFusion);
meta.nFusions      = height(ctx.TableFusion);
meta.useSplit      = logical(ctx.split);
meta.nbsplit       = ctx.nbsplit ;

if all(isfield(pardsg,{'vmax','secu','uniform_dem','bounded_flow','pressure'}))
    meta.vmax = pardsg.vmax;
    meta.secu = pardsg.secu;
    meta.uniform_dem = pardsg.uniform_dem;
    meta.bounded_flow = pardsg.bounded_flow;
    meta.pressure = pardsg.pressure;
end
if exist('sol','var')
    meta.solver = struct( ...
        'message',   sol.message, ...
        'fval', sol.fval);
        % 'fval', sol.fval, ...
        % 'iter',   sol.iter );
end
if ctx.useFusion && ~isempty(ctx.TableFusion)
    meta.traceFile = sprintf('%s_TableFusion.csv', dat.nomv);
end

root   = get_root();
outdir = fullfile(root, 'out', dat.nom);
if ~exist(outdir,'dir'); mkdir(outdir); end

% write_meta(fullfile(outdir, sprintf('%s%s.meta.json', dat.nom, dat.version)), meta);

base = dat.nom ;
fname = write_run_meta(outdir, base, meta);

% ------------------------------

function fname = write_run_meta(outdir, base, meta)
    if ~exist(fullfile(outdir,'runs'),'dir'); mkdir(fullfile(outdir,'runs')); end
    ts   = datetime("now");
    fname = fullfile(outdir,'runs', sprintf('meta_%s_%s.json', base, ts));
    write_meta(fname, meta);
end
