function write_meta(filename, S)
% write_meta  Écrit une structure MATLAB en JSON pretty-print.
    if ~isfield(S,'schemaVersion')
        S.schemaVersion = "NW_design_meta_v1";
    end
    fid = fopen(filename,'w');
    assert(fid>0, 'Impossible d''ouvrir %s en écriture.', filename);
    fprintf(fid, '%s', jsonencode(S, 'PrettyPrint', true));
    fclose(fid);
end
