function [itesim,simRes] = runSimulEngine(simdat)
%UNTITLED Summary of this function goes here
%   Detailed explanation goes here
% arguments (Input)
%     inputArg1
%     inputArg2
% end
% 
% arguments (Output)
%     outputArg1
%     outputArg2
% end

    dat     = simdat.dat ;
    options = simdat.options;
    TA      = simdat.TA;
    TN      = simdat.TN;
    TB      = simdat.TB;
    TD      = simdat.TD;
    TC      = simdat.TC;
    param   = simdat.param;
    
    
    % ======= Générer les tables de scénarios ========
    [demande_borne,clients_par_borne,simRes] = genscen(TC,dat,param) ;
    
    % ====== Simulation des débits ======
    % profile on
    tic
    itesim = 0;
    while itesim < param.nbsim
        itesim = itesim + 1;
    
        % ==== Create the structure work for the current scenario ====
        work = prepare_sim(TA,TB,dat,param,demande_borne,clients_par_borne,itesim) ;
    
        % ====Compute flows and related for the current scenario ====
        work = debits(work,dat,param,options);
    
        % ==== Save results in structure simRes ====
        % indicateur des bornes sélectionnées dans le scénario courant.
        % work.open est l'indicateur des arcs actifs
        ouvT = work.open(dat.idAter);
        simRes.X(work.open,itesim) = work.x; % débits dans l'ensemble des arcs
        simRes.Ouv(:,itesim) = work.open ;
        simRes.Iter(itesim) = work.iter ;
        simRes.Charge(work.open,itesim) = work.charge; %- dat.M_incid(:,work.open)' * work.u;
        simRes.Alpha(work.open,itesim) = work.alpha ;
        simRes.U(:,itesim) = work.u ;
        % simRes.xsup = zeros(dat.nbA,1) ;
        % simRes.xsup(work.open) = work.xsup ;
        simRes.dem_tot(itesim) = sum(demande_borne(:,itesim)) ;
        if param.bounded_flow
            simRes.Satis(ouvT,itesim) =  work.x(work.ArcT) ./ work.xsup(work.ArcT) ;
        else
            simRes.Satis(ouvT,itesim) =  work.x(work.ArcT) ./ work.clients...
                / param.targetflow / 1e-3 ;
        end
    end
    temps=toc;
    simRes.TempsIPM = temps;
    % profile viewer
    % profile off
end