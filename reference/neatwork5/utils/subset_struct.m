function Sout = subset_struct(S, keep)
% Crée une sous-structure de S ne comprenant que les éléments de keep
% Input S structure, keep: cell array des champs à garder. Keep must be a 
% cell array of character vector.
% Output: sous-structure avec les champs de keep.
    keep = intersect(keep, fieldnames(S));
    Sout = rmfield(S, setdiff(fieldnames(S), keep));
end
