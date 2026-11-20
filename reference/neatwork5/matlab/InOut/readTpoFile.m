function lines = readTpoFile(dat)
% Lecture du fichier source .tpo — autonome (sans dépendre du path)
    if exist('get_root','file')
        root = get_root();                           % .../NW_Git
    else
        here = fileparts(mfilename('fullpath'));     % .../matlab/CreateDsg/ReadTpo
        root = fileparts(fileparts(fileparts(here)));% remonte à .../NW_Git
    end

    nom  = dat.nom;
    nomv = dat.nomv;
    filename = fullfile(root, 'examples', nom, [nomv '.tpo']);
    
    if ~exist(filename, 'file')
        error('Fichier introuvable : %s', filename);
    end
    assert(exist(filename,'file')==2, 'Introuvable: %s', filename);
    fid = fopen(filename, 'r');
    assert(fid~=-1, 'Impossible d''ouvrir : %s', filename);
    C = textscan(fid, '%s', 'Delimiter', '\n', 'Whitespace', '');
    fclose(fid);
    
    lines = string(C{1});
end





