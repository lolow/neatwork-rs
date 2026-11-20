function v = mustfield(s, fld, default)
%MUSFIELDS checks wheter the field fld belongs to the structure s. If not, 
%the field is created and is assigned the value default
    if isfield(s, fld) && ~isempty(s.(fld))
        v = s.(fld);
    else
        v = default;
    end
end
