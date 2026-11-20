function root = get_root()
%GET_ROOT  Renvoie le chemin racine du projet NW_Git.
% Usage :
%   root = get_root();
%
% Suppose que ce fichier se trouve dans NW_Git/matlab.

thisFile = mfilename('fullpath');   % chemin complet de get_root.m
root = fileparts(fileparts(thisFile));  % remonte de /matlab à NW_Git
end
