function test_physical_elevations()
% Node geometry remains physical; only arc calculations use the source head.
    for sourceHeight = [100 350]
        for pumpPressure = [0 20]
            for permuted = [false true]
                [TA,TB,TN,TD,~,~,p,~] = fixture(sourceHeight,pumpPressure);
                if permuted
                    TA = TA([2 1],:);
                    TN = TN([3 1 2],:);
                end
                originalNodes = TN;
                [arcs,~,nodes,~] = enrichTablesReinf(TA,TB,TN,TD,p,sourceHeight);
                assertPhysicalNodes(nodes,originalNodes);
                [found,idx] = ismember(["J";"B"],string(arcs.succ));
                assert(all(found));
                closeTo(arcs.haut(idx),[10;-10]-pumpPressure);
                % The source endpoint controls pmax on the uphill first link.
                closeTo(arcs.pmax(idx),[0;10]+pumpPressure);
            end
        end
    end
    checkPhysicalExports();
    fprintf('OK: physical node geometry, source-head calculations and JSON/DSG exports.\n');
end

function checkPhysicalExports()
% Exercise the actual export wrapper; the private dialog stub accepts its
% suggested filename without opening a GUI. It is removed on cleanup.
    originalPath = path;
    stubDir = tempname;
    outDir = tempname(fullfile(get_root(),'out'));
    mkdir(stubDir);
    mkdir(outDir);
    cleanup = onCleanup(@() cleanupExportTest(originalPath,stubDir,outDir)); %#ok<NASGU>
    fid = fopen(fullfile(stubDir,'inputdlg.m'),'w');
    assert(fid>0,'Cannot create private export-dialog stub.');
    fprintf(fid,'function answer = inputdlg(varargin)\nanswer = varargin{4};\nend\n');
    fclose(fid);
    addpath(stubDir,'-begin');
    clear inputdlg

    [TA,TB,TN,TD,TC,TO,p,t] = fixture(100,20);
    % Scramble source placement so preservation cannot depend on its row.
    TN = TN([3 1 2],:);
    TN.orif_ideal = ones(height(TN),1);
    TN.orif_com = ones(height(TN),1);
    TAH = table(TA.pred,TA.succ,[.4;.4],[1;1], ...
        'VariableNames',{'pred','succ','designFlow','margin'});
    TL = table(TA.succ,[.4;.4],'VariableNames',{'ID','B'});
    [~,nom] = fileparts(outDir);
    dat = struct('nom',nom,'version','physical','source_height',100);

    % A source_height field must not trigger an inverse transformation now;
    % omitting it must not revive the legacy reset of the source to zero.
    for sourceMetadata = [true false]
        exportDat = dat;
        if ~sourceMetadata
            exportDat = rmfield(exportDat,'source_height');
        end
        for outputType = ["json" "dsg"]
            exportDat.output_type = char(outputType);
            filename = data4DsgFile(TA,TB,TN,TC,TD,TO,TL,TAH,table(), ...
                p,t,struct('costFinal',0),exportDat);
            assert(isfile(filename),'Expected export was not created.');
            if outputType=="json"
                saved = jsondecode(fileread(filename));
                actualNodes = struct2table(saved.TN);
            else
                lines = splitlines(string(fileread(filename)));
                block = getSectionBlock(lines,'Nodes');
                values = textscan(char(join(block,newline)), ...
                    '%s %f %f %f %f %f %f','Delimiter',',');
                actualNodes = table(string(values{1}),values{2},values{3}, ...
                    values{4},values{5},values{6},values{7}, ...
                    'VariableNames',{'noeud','haut','xdata','ydata', ...
                    'orif_ideal','orif_com','statut'});
            end
            assertPhysicalNodes(actualNodes,TN);
        end
    end
end

function [TA,TB,TN,TD,TC,TO,p,t] = fixture(sourceHeight,pumpPressure)
    TN = table(categorical(["S";"J";"B"]), ...
        sourceHeight+[0;10;-10], ...
        [500000.125;500010.125;500020.125],[200000.25;200000.25;200010.25], ...
        [0;1;2],[0;0;1], ...
        'VariableNames',{'noeud','haut','xdata','ydata','statut','nbrob'});
    TA = table(categorical(["S";"J"]),categorical(["J";"B"]), ...
        [20;20],[20;20],["D1";"D1"],zeros(2,1), ...
        ["fictif";"fictif"],["N";"N"], ...
        'VariableNames',{'pred','succ','lon','lon1','ID1','lon2','ID2','N'});
    TB = table(categorical("B"),1,"N", ...
        'VariableNames',{'borne','pressure','N'});
    TD = table("D1","P50",11,.05,2,1000,1,0, ...
        'VariableNames',{'ID','Nominal','SDR','diam','cost','pressure','type','Roughness'});
    TC = table("C1",categorical("B"),.5,.4, ...
        'VariableNames',{'client','borne','proba','cible'});
    TO = table(.005,'VariableNames',{'in_diam'});
    p = struct('uniform_dem',1,'bounded_flow',1,'pressure',1, ...
        'pump_pressure',pumpPressure,'targetflow',.4,'opentaps',.5, ...
        'pipelength',6,'secu',1,'fusion',0,'split',0);
    t = struct('lambda',2,'nu',4,'bescal',100, ...
        'faucetcoef',1e-4,'orifcoef',.001);
end

function assertPhysicalNodes(actual,expected)
    [found,idx] = ismember(string(expected.noeud),string(actual.noeud));
    assert(all(found) && height(actual)==height(expected),'Exported nodes differ.');
    variables = {'haut','xdata','ydata','statut'};
    if ismember('orif_ideal',expected.Properties.VariableNames)
        variables = [variables,{'orif_ideal','orif_com'}];
    end
    for k = 1:numel(variables)
        name = variables{k};
        closeTo(actual.(name)(idx),expected.(name));
    end
end

function cleanupExportTest(originalPath,stubDir,outDir)
    path(originalPath);
    clear inputdlg
    if isfolder(stubDir), rmdir(stubDir,'s'); end
    if isfolder(outDir), rmdir(outDir,'s'); end
end

function closeTo(actual,expected)
    assert(isequal(size(actual),size(expected)),'Unexpected dimensions.');
    assert(all(abs(actual(:)-expected(:))<1e-10),'Unexpected physical or hydraulic values.');
end
