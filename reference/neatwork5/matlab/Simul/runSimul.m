% Simuler un design à partir d'un fichier .dsg et stocker le résultat dans un
% fichier .sim
% Donner le nom,la version du réseau et le coefficient de sécurité
% Le fichier dsg se trouve dans le dossier NW-Git/out/'nom'/ sous le nomv
% qui est le mom du réseau complété par la version et suivi de _Secu avec
% la valeur du coefficient de sécurité (>= 1).
% 
% 1. CREER le nom et l'adresse du fichier de données de design. Ces
% éléments contiennent le type du fichier (.dsg, .mat ou.json).
% 2. RECUPERER le fichier par son adresse. On extrait les tables TA, TN,
% TC. Puis TD (base de données des diamètres), TO (base de données des
% orifices).  Les paramètres sont récupérés à partir de TD (cas d'un fichier
% dsg) et transformé en structure param, soit donnés directement en
% structures (cas .mat et .json). Créer une structure dat
% 3. COMPLETER les éléments manquants.
% Remarque : la base de données TO n'est pas utile pour la simulation

clear 
format compact

% ========= Nom et adresse du fichier de données =========
nom = 'Toy';  % ou 'Red 1', 'Toy', 'Red 1_NW5', Cruscade, ToyXlx, ToyTxt
version = '3b' ; % Det2406
secu    = [] ; % Put [] if not specified. Else number >= 1
fusion  = [] ; % Put [] if not specified. Else 0 or 1
split   = [] ;
type    = 'dsg' ; % 'mat' % dsg % json % txt
% lieu    = 'examples/dsg_examples'; % nom du dossier parent. Par défaut out. Pour débogage 'examples'.
lieu    = 'out'; % nom du dossier parent. Par défaut out. Pour débogage 'examples'.

% === Lecture du fichier de données ====
[TA,TN,TC,TB,TD,~,TP,param,dat] = readDesign(nom,version,secu,fusion,split,type,lieu);

% ===== Generate options for the simulation =====
param = review_simul_parmeters(param) ;
if param.uniform_dem == 1 && param.bounded_flow == 1
    TB.pressure = ones(height(TB),1) * param.pressure ;
end

% === Generate missing elements in the tables ===
[TA, TN, TD, param] = completeTables(TA, TN, TD, param);

% ======= Generate structures for the simulation ======
[dat,options] = complete_data(TA,TN, dat, param);

% ======= Générer les tables de scénarios ========
[demande_borne,clients_par_borne,simRes] = genscen(TC,dat,param) ;

% ====== Simulation des débits ======
% profile on
tic
itesim = 0;
while itesim < param.nbsim
    itesim = itesim + 1;
    % ---- Create the structure work for the current scenario -----
    work = prepare_sim(TA,TB,dat,param,demande_borne,clients_par_borne,itesim) ;

    % ---- Compute flows and related for the current scenario -----
    work = debits(work,dat,param,options);

    % ---- Save results in structure simRes ---
    % indicateur des bornes sélectionnées dans le scénario courant.
    % work.open est l'indicateur des arcs actifs
    ouvT = work.open(dat.idAter);
    simRes.X(work.open,itesim) = work.x; % débits dans l'ensemble des arcs
    simRes.Ouv(:,itesim) = work.open ;
    simRes.Iter(itesim) = work.iter ;
    simRes.Charge(work.open,itesim) = work.charge; %- dat.M_incid(:,work.open)' * work.u;
    simRes.Alpha(work.open,itesim) = work.alpha ;
    simRes.U(:,itesim) = work.u ;
    % simRes.xsup = zeros(dat.nbA,1) ;
    % simRes.xsup(work.open) = work.xsup ;
    simRes.dem_tot(itesim) = sum(demande_borne(:,itesim)) ;
    if param.bounded_flow
        simRes.Satis(ouvT,itesim) =  work.x(work.ArcT) ./ work.xsup(work.ArcT) ;
    else
        simRes.Satis(ouvT,itesim) =  work.x(work.ArcT) ./ work.clients...
            / param.targetflow / 1e-3 ;
    end
end
temps=toc;
simRes.TempsIPM = temps;
% profile viewer
% profile off
% ===== Compute statistics out of simRes =====
stat_processing(itesim,simRes,param,dat,TN,TA);




