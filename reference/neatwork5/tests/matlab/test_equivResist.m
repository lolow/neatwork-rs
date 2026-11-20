function test_equivResist
% TEST_EQUIVRESIST Verifie les resistances par conservation du debit.
% Depuis ce dossier : test_equivResist
% Aucun fichier de donnees ni solveur d'optimisation n'est necessaire.
    for lambda = [1.2 1.781 2]
        partec = struct('lambda',lambda);
        TD = table(["D0";"D1";"D2"], ["fictif";"petit";"grand"], ...
            [0;.1;.2], [Inf;16;4], ...
            'VariableNames',{'ID','Nominal','diam','resist'});
        TD.cond = TD.resist.^(-1/lambda);
        % Sans equipement, fictif seul, reel seul, deux reels en serie,
        % reel suivi d'un fictif, puis reel uniquement dans le second troncon.
        TA = table(["Z";"A";"Y";"B";"X";"C"], ...
            [100;100;100;100;100;100], ...
            [0;100;100;25;25;0], [0;0;0;75;75;100], ...
            ["absent";"D0";"D1";"D1";"D1";"absent"], ...
            ["absent";"absent";"absent";"D2";"D0";"D2"], ...
            'VariableNames',{'succ','lon','lon1','lon2','ID1','ID2'});
        TN = table(); % Argument actuellement inutilise par equivResist.
        TR = equivResist(TA,TN,TD,partec);
        assert(isequal(size(TR),[height(TA),height(TD)+1]));
        assert(isequal(TR.ID,TA.succ),'Ordre des arcs modifie.');
        assert(isequal(TR.Properties.VariableNames,{'ID','rexist','D1','D2'}));
        actual = TR{:,2:end};
        assert(isequal(actual(1:2,:),repmat(TD.resist.',2,1)), ...
            'Un arc non equipe doit reprendre exactement TD.resist.');

        % Resistances existantes par unite de longueur, calculees a la main.
        existing = [Inf;Inf;16;7;4;4];
        assert(all(abs(actual(3:end,1)-existing(3:end))<1e-12), ...
            'Le candidat fictif doit conserver la resistance existante.');
        for a = 3:height(TA)
            for d = 2:height(TD)
                % Pour un debit total unitaire, repartir le debit de sorte
                % que les deux conduites paralleles aient la meme perte.
                ratio = (TD.resist(d)/existing(a))^(1/lambda);
                qExisting = ratio/(1+ratio);
                qCandidate = 1/(1+ratio);
                expected = existing(a)*qExisting^lambda;
                candidateLoss = TD.resist(d)*qCandidate^lambda;
                assert(abs(expected-candidateLoss)<1e-12);
                assert(abs(actual(a,d)-expected)<1e-12*max(1,expected));
                assert(actual(a,d)<min(existing(a),TD.resist(d)));
            end
        end
        % Diametres fournis ou retrouves par ID : memes resistances.
        withDiam = TA;
        withDiam.diam1 = [0;0;.1;.1;.1;0];
        withDiam.diam2 = [0;0;0;.2;0;.2];
        assert(isequal(TR,equivResist(withDiam,TN,TD,partec)));
        for idType = ["cell","categorical"]
            typedTD = TD;
            if idType=="cell"
                typedTD.ID = cellstr(TD.ID);
            else
                typedTD.ID = categorical(TD.ID);
            end
            assert(isequal(TR,equivResist(TA,TN,typedTD,partec)));
        end
    end

    bad = TA;
    bad.lon1(3) = 99;
    mustFail(@() equivResist(bad,TN,TD,partec),'NW5:EquipmentLength');
    bad = TA;
    bad.ID1(3) = "inconnu";
    mustFail(@() equivResist(bad,TN,TD,partec),'');
    bad = TA;
    bad.ID2(4) = "inconnu";
    mustFail(@() equivResist(bad,TN,TD,partec),'');
    mustFail(@() equivResist(removevars(TA,'ID1'),TN,TD,partec),'');
    mustFail(@() equivResist(TA,TN,removevars(TD,'diam'),partec),'');
    bad = withDiam;
    bad.diam1(3) = NaN;
    mustFail(@() equivResist(bad,TN,TD,partec),'');
    fprintf('OK: equivResist, equipements, parallele, types et integrite.\n');
end

function mustFail(action,identifier)
    try
        action();
    catch exception
        if ~isempty(identifier)
            assert(strcmp(exception.identifier,identifier), ...
                'Erreur inattendue : %s',exception.message);
        end
        return
    end
    error('test_equivResist:MissingError','Une entree invalide a ete acceptee.');
end
