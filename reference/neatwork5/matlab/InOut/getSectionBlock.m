function [block, firstData, lastData] = getSectionBlock(lines, sectionName)
    % lines : string array / cellstr
    % sectionName : ex. "Pipes", "Nodes", "Default Properties", etc.
    hdr = "!" + sectionName;

    idxStart = find(startsWith(strtrim(lines), hdr), 1, 'first');
    if isempty(idxStart)
        block = strings(0,1); firstData = []; lastData = [];
        return
    end

    isHeader = startsWith(strtrim(lines), '!');
    idxAll   = (1:numel(lines))';
    idxNext  = find(isHeader & idxAll > idxStart, 1, 'first');

    firstData = idxStart + 1;
    if isempty(idxNext)
        lastData = numel(lines);
    else
        lastData = idxNext - 1;
    end

    if lastData < firstData
        block = strings(0,1);
    else
        block = lines(firstData:lastData);
    end
end
