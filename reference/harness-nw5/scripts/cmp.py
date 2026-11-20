import json, sys, numpy as np
a, b = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
worst = {}
def walk(x, y, p):
    if isinstance(x, dict):
        for k in x: walk(x[k], y[k], p + "." + k)
    elif isinstance(x, list):
        for i, (u, v) in enumerate(zip(x, y)): walk(u, v, p)
    elif isinstance(x, (int, float)) and not isinstance(x, bool) and x is not None and v_ok(y):
        rel = abs(x - y) / max(abs(x), abs(y), 1e-300); ab = abs(x - y)
        r = worst.setdefault(p, [0, 0, None]); 
        if rel > r[0]: r[0], r[2] = rel, (x, y)
        r[1] = max(r[1], ab)
    elif x != y: worst.setdefault(p + " (NONNUM DIFF)", [x, y, None])
def v_ok(y): return isinstance(y, (int, float)) and not isinstance(y, bool)
walk(a, b, "$")
for k, v in sorted(worst.items(), key=lambda kv: -kv[1][0] if isinstance(kv[1][0], (int,float)) else 0)[:25]: print(f"{k}: rel={v[0]:.3g} abs={v[1]:.3g} ex={v[2]}")
