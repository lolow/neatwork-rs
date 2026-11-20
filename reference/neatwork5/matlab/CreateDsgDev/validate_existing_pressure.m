function validate_existing_pressure(TA, TD)
%VALIDATE_EXISTING_PRESSURE Check installed pipes before any flow estimate.
% TA.pmax and TD.pressure are expressed in metres of water. Each existing
% positive-length section must support the link's hydrostatic pressure.
% Adding a parallel reinforcement cannot correct an insufficient class.
    hasSegment = [TA.lon1 > 0, TA.lon2 > 0];
    if ~any(hasSegment,'all')
        return
    end

    pipeIds = [string(TA.ID1), string(TA.ID2)];
    [known, pipeRows] = ismember(pipeIds,string(TD.ID));
    assert(all(known(hasSegment)), 'NW5:UnknownPipe', ...
        'Un tuyau existant est absent du catalogue TD : %s.', ...
        strjoin(unique(pipeIds(hasSegment & ~known)), ', '));

    required = repmat(TA.pmax,1,2);
    limits = inf(size(hasSegment));
    limits(hasSegment) = TD.pressure(pipeRows(hasSegment));
    assert(isreal(required) && all(isfinite(required(hasSegment))), ...
        'NW5:ExistingPressureData', ...
        'La pression hydrostatique de l''existant doit etre reelle et finie.');
    assert(isreal(limits) && all(~isnan(limits(hasSegment)) & limits(hasSegment)>=0), ...
        'NW5:ExistingPressureData', ...
        'Les classes de pression des tuyaux existants doivent etre valides et non negatives.');

    insufficient = hasSegment & (limits < required);
    if ~any(insufficient,'all')
        return
    end

    [arcRows, sections] = find(insufficient);
    details = strings(numel(arcRows),1);
    for k = 1:numel(arcRows)
        a = arcRows(k);
        s = sections(k);
        details(k) = sprintf( ...
            ['Liaison %s -> %s, troncon %d, tuyau %s : ' ...
             'pression requise %.6g m, pression admissible %.6g m.'], ...
            string(TA.pred(a)),string(TA.succ(a)),s,pipeIds(a,s), ...
            required(a,s),limits(a,s));
    end
    error('NW5:ExistingPressure', ...
        'Dimensionnement interrompu : pression hydrostatique incompatible avec l''existant.\n%s', ...
        strjoin(details,newline));
end
