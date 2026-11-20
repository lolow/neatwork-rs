function test_solveModel
% TEST_SOLVEMODEL Resolution reelle LP/MILP sur un optimum analytique.
% Necessite Optimization Toolbox. Executer : test_solveModel
    assert(exist('linprog','file')==2 && exist('intlinprog','file')==2 && ...
        license('test','Optimization_Toolbox'), ...
        'Optimization Toolbox est necessaire pour tester solveModel.');
    model = fixture();
    % Le premier arc doit etre renforce. Son tuyau bon marche ne peut
    % couvrir que 25 %% de la longueur : le melange optimal est [.25;.75].
    % Le deuxieme arc doit etre active par la seconde inegalite ; le
    % troisieme est bloque. Les couts fixes font partie de l'objectif.
    expected = [.25;.75;1;0;0;0;1;1;0];
    sol = solveModel(model); % MILP et mode d'affichage par defaut.
    checkSolution(model,sol,expected,20);

    % Tous les y fixes : meme optimum, mais resolution par linprog.
    lp = model;
    lp.lb(7:9) = [1;1;0];
    sol = solveModel(lp,"off");
    checkSolution(lp,sol,expected,20);

    % Un y libre sans obligation hydraulique : le cout evite son activation.
    optional = model;
    optional.bineq(2) = 0;
    sol = solveModel(optional);
    checkSolution(optional,sol,[.25;.75;0;0;0;0;1;0;0],10);

    % Aucun renforcement : couvre aussi la normalisation d'une selection vide.
    none = optional;
    none.lb(7:9) = 0;
    none.ub(7:9) = 0;
    sol = solveModel(none);
    checkSolution(none,sol,zeros(9,1),0);

    % Toutes les variables nulles sauf une valeur sous le seuil d'elagage.
    % La valeur nettoyee doit etre remise a zero et le cout recalcule.
    tiny = lp;
    tiny.Aineq(1,1:2) = [0 1];
    tiny.bineq(1) = 1e-9;
    tiny.f(1:2) = [6;2];
    sol = solveModel(tiny);
    checkSolution(tiny,sol,[1;0;1;0;0;0;1;1;0],21);

    % Modele infaisable, aussi bien dans la branche LP que MILP.
    for fixed = [false true]
        bad = model;
        if fixed, bad.lb(7:9) = [1;1;0]; end
        bad.Aineq(1,1:2) = [1 1];
        bad.bineq(1) = .5;
        try
            solveModel(bad);
        catch exception
            assert(strcmp(exception.identifier,'NW5:DesignFailure'), ...
                'Erreur inattendue : %s',exception.message);
            assert(contains(exception.message,'Solver diagnostic'));
            continue
        end
        error('test_solveModel:MissingError','Le modele infaisable a ete accepte.');
    end
    fprintf('OK: solveModel, LP/MILP, melanges, elagage, couts et infaisabilite.\n');
end

function model = fixture()
    model = struct('nbA',3,'nbD',2);
    model.f = [2;6;3;9;4;8;5;7;11];
    model.Aeq = sparse([1 1 0 0 0 0 -1 0 0; ...
        0 0 1 1 0 0 0 -1 0;0 0 0 0 1 1 0 0 -1]);
    model.beq = zeros(3,1);
    model.Aineq = sparse([1 0 0 0 0 0 0 0 0; ...
        0 0 0 0 0 0 0 -4 0;0 0 0 0 1 1 0 0 0]);
    model.bineq = [.25;-2;0];
    model.lb = [zeros(6,1);1;0;0];
    model.ub = [ones(8,1);0];
end

function checkSolution(model,sol,expected,cost)
    assert(sol.ok && sol.exitflag==1);
    assert(~isempty(sol.message));
    assert(isequal(size(sol.x_mat),[model.nbD model.nbA]));
    assert(isequal(size(sol.y),[model.nbA 1]));
    closeTo(sol.x,sol.z(1:6));
    revised = [sol.x_mat(:);sol.y];
    closeTo(revised,expected);
    assert(all(sol.y==0 | sol.y==1));
    assert(isequal(sol.reinfArc,logical(expected(7:9))));
    closeTo(sum(sol.x_mat,1).',sol.y);
    closeTo(model.Aeq*revised,model.beq);
    assert(all(revised>=model.lb-1e-7 & revised<=model.ub+1e-7));
    closeTo(sol.slack,model.bineq-model.Aineq*revised);
    assert(all(sol.slack>=-1e-6));
    closeTo(sol.new_fval,model.f.'*revised);
    closeTo(sol.new_fval,cost);
    closeTo(sol.fval,model.f.'*sol.z);
    assert(abs(sol.fval-cost)<1e-6);
end

function closeTo(actual,expected)
    assert(isequal(size(actual),size(expected)),'Dimensions inattendues.');
    assert(all(abs(actual(:)-expected(:))<1e-7), ...
        'Ecart numerique avec le resultat attendu.');
end
