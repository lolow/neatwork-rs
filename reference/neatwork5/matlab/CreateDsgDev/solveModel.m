function sol = solveModel(model, displayMode)
%SOLVEMODEL solves the MILP model using intlinprog or linprog depending 
% on whether all y are fixed. The choice between the two solvers could be 
% made optional. If all y are fixed, the MILP solver behaves like a LP solver, 
% but it is slower than linprog and does not provide dual information.

    if nargin < 2
        displayMode = "off";   % valeur par défaut
    end
    
    f       = model.f;
    nbA     = model.nbA;
    nbD     = model.nbD;
    Aineq   = model.Aineq;
    bineq   = model.bineq;
    Aeq     = model.Aeq;
    beq     = model.beq;
    lb      = model.lb;
    ub      = model.ub;
    
    ny = length(lb) - nbA*nbD;
    idxY = nbA*nbD + (1:ny);

    % ========= A- Solve LP or MILP ============
    
    if all(lb(idxY) == ub(idxY))
        % tous les y sont fixés : LP
        opts = optimoptions('linprog', 'Display', displayMode);

        [z,fval,exitflag,output] = linprog(f,Aineq,bineq,Aeq,beq,lb,ub,opts);
    else
        % au moins un y est libre : MILP
        opts = optimoptions('intlinprog', 'Display', displayMode);
        intcon = idxY;
        [z,fval,exitflag,output] = intlinprog(...
            f,intcon,Aineq,bineq,Aeq,beq,lb,ub,opts);
    end

    % Check the solver ouptput prior to using z
    if exitflag ~= 1
        if exitflag == -2
            diagnostic = ['No solution satisfies all constraints. ', ...
                'Check the model formulation and the input data.'];
        elseif isempty(z)
            diagnostic = 'The solver did not return a solution.';
        else
            diagnostic = 'The solver did not certify z as an optimal solution.';
        end

        error('NW5:DesignFailure', ...
            '%s\nSolver diagnostic :\n%s', ...
            diagnostic, output.message);
    end


    % ======== B- Retrieve x and y an matrices ========
    nx = nbA*nbD;
    x = z(1:1:nx);
    y = z(nx+1 : end);

    % Test equality constraints
    assert(all(abs(Aeq * z) < 1e-6),'Equality constraints not satisfied')

    % Compute slack and test inequality constraints
    slack = bineq - Aineq * z;
    assert(all(slack > -1e-06),'Inequality constraints not satisfied')

    % Check bound constraints
    assert(max([0; lb-z; z-ub]) < 1e-7, 'Bound constraints not satisfied')

    % Test non-integer y
    assert(all(y < 1e-6 | y > 1-1e-6), 'Variable y not close enough to binary')

    % ======= C- Trim solution ========

    % Fix y to binary
    idyS = abs(y)<1e-6;
    y(idyS) = 0;
    idyL = abs(1-y)<1e-6;
    y(idyL) = 1;

    % Display x as a matrix  and set small x to zero   
    x_mat = reshape(x,nbD,nbA);
    idxS = abs(x_mat) < 1e-7;
    x_mat(idxS) = 0;

    % Restore equality constraints
    idreinf = y == 1;
    x_mat(:,~idreinf) = 0;
    x_mat(:,idreinf) = x_mat(:,idreinf) ./ sum(x_mat(:,idreinf,1));


    % === D- Compute slack for revised x and y ===
    x_new = reshape(x_mat,nx,1);
    slack_new = bineq - Aineq * [x_new ; y];

    if ~all(slack_new > -1e-06)
        warning('After trimming the solution, the min slack is : %f', min(slack_new))
    end
    slack = slack_new;
    
    % ===== E- Impact on cost =====
    new_fval = f' * [x_new;y];
    
    impact = (new_fval-fval) / (fval + eps) ;
    if abs(impact) > 1e-5
        sprintf('Trimming changed total cost by %0.4f %%',100*impact)
    end

    % path_pipe_loss = zeros(nbA,1);
    % 
    % % Cumulated losses on path due to reinforcement
    % path_pipe_loss(idreinf) = Aineq(idreinf,1:nx) * x_new;
    % 
    % % Cumulated losses on path with reinforcement
    % path_pipe_loss(~idreinf) = Aineq(~idreinf,1+nx:end) * y;

    % Retrieve reinforced arcs and the combined headLoss. Dense matrices. 
    % Reminder. The matrix Aineq(:,1:nx) has the following property. Row i
    % is associtated with arc i. All columns are zeros but the block
    % associated with arc i, that runs on 1+(i-1)*nbD:i*nbD. The elments on
    % that block are the headlos associated with the various pipes. 

    AineqX = zeros(nbA,nbD);
    % AeqX  = zeros(nbA,nbD);
    for i=1:nbA
        J = 1 + (i-1)*nbD : i*nbD;
        AineqX(i,:) = Aineq(i,J);
        % AeqX(i,:) = Aeq(i,J);
    end
    AineqY = full(Aineq(:,idxY));

    % load on arcs
    reinfHeadloss = sum(AineqX .* x_mat',2)' ;
    reinfArc = y > 0.99; % y is binary, but numerically might be close to 0 or 1

    % Headloss on non reinforced arcs
    equippedHeadloss = max(-AineqY,[],1)';
    notReinfHeadloss = zeros(nbA,1);
    notReinfHeadloss(~reinfArc) = equippedHeadloss(~reinfArc); 

    % notReinfHeadloss = -sum(AineqY(:,~reinfArc),1);
    % notReinfHeadloss = full(notReinfHeadloss');

    % Prepare output
    sol = struct();
    sol.z               = z;
    sol.x               = x;
    sol.x_mat           = x_mat;
    sol.y               = y;
    sol.slack           = slack;   
    sol.fval            = fval;
    sol.new_fval        = new_fval;
    sol.exitflag        = exitflag;
    sol.message         = output.message;
    % sol.iter     = output.iterations; % 
    sol.ok              = (exitflag == 1);
    sol.reinfArc        = reinfArc;
    % sol.reinfHeadloss   = reinfHeadloss;
    % sol.notReinfHeadloss= notReinfHeadloss;

    end