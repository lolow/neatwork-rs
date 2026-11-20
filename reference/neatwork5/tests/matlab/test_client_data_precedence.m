function test_client_data_precedence()
% Import real TPO/DSG files: clients override counts and uniform parameters.
% The temporary fixture directory is private to this test and removed on exit.
    fixtureDir = tempname(fullfile(get_root(),'examples','tpo_examples'));
    mkdir(fixtureDir);
    cleanup = onCleanup(@() rmdir(fixtureDir,'s')); %#ok<NASGU>
    [~,nom] = fileparts(fixtureDir);
    uniformClients = ["C3,B2,0.7,0.6";"C2,B1,0.7,0.6";"C1,B1,0.7,0.6"];
    heterogeneousClients = ["C3,B2,0.2,0.5";"C2,B1,0.8,0.4";"C1,B1,0.2,1"];

    % Explicit clients replace incorrect TPO counts and obsolete parameters.
    dat = writeFixture(fixtureDir,nom,'_uniform','tpo',uniformClients,0);
    [TN,~,TC,~,~,~,~,~,pardsg] = parseTpoToTables(dat);
    assertNodeCounts(TN);
    assertUniformDemand(TC,pardsg);

    % Heterogeneous clients override an obsolete uniform flag, without losing
    % their own probabilities/targets or replacing them by global defaults.
    dat = writeFixture(fixtureDir,nom,'_heterogeneous','tpo',heterogeneousClients,1);
    [TN,~,TC,~,~,~,~,~,pardsg] = parseTpoToTables(dat);
    assertNodeCounts(TN);
    assertHeterogeneousDemand(TC,pardsg);

    % A TPO without clients still builds them from its terminal counts.
    dat = writeFixture(fixtureDir,nom,'_legacy','tpo',strings(0,1),0);
    [TN,~,TC,~,~,~,~,~,pardsg] = parseTpoToTables(dat);
    assertNodeCounts(TN);
    assert(height(TC)==3 && pardsg.uniform_dem==1);
    closeTo(TC.proba,.1*ones(3,1));
    closeTo(TC.cible,9*ones(3,1));
    closeTo(pardsg.opentaps,.1);
    closeTo(pardsg.targetflow,9);

    % DSG nodes carry orifices, not counts. The parser must preserve these
    % columns and derive nbrob only from the explicit clients.
    dat = writeFixture(fixtureDir,nom,'_uniform','dsg',uniformClients,0);
    [TN,~,TC,~,~,~,~,param] = parseDsgToTables(dat,strings(0,1));
    assertNodeCounts(TN);
    assertUniformDemand(TC,param);
    assertDsgOrifices(TN);
    [~,TN,TC,~,~,~,~,param,~,pardsg] = ...
        readDesign(nom,'_uniform',[],[],[],'dsg','examples/tpo_examples');
    assertNodeCounts(TN);
    assertUniformDemand(TC,param);
    assertUniformDemand(TC,pardsg);
    assertDsgOrifices(TN);

    % The shared reader must not reinstate an obsolete uniform flag.
    dat = writeFixture(fixtureDir,nom,'_heterogeneous','dsg',heterogeneousClients,1);
    [TN,~,TC,~,~,~,~,param] = parseDsgToTables(dat,strings(0,1));
    assertNodeCounts(TN);
    assertHeterogeneousDemand(TC,param);
    [~,TN,TC,~,~,~,~,param,~,pardsg] = ...
        readDesign(nom,'_heterogeneous',[],[],[],'dsg','examples/tpo_examples');
    assertNodeCounts(TN);
    assertHeterogeneousDemand(TC,param);
    assertHeterogeneousDemand(TC,pardsg);

    % Unlike a TPO, an old DSG without !Clients must remain an explicit error.
    dat = writeFixture(fixtureDir,nom,'_legacy','dsg',strings(0,1),1);
    assertMissingClients(@() parseDsgToTables(dat,strings(0,1)));
    assertMissingClients(@() readDesign(nom,'_legacy',[],[],[],'dsg','examples/tpo_examples'));
    checkSimulationAfterReview();
    fprintf('OK: client precedence, TPO fallback, DSG orifices, missing clients and reviewed scenarios.\n');
end

function dat = writeFixture(folder,nom,version,format,clients,uniformFlag)
    if strcmp(format,'tpo')
        % Deliberately wrong counts when clients exist; actual counts otherwise.
        if isempty(clients)
            nodes = ["!Nodes";"B2,70,2,-1,1,2";"S,100,0,0,0,0"; ...
                "B1,80,2,1,2,2";"J,90,1,0,0,1"];
        else
            nodes = ["!Nodes";"B2,70,2,-1,9,2";"S,100,0,0,0,0"; ...
                "B1,80,2,1,17,2";"J,90,1,0,4,1"];
        end
        pipes = ["!Pipes";"J,B2,100";"S,J,100";"J,B1,100"];
        diameters = ["!Diameters";"D1,P50,11,0.05,2,1000,1,0"];
        orifices = ["!Orifices";"0.005,N"];
    else
        nodes = ["!Nodes";"B2,70,2,-1,0.006,0.007,2"; ...
            "S,100,0,0,1,1,0";"B1,80,2,1,0.004,0.005,2"; ...
            "J,90,1,0,1,1,1"];
        pipes = ["!Pipes";"J,B2,100,100,D1,0,fictif,N"; ...
            "S,J,100,100,D1,0,fictif,N";"J,B1,100,100,D1,0,fictif,N"];
        diameters = ["!Diameters";"D1,P50,11,0.05,2,1000,1,0,N"];
        orifices = ["!Orifices";"0.005,N,N,N,N,N"];
    end
    properties = ["!Default properties";"opentaps,0.1";"targetflow,9"; ...
        "uniform_dem,"+string(uniformFlag);"bounded_flow,1";"pressure,2"];
    lines = [nodes;pipes;properties;diameters;orifices];
    if ~isempty(clients)
        lines = [lines;"!Clients";clients];
    end
    dat = struct('nom',nom,'nomv',[nom version], ...
        'adresse2fiche',fullfile(folder,[nom version '.' format]));
    fid = fopen(dat.adresse2fiche,'w');
    assert(fid>0,'Cannot create private parser fixture.');
    cleanup = onCleanup(@() fclose(fid)); %#ok<NASGU>
    fwrite(fid,char(join(lines,newline)),'char');
end

function assertNodeCounts(TN)
    assert(ismember('nbrob',TN.Properties.VariableNames));
    [found,idx] = ismember(["S";"J";"B1";"B2"],string(TN.noeud));
    assert(all(found));
    closeTo(TN.nbrob(idx),[0;0;2;1]);
end

function assertUniformDemand(TC,param)
    assert(height(TC)==3 && param.uniform_dem==1);
    assert(isnumeric(param.uniform_dem),'The persisted flag must remain numeric.');
    closeTo(param.opentaps,.7);
    closeTo(param.targetflow,.6);
    closeTo(TC.proba,.7*ones(3,1));
    closeTo(TC.cible,.6*ones(3,1));
end

function assertHeterogeneousDemand(TC,param)
    assert(height(TC)==3 && param.uniform_dem==0);
    assert(isnumeric(param.uniform_dem),'The persisted flag must remain numeric.');
    [found,idx] = ismember(["C1";"C2";"C3"],string(TC.client));
    assert(all(found));
    closeTo(TC.proba(idx),[.2;.8;.2]);
    closeTo(TC.cible(idx),[1;.4;.5]);
    % These globals are irrelevant to heterogeneous demand and stay untouched.
    closeTo(param.opentaps,.1);
    closeTo(param.targetflow,9);
end

function assertDsgOrifices(TN)
    [~,idx] = ismember(["B1";"B2"],string(TN.noeud));
    closeTo(TN.orif_ideal(idx),[.004;.006]);
    closeTo(TN.orif_com(idx),[.005;.007]);
end

function assertMissingClients(action)
    try
        action();
    catch err
        assert(contains(err.message,'ne contient pas de section Clients'), ...
            'Unexpected failure instead of the missing-client diagnostic.');
        return
    end
    error('test_client_data_precedence:MissingError', ...
        'A DSG without !Clients must be rejected.');
end

function checkSimulationAfterReview()
    rngState = rng;
    cleanup = onCleanup(@() rng(rngState)); %#ok<NASGU>
    TC = table(["B1";"B1";"B2"],.3*ones(3,1),.4*ones(3,1), ...
        'VariableNames',{'borne','proba','cible'});
    dat = struct('nbA',3,'nbAter',2,'nbNtra',1);
    reviewed = struct('uniform_dem',1,'opentaps',1,'targetflow',.7,'nbsim',3);
    [demand,clients] = genscen(TC,dat,reviewed);
    closeTo(demand,repmat([1.4;.7],1,3));
    closeTo(clients,repmat([2;1],1,3));

    % Global review settings do not replace individual heterogeneous demands.
    TC.proba(:) = 1;
    TC.cible = [.2;.3;.9];
    reviewed.uniform_dem = 0;
    reviewed.opentaps = 0;
    reviewed.targetflow = 99;
    [demand,clients] = genscen(TC,dat,reviewed);
    closeTo(demand,repmat([.5;.9],1,3));
    closeTo(clients,repmat([2;1],1,3));
end

function closeTo(actual,expected)
    assert(isequal(size(actual),size(expected)),'Unexpected dimensions.');
    assert(all(abs(actual(:)-expected(:))<1e-12),'Unexpected imported values.');
end
