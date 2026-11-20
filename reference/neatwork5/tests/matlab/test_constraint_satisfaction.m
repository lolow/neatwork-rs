function test_constraint_satisfaction()
% Test processSol() with a simple network and a solution that violates the
% demand satisfaction constraint at node B2. The test checks that the
% constraint satisfaction table TS is correctly generated, and that the
% JSON and DSG exports include the TS values. It also checks that legacy
% export calls remain valid without TS.
% Post-adjustment diagnostics: node mapping, constraint types and exports.
TA = table(["S";"J";"J"], ["J";"B1";"B2"], ...
    ones(3,1), ones(3,1), repmat("D1",3,1), zeros(3,1), ...
    repmat("fictif",3,1), [0;2;2], [-2;-10;1], [0;1;1], ...
    repmat("N",3,1), true(3,1), ...
    'VariableNames',{'pred','succ','lon','lon1','ID1','lon2','ID2', ...
    'statut','haut','pressure','N','equip'});
TN = table(["B2";"S";"J";"B1"], [1;0;-2;-10], zeros(4,1), ...
    zeros(4,1), [2;0;1;2], ...
    'VariableNames',{'noeud','haut','xdata','ydata','statut'});
TD = table(["fictif";"D1"], [0;.05], [0;5], [inf;1e5], ...
    'VariableNames',{'ID','diam','cost','resist'});
TC = table(); TO = table(.005,'VariableNames',{'in_diam'});
TL = table(TA.succ,[1;1;1],'VariableNames',{'ID','B'});
TR = table(TA.succ,repmat(1e5,3,1),repmat(25000,3,1), ...
    'VariableNames',{'ID','rexist','D1'});
dat.M_path = sparse([1 1 1;0 1 0;0 0 1]);
pardsg = struct('targetflow',.4,'pipelength',1,'secu',1.2, ...
    'uniform_dem',1,'bounded_flow',1);
partec = struct('lambda',2,'faucetcoef',1.83e-7,'orifcoef',.001);
model.f = zeros(6,1);
sol = struct('ok',true,'x_mat',zeros(1,3),'y',zeros(3,1));
[~,~,~,TAH,~,~,TS] = processSol(TA,TN,TD,TC,TO,TL,TR, ...
    dat,pardsg,partec,model,sol,struct());
assert(isequal(string(TS.noeud),["J";"B1";"B2"]));
assert(isequal(TS.constraint_type,["intrusion_prevention"; ...
    "demand_satisfaction";"demand_satisfaction"]));
assert(max(abs(TS.head_available-([2;10/1.2;-1/1.2]-[.1;.2;.2])))<1e-12);
assert(max(abs(TS.head_required-[0;1+.0004^2/partec.faucetcoef; ...
    1+.0004^2/partec.faucetcoef]))<1e-12);
assert(isequal(TS.head_margin,TAH.margin));
assert(isequal(TS.head_deficit,max(0,-TS.head_margin)));
assert(TS.head_margin(2)>0 && TS.head_margin(3)<0);

jsonFile = [tempname '.json']; dsgFile = [tempname '.dsg'];
cleanup = onCleanup(@() deleteFiles(jsonFile,dsgFile)); %#ok<NASGU>
save_dsg_json(jsonFile,TN,TA,table(),TC,table(),TD,TO,table(),TAH, ...
    table(),pardsg,partec,struct(),dat,TS);
P = jsondecode(fileread(jsonFile));
assert(numel(P.TS)==3 && strcmp(P.TS(3).noeud,'B2'));
assert(abs(P.TS(3).head_margin-TS.head_margin(3))<1e-12);
export_neatwork_csv(dsgFile,table(),table(),table(),table(),table(), ...
    table(),table(),table(),table(),table(),table(),TS);
assert(contains(fileread(dsgFile),'!ConstraintSatisfaction'));
% Legacy export calls remain valid without TS.
save_dsg_json(jsonFile,TN,TA,table(),TC,table(),TD,TO,table(),TAH, ...
    table(),pardsg,partec,struct(),dat);
export_neatwork_csv(dsgFile,table(),table(),table(),table(),table(), ...
    table(),table(),table(),table(),table(),table());
fprintf('PASS: TS values, node mapping, JSON/DSG exports and legacy calls.\n');
end

function deleteFiles(varargin)
for k = 1:nargin
    if isfile(varargin{k}), delete(varargin{k}); end
end
end
