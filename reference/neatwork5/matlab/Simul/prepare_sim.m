function work = prepare_sim(TA,TB,dat,param,demande_borne,clients_par_borne,k)

% Définir les données d'entrée pour la fonction debit spécifiques au
% scénario courant (le kième généré).
% Les données fournies sont
%   - Le numéro k de la simulation
%   - Les clients et leurs demandes aux bornes
%   - Les données du réseau et les paramètres de design
% Les données calculées sont
%   - Les bornes actives caractéréisées par une demande effective non nulle
%   - Le paramètre alpha (perte de charge aux bornes) qui dépend des
%   consigens de design.
%   - Le paramète beta pour tous les arcs.
%   - Les limites sur les débits sur chacun des arcs.

    work = struct(); % structure contenant les infos pour la simulation
    
    dem = demande_borne(:,k) * 1e-3; % Exprimer en m^3/s
    clients = zeros(height(TA),1);
    clients(dat.idAter) = clients_par_borne(:,k);
    
    
    % Indicateur des bornes actives dans l'ensemble des bornes
    ouvTer = dem > 0;
    
    % indicateur des arcs porteurs de débit
    idactif = true(height(TA),1);
    idactif(dat.idAter) = ouvTer;
    
    % indicateur des arcs terminaux (bornes) et  qui sont bornes actives.
    idTactif = idactif ;
    idTactif(~dat.idAter) = false;
    
    % nombre de clients et demande cumulée par borne active
    nbClients = clients_par_borne(ouvTer,k);
    dem_positive = dem(ouvTer);
    
    % ======== A- Calcul des alphas =========
    % Les coefficients de friction alpha sont nuls sauf pour les bornes
    % actives.
    
    % --- alpha calculés pour les terminaux avec orifice ---
    if param.uniform_dem == 1 && param.bounded_flow == 0
        % Cas NW4 avec un ou plusieurs robinets par terminal. Les alphas
        % ont été calculés à partir du diam des orifices et du du
        % coefficient de friction des robinets param.rob. On s'assure
        % d'abord que ce calcul a bien été fait
         requis = {'alpha_com' 'alpha_ideal'};
        if ~all(ismember(requis, TA.Properties.VariableNames))
            msg = ['Critical error: missing data.\n' ...
                'TA must have variables related to orifices and related alphas'...
                ' when the design has uniform demand and bounded flow.']
            error(sprintf(msg))
        end

        % Les alphas ont été calculés pour la perte de charge au robinet en
        % fonction du débit cible. S'il y a n robinets ouverts à un
        % terminal, le débit cible au terminal est n*cible. Le alpha
        % correspondant au terminal est le alpha aux robinets individuelss
        % divisé par n^2
        alpha = zeros(height(TA),1);
        if param.orif_com == 1
            alpha(idTactif) = TA.alpha_com(idTactif)./ clients(idTactif).^2;
        else
            alpha(idTactif) = TA.alpha_ideal(idTactif)./ clients(idTactif).^2;
        end
        alpha(~idTactif) = 0;
 
    
    elseif param.uniform_dem == 0 && param.bounded_flow == 0
        % A faire : traiter les clients homogenes au sein de chaque borne,
        % mais pouvant differer entre bornes. Un client par borne est un cas
        % particulier ; les cibles restent des objectifs, sans plafonnement.
        % Adapter l'equipement equivalent et la satisfaction avec la personne
        % chargee de la simulation Python (docs/forum/a_faire.md, section 2).
        % Le refus actuel est maintenu en attendant cette evolution coordonnee.
        error('Ce cas n''est pas (encore) traité')
    else
        % Il reste deux cas possibles. On a bounded_flow=1 et uniform_dem
        % vaut 0 ou 1. On est dans le cas irrigation.
        % du type irrigation ; et un seul cas si bounded_flow=0 qui est
        % traité comme NW4 avec plusieurs robinets.
        % Les clients ont un contrat de pression et de demande. Il n'y a
        % plus d'orifice par client, mais une contrôle de pression à la
        % borne.
        % Initialiser alpha
        alpha = zeros(height(TA),1);
        % Calculer les alphas aux bornes

        % EST-CE QUE LA PRESSION EST DONNEE PAR TB OU PAR PARAM ?

        alpha(idTactif) = param.pressure ./ dem_positive.^2 ;
        % alpha(idTactif) = ((TA.orif_ideal(idTactif)/0.59).^(-4) + ...
            % 1/param.faucetcoef) ./ nbClients .^2;
    end
    
    % Valeur de alpha pour les variables actives dans débit
    work.alpha = alpha(idactif);
    
    % ======== B- Calcul des betas =========
    % récupérés de TA
    work.beta = TA.beta(idactif);
    
    
    
    % ==== C- Contraintes de boîtes sur les variables ===
    if param.bounded_flow == 1
        % sum(dem) est, en valeur absolue, une borne sur le débit pour tout arc
        xsup = sum(dem) * ones(dat.nbA,1) ;
        xinf = - xsup ;
        % Arcs terminaux
        xsup(dat.idAter) = dem ;
    else 
        xsup = 20 * sum(dem) * ones(dat.nbA,1) ;
        xinf = - xsup ;
        % Il n'y a pas de borne sur les débits aux bornes
        xsup(dat.idAter) = 20 * dem;
    end
    
    % borne inférieure aux terminaux et aux stations
    xinf(dat.idAter) = 0 ;
    xinf(dat.idAsrc) = 0 ;

    % % Arcs de transit
    % xsup(dat.idAtra) = sum(dem) ;
    % xinf(dat.idAtra) = - sum(dem) ;
    % % Arcs avec station
    % xinf(dat.idAsrc) = - sum(dem) ;
    
    work.xsup = xsup(idactif);
    work.xinf = xinf(idactif);
    
    % ---- Eliminer les arcs fermés ---
    % Le cas EPANET où la demande à une borne est fixée à une valeur
    % non-nulle n'est pas (encore) traitée. Dans la présente modélisation,
    % la borne inférieure serait égale à la borne sup ; la borne serait
    % déclarée fermée. Le cas sera traité par modification de la contrainte
    % de conservation des masses. (Borne fermée, mais demande reportée dans
    % le membre de droite.)
    
    work.closed = (xinf == xsup) ;
    work.open = ~work.closed ;
    
    work.A          = dat.M_incid(:,work.open(:)) ;
    work.xsup       = xsup(work.open(:));
    work.xinf       = xinf(work.open(:));
    work.ArcT       = dat.idAter(work.open(:));
    work.ArcB       = dat.idAtra(work.open(:));
    work.ArcS       = dat.idAsrc(work.open(:));
    % work.alpha      = work.alpha(work.open(:));
    % work.beta       = work.beta(work.open(:));
    borne_active    = clients_par_borne(:,k) > 0 ;
    work.clients    = clients_par_borne(borne_active,k) ;
    work.gravit     = dat.gravit(work.open(:));

end