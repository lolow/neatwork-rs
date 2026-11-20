function [TN, TA, TC, TPar, TD, TO, pardsg, partec, dat] = load_dsg_json(jsonFile)
    P = jsondecode(fileread(jsonFile));
    TN = struct2table(P.TN);
    TA = struct2table(P.TA);
    TC = struct2table(P.TC);
    TD = struct2table(P.TD);
    TO = struct2table(P.TO);
    TPar   = P.TPar;
    pardsg = P.pardsg;
    partec = P.partec;
    dat    = P.dat;
    % (si tu veux recatégoriser/rediagnostiquer des types, tu peux le faire ici)
end