function [b] = percentile(x,a)

y = sort(x);
if a >= 1 / length(x) * 100 ;
%    b = y(max(1,round(a/length(x) * 100)));
    b = y(max(1,round(a*length(x) / 100)));
else
    b = y(1)  ;
end