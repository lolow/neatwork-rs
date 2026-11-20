function model = build_reinf_model(TA, TD, TL, TLv, TR, pardsg, partec, dat)
% LPMODELREINFORCEMENT  This fucntion builds the model to decide on reinforcement 
% for each link of the network. A reinforcment is a pipes with appropriate diameters and 
% characteristics that can be serially installed in parallel to the existing one. If 
% the link is already equipped, reinforcement is optional unless its diameter
% cannot carry the full control flow within vmax. Otherwise reinforcement
% is mandatory. The model is a MILP with the following variables: A vector x of convex 
% combinations of the available reinforcement pipes for each link, and a vector y of 
% binary variables indicating whether the link is blocked or not. The objective is to 
% minimize the total cost of reinforcement while satisfying pressure and flow constraints.
%
% Usefull inputs in this function:
%   TA  : Table of Links (arcs) ; contains ID1, ID2, succ, lon, pmax, haut, statut, N, Qmax
%   TD  : Table of Pipes. Includes  ID, diam, pressure,
%         resist and cond. Pipe ID "fictif" is a fake to describe the absence of equipment. 
%         It is used to model unequipped arcs, but is not an investment variable.
%   TL  : Table of Flows. Includes ID, Q. It estimates the percentile associated with 
%         param.servicequal based on the probabilityy distribution of the demand. 
%   TLv : The same as TL but with a higher value for the percentile. It is used to 
%         compute the maximum flow on each arc.
%   TR  : Two parallel arcs for a link can be replaced by a single equivalent arc 
%         with a resistance equal to the parallel combination of the two resistances. 
%         This table contains the equivalent resistances for each link and each candidate 
%         reinforcement pipe. It is used to compute the load on each arc for each 
%         reinforcement choice.
%         TR.ID = TA.succ The other variables are the ID de TD.
%   pardsg.vmax
%   partec.lambda
%
% In the model the x variables apply to the real pipes only.
%
% Hydraulic assumption for reinforcement of an existing pipe:
% If the reinforcement uses two different pipes in series, their junction
% is assumed to be connected to the existing pipe at the same location.
% The link therefore consists of two sections in series, each containing
% the existing pipe and a reinforcement pipe in parallel. Its equivalent
% linear resistance is the length-weighted average of the two section
% resistances. For a common headloss exponent lambda, linear resistance r
% and conductance c satisfy c = r^(-1/lambda), or r = c^(-lambda).
% Parallel conductances add, so each section has equivalent resistance
% (c_existing + c_reinforcement)^(-lambda). For link length L and flow Q,
% the headloss magnitude is L*r_equivalent*abs(Q)^lambda.
% processSol subsequently removes the assumed connection to the existing
% pipe while preserving the equivalent link resistance.

    nbA = height(TA);

    % ===== A- Prolegomena =====

    % TD.resist et TD.cond sont calculés en amont.
    assert(ismember('resist',TD.Properties.VariableNames), ...
        'NW5:MissingResistance','TD.resist is missing.');
    assert(ismember('cond',TD.Properties.VariableNames), ...
        'NW5:MissingConductance','TD.cond is missing.');

    % Le fictif décrit l'absence d'équipement ; il n'est pas un investissement.
    fake = string(TD.ID) == "fictif";
    assert(nnz(fake)==1,'NW5:FictitiousPipe', ...
        'TD must contain exactly one fictitious pipe.');

    real = ~fake;
    nbD  = nnz(real);      % nombre de tuyaux réels = types d'investissement

    % Vérifier l'alignement de TR avec les arcs et le catalogue.
    assert(height(TR)==nbA,'NW5:TRSize', ...
        'TR and TA must have the same number of rows.');
    assert(all(string(TR.ID)==string(TA.succ)),'NW5:TRAlignment', ...
        'TR.ID must be aligned with TA.succ.');

    realID = string(TD.ID(real));
    trNames = string(TR.Properties.VariableNames);
    assert(all(ismember(realID,trNames)),'NW5:TRVariables', ...
        'TR must contain one variable for each real pipe in TD.');

    assert(isnumeric(pardsg.secu) && isscalar(pardsg.secu) && ...
       isreal(pardsg.secu) && isfinite(pardsg.secu) && ...
       pardsg.secu >= 1, ...
       'NW5:Parameters', ...
       'pardsg.secu must be a finite real scalar greater than or equal to 1.');


    % ===== B- Allowed pipes with respect to pressure and speed =====

    % ----- 1- Constraint on the maximal hydrostatic pressure -----
    % Dimensions: nbD x nbA (real pipes x arcs)
    idxp = TD.pressure(real) >= TA.pmax';


    % ----- 2- Constraint on the maximal speed -----

    % Estimated maximal node-to-node flow on each arc [m3/s].
    TLvm3 = table2array(TLv(:,2:end)) / 1000;
    Qvmax = full(max(TLvm3,[],2));                 % nbA x 1

    % Equivalent resistance for each arc / real reinforcement pipe.
    % Dimensions: nbA x nbD.
    TRreal = table2array(TR(:,realID));

    % Conservative candidate screening: share = 1, so every chosen pipe
    % can carry the complete control flow, including after post-processing.
    % The physical contribution of the existing pipe remains in TRreal.
    minDiam = 2 * sqrt(Qvmax ./ (pi*pardsg.vmax)); % nbA x 1

    % Speed admissibility. Dimensions: nbD x nbA.
    idxv = TD.diam(real) >= minDiam';

    % Pipes admissible simultaneously for pressure and speed.
    % Only real pipes occur here: the fictitious pipe is not an investment.
    allowed_pipes = idxp & idxv;                  % nbD x nbA


    % --- 3- Bounds on variables ---

    % Upper bounds on x
    ubx = double(reshape(allowed_pipes,nbA*nbD,1));

    % Upper bounds on y
    uby = double(TA.N ~= "B");

    % Complete upper bound
    ub = [ubx; uby];

    % Lower bounds on x
    lbx = zeros(nbA*nbD,1);

    % Test the existing pipe alone: reinforcement share = 0, hence the
    % existing pipe must carry all Qvmax. Both positive-length segments
    % carry this flow; the smallest diameter determines admissibility.
    hasSegment = [TA.lon1 > 0, TA.lon2 > 0];
    [known1, pipe1] = ismember(string(TA.ID1),string(TD.ID));
    [known2, pipe2] = ismember(string(TA.ID2),string(TD.ID));
    assert(all(known1 & known2), 'NW5:UnknownPipe', ...
        'An existing pipe ID is absent from TD.');
    existingDiam = [TD.diam(pipe1), TD.diam(pipe2)];
    existingDiam(~hasSegment) = inf; % Ignore absent/fictitious segments.
    minExistingDiam = min(existingDiam,[],2);
    unequipped = ~any(hasSegment,2);
    existingTooFast = minExistingDiam < minDiam;

    % Reinforcement is mandatory for bare links and excessive standalone speed.
    lby = double(unequipped | existingTooFast);

    % Complete lower bound
    lb = [lbx; lby];

    conflict = lby > uby;
    if any(conflict)
        error('NW5:ReinforcementBlocked', ...
            ['Renforcement obligatoire mais interdit (N="B") sur les liaisons ' ...
             'aboutissant aux noeuds : %s.'], ...
            strjoin(string(TA.succ(conflict)), ', '));
    end

    % --- 4. Active variables in the LP and MILP (value 1 in idxpv)
    activ_variables  = (ub == 1) ;


    % ====== C- Build the LP/MILP model =====

    % ---------- 1. Dimensions & allocations ----------
    
    % Allocate space in sparse matrices
    nVar = nbA*(nbD+1) ;
    f     = zeros(nVar,1);
    Aeq   = spalloc(nbA, nVar, nVar);

    nzIneq = nnz(dat.M_path) * (nbD +1);
    Aineq = spalloc(nbA, nVar, nzIneq);

    % Make an array of TL and convert in cubic meters.
    TLm3 = table2array(TL(:,2:end)) / 1000; 

    % Convert targetflow (in case of uniform demand) in cubic meters
    cible = pardsg.targetflow / 1000;

    % ----- 2. Cost and equality constraints Aeq * x = (nbA,1) -----
    % Deal with real pipes onlx 
    for a = 1:nbA
        idx         = (a-1)*nbD + (1:nbD);
        f(idx)      = TA.lon(a) * TD.cost(real);
        if ismember("fixedCost",TA.Properties.VariableNames)
            f(nbA*nbD + a) = TA.fixedCost(a);
        end
        Aeq(a,idx)  = 1;
        Aeq(a,nbA*nbD + a)  = -1;                    
    end
    beq = zeros(nbA,1);


    % --- 3. Inequality constraints Aeq * x - r*y <= bineq ----
    
    % --- 3.1 Right-hand side ---
    % Default value for the right-hand side
    % The headloss at any node cannot exceed the gravity potential
    
    bineq = -TA.haut;
    
    % Modify bineq at the terminal nodes according to the security factor.
    % Divide gravity at terminal by the multiplicative factor pardsg.secu >= 1
    isTerm = (TA.statut == 2);
    bineq(isTerm) = bineq(isTerm) / max(pardsg.secu, eps);

    % Include the impact of the equipment (faucet) into bineq
    if pardsg.bounded_flow == 1
        % At terminal, the upstream pressure loss plus the local loss
        % cannot exceed the the gravity. Aternatively said, the local loss is 
        % substracted to gravity.
        bineq(isTerm) = bineq(isTerm) - TA.pressure(isTerm);
        
        if pardsg.uniform_dem == 1
            % Account for the pressure loss due to the equipement
            bineq(isTerm) = bineq(isTerm) - (cible)^2 / max(partec.faucetcoef, eps);
        end
    else
        if pardsg.uniform_dem == 1
            bineq(isTerm) = bineq(isTerm) - (cible)^2 / max(partec.faucetcoef, eps);
    	else
		    error('The case with nonuniform demands and unbounded flows is not treated')
            % A faire : accepter bounded_flow=0 si les clients sont homogenes
            % au sein de chaque borne, sans imposer l'identite entre bornes.
            % Adapter bineq, les marges et les orifices avec une cible par borne.
            % Coordonner avec la personne chargee de la simulation Python ;
            % voir docs/forum/a_faire.md, section 2. Le refus actuel est maintenu.
	    end
   end

   % Adjust rith-hand side to account for the existing network
    isEquipped = TR.rexist < inf;
    T = max(TL(:,2:end),[],2);
    T = renamevars(T,"max","Qmax");
    TA.Qmax = T.Qmax / 1000;
    Qmax_lambda = TA.Qmax.^partec.lambda;
    l =  zeros(height(TA),1);
    l(isEquipped) = TR.rexist(isEquipped) .* Qmax_lambda(isEquipped);
    load_exist = l .* TA.lon ;
    % contribution cumulated on the path
    load_on_path = dat.M_path' * load_exist;
    bineq = bineq - load_on_path;

    % --- 3.2 Constraint matrix ---

    % Resistance of reinforced arcs: nbA x nbD
    % Respecter l'ordre du catalogue, independamment des colonnes de TR.
    T = TRreal;

    % Head loss on each arc for each reinforcement choice
    load_reinf = T .* Qmax_lambda;       % nbA x nbD
    load_reinf = load_reinf .* TA.lon;  % nbA x nbD

    % Local matrix: one block of nbD variables per arc
    A_loc = spalloc(nbA,nbA*nbD,nbA*nbD);

    for i = 1:nbA
        idx = (i-1)*nbD + (1:nbD);
        A_loc(i,idx) = load_reinf(i,:);
    end

    % Accumulation along paths
    Aineq_x = dat.M_path' * A_loc;

    Aineq_y = -(dat.M_path .* load_exist).';
    % Aineq = [Aineq_x Aineq_y];

    Aineq(:,1:nbA*nbD) = Aineq_x;
    Aineq(:,nbA*nbD+1:end) = Aineq_y;

    % ---------- Packaging ----------
    model = struct();
    model.f         = f;
    model.Aeq       = Aeq;
    model.beq       = beq;
    model.Aineq     = Aineq;
    model.Aineq_x   = Aineq_x;
    model.Aineq_y   = Aineq_y;
    model.bineq     = bineq;
    model.lb        = lb;
    model.ub        = ub;
    model.nbA       = nbA;
    model.nbD       = nbD;
    % model.nbI    = nbI;
    % model.nbT    = nbT;
    model.nVar   = nVar;
    model.activ_variables = activ_variables ;

end
