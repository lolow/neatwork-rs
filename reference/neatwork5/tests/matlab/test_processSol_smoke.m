%TEST_PROCESSSOL_SMOKE  Test autonome minimal de processSol.m.
% Executer dans MATLAB avec processSol.m sur le path.
clear;
TD = table(["fictif";"D1";"D2"], [0;0.05;0.10], [0;5;8], ...
    'VariableNames',{'ID','diam','cost'});
TA = table(["S";"J";"J"], ["J";"B1";"B2"], ...
    [60;30;42], [0;30;42], ["fictif";"D2";"D2"], ...
    [0;0;0], ["fictif";"fictif";"fictif"], ...
    [0;2;2], [-2;-5;-6], zeros(3,1), ["N";"N";"N"], ...
    'VariableNames', {'pred','succ','lon','lon1','ID1','lon2','ID2', ...
                      'statut','haut','pressure','N'});
TN = table(["S";"J";"B1";"B2"], [0;-2;-5;-6], ...
    'VariableNames',{'noeud','haut'});
TN.xdata = zeros(height(TN),1);
TN.ydata = zeros(height(TN),1);
TN.statut = [0;1;2;2];
TA.equip = TA.lon1>0 | TA.lon2>0;
TC = table();
TO = table([0.004;0.005],'VariableNames',{'in_diam'});
TL = array2table([0.7,0.5;0.7,0;0,0.5], ...
    'VariableNames',{'B1','B2'});
TL = addvars(TL,TA.succ,'Before',1,'NewVariableNames','ID');
dat.M_path = sparse([1 1 1;0 1 0;0 0 1]);
pardsg = struct('targetflow',0.4, 'pipelength',6, 'secu',1, ...
                'uniform_dem',1, 'bounded_flow',1);
partec = struct('bescal',9.42/10000*1.27^(-0.219), ...
                'nu',4.781, 'lambda',1.781, 'faucetcoef',1.83e-7, ...
                'orifcoef',0.001);
TD.resist = inf(height(TD),1);
TD.resist(2:end) = partec.bescal ./ TD.diam(2:end).^partec.nu;
rExist = [inf;TD.resist(3);TD.resist(3)];
% Candidates combine each new pipe with the existing pipe in parallel.
rCandidate = (rExist.^(-1/partec.lambda) + ...
    TD.resist(2:end)'.^(-1/partec.lambda)).^(-partec.lambda);
TR = table(TA.succ,rExist,rCandidate(:,1),rCandidate(:,2), ...
    'VariableNames',{'ID','rexist','D1','D2'});
model = struct('f',zeros(9,1));
sol = struct('ok',true,'x_mat',[1 0.4 0;0 0.6 0], ...
             'y',[1;1;0], 'fval',200, 'slack',zeros(3,1));
[TAout,TNout,TCP,TAH,reinfcost,ctx,TS] = processSol(TA,TN,TD,TC,TO,TL,TR, ...
                              dat,pardsg,partec,model,sol,struct());
assert(height(TAout)==4);
assert(height(TNout)==height(TN));
assert(height(TCP)==height(TAout));
assert(nnz(TCP.nature=="old")==2 && nnz(TCP.nature=="new")==2);
assert(nnz(string(TAout.pred)=="J" & string(TAout.succ)=="B1")==2);
assert(all(TAout.ID1~="fictif"));
assert(all(abs(TAout.lon1+TAout.lon2-TAout.lon)<1e-9));
assert(TAH.equivResist(2) < TR.rexist(2));
assert(abs(TAH.equivResist(3)-TR.rexist(3)) < 1e-12*TR.rexist(3));
assert(all(isfinite(TAH.equivResist)) && all(isfinite(TAH.margin)));
assert(isequal(TS.head_margin,TAH.margin));
assert(reinfcost.costFinal==ctx.reinforcement.costPipesAfterRounding);
fprintf('OK : 3 liaisons -> %d arcs physiques; cout ajoute %.2f.\n', ...
         height(TAout),reinfcost.costFinal);

% Un arc initial nu non renforce doit etre refuse.
sol_bad = sol; sol_bad.y(1)=0; sol_bad.x_mat(:,1)=0;
try
    processSol(TA,TN,TD,TC,TO,TL,TR,dat,pardsg,partec,model,sol_bad,struct());
    error('Le test aurait du provoquer une erreur.');
catch ME
    assert(strcmp(ME.identifier,'NW5:DisconnectedArc'));
    fprintf('OK : un arc nu non renforcé est refusé.\n');
end

% Symetrie des segments : un arc reel peut etre porte par le seul lon2.
TA2 = TA;
TA2.lon1(3)=0; TA2.ID1(3)="fictif";
TA2.lon2(3)=42; TA2.ID2(3)="D2";
[TA2out,~,~,~] = processSol(TA2,TN,TD,TC,TO,TL,TR, ...
                                 dat,pardsg,partec,model,sol,struct());
assert(height(TA2out)==4);
assert(any(TA2out.lon1==0 & TA2out.lon2==42 & TA2out.ID2=="D2"));
fprintf('OK : symetrie lon1/lon2 validee.\n');

% Creation ex nihilo : tous les arcs sont initialement fictifs.
TA0 = TA;
TA0.lon1(:)=0; TA0.lon2(:)=0;
TA0.ID1(:)="fictif"; TA0.ID2(:)="fictif";
TA0.equip(:)=false;
TR0 = TR;
TR0.rexist(:)=inf;
TR0.D1(:)=TD.resist(2); TR0.D2(:)=TD.resist(3);
sol0 = sol; sol0.y=ones(3,1); sol0.x_mat=[1 1 1;0 0 0];
[TA0out,~,~,~] = processSol(TA0,TN,TD,TC,TO,TL,TR0, ...
                                 dat,pardsg,partec,model,sol0,struct());
assert(height(TA0out)==3 && all(TA0out.ID1=="D1"));
fprintf('OK : creation ex nihilo validee.\n');
