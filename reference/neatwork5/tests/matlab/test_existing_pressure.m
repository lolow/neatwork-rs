function test_existing_pressure()
% Existing pipe pressure is checked without flows or reinforcement choices.
    for categoricalIds = [false true]
        for permuted = [false true]
            [TA,TD] = fixture();
            if categoricalIds
                TA.pred = categorical(TA.pred);
                TA.succ = categorical(TA.succ);
                TA.ID1 = categorical(TA.ID1);
                TA.ID2 = categorical(TA.ID2);
                TD.ID = categorical(TD.ID);
            end
            if permuted
                TA = TA([3 1 2],:);
                TD = TD([2 1],:);
            end
            % Includes equality, a bare link at high pressure, and a pipe
            % carried solely by its second section. No fictitious TD row.
            validate_existing_pressure(TA,TD);
            validate_existing_pressure(removevars(TA,'N'),TD);

            first = TA;
            row = find(string(first.succ)=="B1");
            first.pmax(row) = 31;
            mustReject(first,TD,["J","B1","D30","31 m","30 m","troncon 1"]);

            second = TA;
            row = find(string(second.succ)=="B2");
            second.pmax(row) = 21;
            % N="B" is irrelevant to a hydrostatic pressure violation.
            assert(string(second.N(row))=="B");
            mustReject(second,TD,["J","B2","D20","21 m","20 m","troncon 2"]);
        end
    end

    [TA,TD] = fixture();
    % A catalogue entry on a zero-length section does not impose a pressure
    % constraint, even when its class is below the link's maximum pressure.
    TA.ID2(2) = "D20";
    validate_existing_pressure(TA,TD);

    % Report both insufficient sections on the same link, not only the first.
    twoSections = TA;
    twoSections.lon1(2) = 5;
    twoSections.lon2(2) = 5;
    twoSections.pmax(2) = 31;
    mustReject(twoSections,TD, ...
        ["J","B1","D30","D20","troncon 1","troncon 2","31 m","30 m","20 m"]);

    % A link above the source head is handled by later head constraints;
    % a finite negative pmax does not constitute an excessive pressure.
    negativePressure = TA;
    negativePressure.pmax(:) = -1;
    validate_existing_pressure(negativePressure,TD);

    % Bare links require no pressure lookup at all, even without catalogue rows.
    validate_existing_pressure(TA(1,:),TD([],:));

    % Missing pressure data must not be silently treated as admissible;
    % an explicit unlimited class remains admissible.
    unknownClass = TD;
    unknownClass.pressure(unknownClass.ID=="D30") = NaN;
    mustReject(TA,unknownClass,strings(1,0),'NW5:ExistingPressureData');
    unlimitedClass = TD;
    unlimitedClass.pressure(unlimitedClass.ID=="D30") = Inf;
    validate_existing_pressure(TA,unlimitedClass);

    checkSourcePumpPressure();
    fprintf('OK: existing pressure classes, diagnostics, empty sections and source pump.\n');
end

function [TA,TD] = fixture()
    TA = table(["S";"J";"J"],["J";"B1";"B2"], ...
        [0;10;0],["fictif";"D30";"fictif"], ...
        [0;0;10],["fictif";"fictif";"D20"], ...
        [100;30;20],["N";"N";"B"], ...
        'VariableNames',{'pred','succ','lon1','ID1','lon2','ID2','pmax','N'});
    TD = table(["D20";"D30"],[20;30],'VariableNames',{'ID','pressure'});
end

function checkSourcePumpPressure()
    TN = table(categorical(["S";"J";"B"]),[100;110;90], ...
        [0;1;2],[0;0;0],[0;1;2],[0;0;1], ...
        'VariableNames',{'noeud','haut','xdata','ydata','statut','nbrob'});
    TA = table(categorical(["S";"J"]),categorical(["J";"B"]), ...
        [20;20],[20;20],["P15";"P100"],zeros(2,1), ...
        ["fictif";"fictif"],["N";"N"], ...
        'VariableNames',{'pred','succ','lon','lon1','ID1','lon2','ID2','N'});
    TB = table(categorical("B"),0,'VariableNames',{'borne','pressure'});
    TD = table(["P15";"P100"],[15;100],'VariableNames',{'ID','pressure'});
    p = struct('uniform_dem',1,'pressure',0,'pump_pressure',0);
    [arcs,~,~,~] = enrichTablesReinf(TA,TB,TN,TD,p,100);
    validate_existing_pressure(arcs,TD);

    p.pump_pressure = 20;
    [arcs,~,~,~] = enrichTablesReinf(TA,TB,TN,TD,p,100);
    first = string(arcs.pred)=="S";
    assert(arcs.pmax(first)==20);
    mustReject(arcs,TD,["S","J","P15","20 m","15 m","troncon 1"]);
    TD.pressure(TD.ID=="P15") = 20;
    validate_existing_pressure(arcs,TD);
end

function mustReject(TA,TD,expectedText,identifier)
    if nargin<4, identifier='NW5:ExistingPressure'; end
    try
        validate_existing_pressure(TA,TD);
    catch err
        assert(strcmp(err.identifier,identifier), ...
            'Unexpected failure: %s',err.message);
        for item = expectedText
            assert(contains(err.message,item), ...
                'The pressure diagnostic must include %s: %s',item,err.message);
        end
        return
    end
    error('test_existing_pressure:MissingError', ...
        'An existing pipe with insufficient pressure class was accepted.');
end
