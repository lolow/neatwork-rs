"""Clean-room re-implementation of the NW5 python-core simulation, written from the
spec notes (no import of the `app` package). Used to validate rust/nw5/specs/simulation.md.
Usage: python spec_check.py <dsg> <seed> <nscen> <ref_prefix>"""
import json, math, sys
from pathlib import Path
import numpy as np

sys.path.insert(0, str(Path(__file__).parent))
from pcg_check import PCG64  # verified bit-exact against numpy

# ---------------- 1. DSG reading (dsg_parser + parsing_utils) ----------------
def clean_lines(text):
    out = []
    for line in text.splitlines():
        s = line.strip()
        if s and not s.startswith("#"):
            out.append(s.lstrip("﻿"))
    return out

def norm_sec(v): return v.strip().lstrip("﻿").lower().replace("_", " ").replace("-", " ")

def section(lines, name):
    exp, start = norm_sec(name), -1
    for i, l in enumerate(lines):
        if not l.startswith("!"): continue
        if norm_sec(l[1:]) == exp: start = i + 1; continue
        if start != -1: return lines[start:i]
    return lines[start:] if start != -1 else []

def to_number(t):
    t = t.strip()
    if t == "": return ""
    try:
        f = float(t)
        return int(f) if f.is_integer() else f
    except ValueError:
        return t

def rows(lines, cols):
    out = []
    for r in lines:
        v = [x.strip() for x in r.split(",")]
        if len(v) < len(cols): continue
        out.append({c: to_number(v[i]) for i, c in enumerate(cols)})
    return out

DSG_DEFAULTS = {"bescal": 8.9396e-04, "bounded-flow": 0, "faucetcoef": 1.0, "fusion": 0, "lambda": 0,
    "limitbudget": float("inf"), "nbsim": 100, "nu": 4.7810, "opentaps": 1, "orif_com": 1, "orifcoef": 1.0,
    "pipelength": 6.0, "pressure": 2, "secu": 1.0, "serviceequal": 0.65, "seuil": 0.0, "split": 0,
    "targetflow": 0.0, "uniform_dem": 1, "use_efficient_pipes": 0, "vmax": 1.5, "vmax_secu": 0.95, "watertemp": 20.0}
CORE_DEFAULTS = {"bescal": 8.9396e-04, "bounded_flow": 0, "faucetcoef": 1.0, "fusion": 0, "lambda": 1.7810,
    "limitbudget": float("inf"), "nbsim": 100, "nu": 4.7810, "opentaps": 1, "orif_com": 1, "orifcoef": 1.0,
    "pipelength": 6.0, "pressure": 2, "secu": 1.0, "servicequal": 0.65, "seuil": 0.0, "split": 0,
    "targetflow": 0.0, "uniform_dem": 1, "use_efficient_pipes": 0, "vmax": 1.5, "vmax_secu": 0.95, "watertemp": 20.0}
INT_PARAMS = {"bounded_flow", "fusion", "nbsim", "orif_com", "split", "uniform_dem", "use_efficient_pipes"}
ALIASES = {"serviceequal": "servicequal", "bounded_flow_enabled": "bounded_flow", "open_taps": "opentaps",
    "orifice_commercial": "orif_com", "orifice_coefficient": "orifcoef", "faucet_coefficient": "faucetcoef",
    "pipe_length": "pipelength", "service_quality": "servicequal", "target_flow": "targetflow",
    "uniform_demand": "uniform_dem", "maximum_velocity": "vmax", "maximum_safe_velocity": "vmax_secu",
    "water_temperature": "watertemp"}
import re
def canon(name):
    n = name.strip().replace("-", "_").replace(" ", "_")
    n = re.sub(r"(?<!^)(?=[A-Z])", "_", n)
    while "__" in n: n = n.replace("__", "_")
    n = n.lower()
    return ALIASES.get(n, n)

def load(path):
    L = clean_lines(Path(path).read_text(encoding="utf-8"))
    nodes = rows(section(L, "Nodes"), ["noeud", "haut", "xdata", "ydata", "orif_ideal", "orif_com", "statut"])
    pipes = rows(section(L, "Pipes"), ["pred", "succ", "lon", "lon1", "ID1", "lon2", "ID2", "N"])
    clients = rows(section(L, "Clients"), ["client", "borne", "proba", "cible"])
    diams = rows(section(L, "Diameters"), ["ID", "Nominal", "SDR", "diam", "cost", "Pmax", "Type", "Roughness", "N"])
    bornes = rows(section(L, "Borne"), ["borne", "pressure", "N"])
    raw = dict(DSG_DEFAULTS)
    for r in section(L, "Default properties"):
        v = [x.strip() for x in r.split(",")]
        if len(v) >= 2: raw[v[0]] = to_number(v[1])
    P = dict(CORE_DEFAULTS)
    for k, v in raw.items():
        if v is None: continue
        c = canon(str(k))
        v = int(v) if isinstance(v, bool) else (v if isinstance(v, (int, float)) else float(str(v)))
        P[c] = int(v) if c in INT_PARAMS else float(v)
    S = str
    nodes = [dict(id=S(n["noeud"]).strip(), z=float(n["haut"]), oi=float(n["orif_ideal"]), oc=float(n["orif_com"]),
                  st=int(n["statut"])) for n in nodes]
    did = lambda v: None if v in (None, "", 0, "0") else S(v)
    pipes = [dict(eid=f"A{i+1}", pred=S(p["pred"]).strip(), succ=S(p["succ"]).strip(), lon1=float(p["lon1"]),
                  id1=did(p["ID1"]), lon2=float(p["lon2"]), id2=did(p["ID2"]), open=True) for i, p in enumerate(pipes)]
    clients = [dict(c=S(c["client"]).strip(), b=S(c["borne"]).strip(), p=float(c["proba"]), q=float(c["cible"])) for c in clients]
    diams = {S(d["ID"]).strip(): float(d["diam"]) for d in diams}
    bornes = {S(b["borne"]).strip(): float(b["pressure"]) for b in bornes}
    uniform = len({c["p"] for c in clients}) == 1 and len({c["q"] for c in clients}) == 1
    if uniform:
        P["opentaps"], P["targetflow"], P["uniform_dem"] = clients[0]["p"], clients[0]["q"], 1
    else:
        P["uniform_dem"], P["bounded_flow"] = 0, 1
    return nodes, pipes, clients, diams, bornes, P

# ---------------- 2. preprocessing ----------------
def prepare(nodes, pipes, clients, diams, bornes, P):
    st = {n["id"]: n["st"] for n in nodes}; z = {n["id"]: n["z"] for n in nodes}
    for p in pipes:
        p["st"] = 0 if st.get(p["pred"]) == 0 else 1
        if st.get(p["succ"]) == 2: p["st"] = 2
        p["g"] = -z[p["pred"]] if p["st"] == 0 else (z[p["succ"]] if p["st"] == 2 else 0.0)
        p["d1"] = diams[p["id1"]] if p["lon1"] > 0 else 1.0
        beta = 0.0
        if p["lon1"] > 0: beta = P["bescal"] * p["lon1"] / p["d1"] ** P["nu"]
        if p["lon2"] > 0: beta += P["bescal"] * p["lon2"] / diams[p["id2"]] ** P["nu"]
        p["beta"] = beta
        fa = 1.0 / P["faucetcoef"] if P["faucetcoef"] > 0 else 0.0
        node = next(n for n in nodes if n["id"] == p["succ"])
        p["a_id"] = (P["orifcoef"] / node["oi"]) ** 4 + fa if p["st"] == 2 and node["oi"] > 0 else 0.0
        p["a_com"] = (P["orifcoef"] / node["oc"]) ** 4 + fa if p["st"] == 2 and node["oc"] > 0 else 0.0
    nodes = sorted(nodes, key=lambda n: (n["st"], n["id"]))
    pipes = sorted(pipes, key=lambda p: (p["st"], p["succ"], p["pred"], p["eid"]))
    clients = sorted(clients, key=lambda c: (c["b"], c["c"]))
    transit = [n["id"] for n in nodes if n["st"] == 1]
    ti = {k: i for i, k in enumerate(transit)}
    A = np.zeros((len(transit), len(pipes)))
    for j, p in enumerate(pipes):
        if p["pred"] in ti: A[ti[p["pred"]], j] = -1.0
        if p["succ"] in ti: A[ti[p["succ"]], j] = 1.0
    return nodes, pipes, clients, A

# ---------------- 3. scenarios ----------------
def scenarios(nodes, clients, P, seed, nscen, osf=1.10, maxb=20, maxd=100_000):
    term = [n["id"] for n in nodes if n["st"] == 2]
    if P["uniform_dem"]:
        prob = np.full(len(clients), float(P["opentaps"])); tq = np.full(len(clients), float(P["targetflow"]))
    else:
        prob = np.array([c["p"] for c in clients]); tq = np.array([c["q"] for c in clients])
    vp = 1.0 - float(np.prod(1.0 - prob))
    rng = PCG64(seed); C = len(clients); openm = np.zeros((C, nscen), bool); got = 0; batches = 0; draws = 0
    while got < nscen:
        batches += 1; assert batches <= maxb
        rem = nscen - got
        d = min(max(rem, int(math.ceil(osf * rem / vp))), maxd); draws += d
        R = np.array([[rng.random() for _ in range(d)] for _ in range(C)])  # C-order: client-major
        S = R < prob[:, None]; S = S[:, S.any(axis=0)]; take = min(S.shape[1], rem)
        if take == 0: continue
        openm[:, got:got + take] = S[:, :take]; got += take
    T = np.zeros((len(term), C)); tidx = {t: i for i, t in enumerate(term)}
    for k, c in enumerate(clients): T[tidx[c["b"]], k] = 1.0
    dem = T @ (openm * tq[:, None]); cnt = np.rint(T @ openm.astype(float)).astype(int)
    return term, dem, cnt, draws, batches

# ---------------- 4. IPM ----------------
def chol(M):
    try: return np.linalg.cholesky(M)
    except np.linalg.LinAlgError: pass
    d = 1e-14
    for _ in range(8):
        try: return np.linalg.cholesky(M + d * np.eye(len(M)))
        except np.linalg.LinAlgError: d *= 10
    raise RuntimeError("chol failed")
def csolve(L, b): return np.linalg.solve(L.T, np.linalg.solve(L, b))

def fderiv(x, al, be, g, lbd, tolx):
    ax = np.abs(x); reg = ax > tolx; sm = ~reg; axl = np.zeros_like(x); axl[reg] = ax[reg] ** (lbd - 1)
    grad = np.zeros_like(x); F = np.zeros_like(x); G = np.zeros_like(x)
    grad[reg] = be[reg] * axl[reg] * x[reg] + al[reg] * ax[reg] * x[reg] + g[reg]
    grad[sm] = tolx ** (lbd - 1) * be[sm] * x[sm] + tolx * al[sm] * x[sm] + g[sm]
    F[reg] = lbd * be[reg] * axl[reg] + 2 * al[reg] * ax[reg]; F[sm] = tolx ** (lbd - 1) * be[sm] + tolx * al[sm]
    G[reg] = lbd * (lbd - 1) * be[reg] * axl[reg] / x[reg] + 2 * al[reg] * np.sign(x[reg])
    return grad, F, G

def steplen(x, lo, up, s, t, dx, ds, dt):
    th = [1.0]
    m = dx < 0; th.append(np.min(-(x[m] - lo[m]) / dx[m]) if m.any() else 1)
    m = dx > 0; th.append(np.min((up[m] - x[m]) / dx[m]) if m.any() else 1)
    m = ds < 0; th.append(np.min(s[m] / (-ds[m])) if m.any() else 1)
    m = dt < 0; th.append(np.min(t[m] / (-dt[m])) if m.any() else 1)
    return min(th)

def ipm(A, lo, up, al, be, g, tm, sm_, srcm, bounded, lbd, tolx=1e-8, tolp=1e-6, maxit=30, sig=0.1, tau=2.5, sec=0.999):
    m, n = A.shape; x = np.zeros(n); u = np.zeros(m); lo = lo.copy(); up = up.copy()
    if not bounded: x[tm] = up[tm]; up *= 5; lo *= 5
    else: x[tm] = (up[tm] + lo[tm]) / 2
    r0 = A @ x; xs = np.zeros(n); srcs = np.flatnonzero(srcm)
    for si in srcs:
        sel = sm_.copy(); sel[si] = True; B = A[:, sel]; du = csolve(chol(B @ B.T), r0)
        cand = x.copy(); cand[sel] += -B.T @ du; xs += cand
    x = xs / len(srcs); s = 1 / (x - lo); t = 1 / (up - x); mu = (s @ (x - lo) + t @ (up - x)) / (2 * n)
    grad, F, G = fderiv(x, al, be, g, lbd, tolx); invD = np.ones(n); L = chol((A * invD) @ A.T)
    it = 0; conv = False
    while it < maxit:
        it += 1
        ch = grad + A.T @ u; rp = A @ x; rd = ch - s + t
        atl = (x - lo) < tolx; atu = (up - x) < tolx; inn = ~(atl | atu)
        bad = np.max(np.abs(rp), initial=0) > tolx or (inn.any() and np.max(np.abs(ch[inn])) > tolp) \
            or (atu.any() and ch[atu].max() > tolp) or (atl.any() and ch[atl].min() < -tolp)
        if not bad: conv = True; break
        rho = -ch; du = csolve(L, rp + A @ (invD * rho)); dx = -invD * (A.T @ du - rho)
        ds = -(s * dx) / (x - lo) - s; dt = (t * dx) / (up - x) - t
        ta = steplen(x, lo, up, s, t, dx, ds, dt)
        mua = ((s + ta * ds) @ (x + ta * dx - lo) + (t + ta * dt) @ (up - x - ta * dx)) / (2 * n)
        smu = min(1.0, (mua / mu) ** tau) * mu
        rcs = s + (dx * ds - smu) / (x - lo); rct = t - (dx * dt + smu) / (up - x)
        rc = -(rd + 0.5 * dx * G * dx + rcs - rct)
        du2 = csolve(L, rp + A @ (invD * rc)); dx2 = -invD * (A.T @ du2 - rc)
        ds2 = -(s * dx2) / (x - lo) - rcs; dt2 = (t * dx2) / (up - x) - rct
        th = sec * steplen(x, lo, up, s, t, dx2, ds2, dt2)
        x = x + th * dx2; u = u + th * du2; s = s + th * ds2; t = t + th * dt2
        mu = sig * (s @ (x - lo) + t @ (up - x)) / (2 * n)
        grad, F, G = fderiv(x, al, be, g, lbd, tolx)
        invD = np.maximum(1.0 / (F + s / (x - lo) + t / (up - x)), 1e-16); L = chol((A * invD) @ A.T)
    return x, u, grad + A.T @ u, it, conv

# ---------------- 5. batch + statistics ----------------
def run(path, seed, nscen):
    nodes, pipes, clients, diams, bornes, P = load(path)
    nodes, pipes, clients, A = prepare(nodes, pipes, clients, diams, bornes, P)
    term, dem, cnt, draws, batches = scenarios(nodes, clients, P, seed, nscen)
    npipe = len(pipes); st = np.array([p["st"] for p in pipes]); tmask = st == 2; smask = st == 0; bmask = st == 1
    beta = np.array([p["beta"] for p in pipes]); g = np.array([p["g"] for p in pipes]); lbd = float(P["lambda"])
    bounded = bool(P["bounded_flow"]); uniform = bool(P["uniform_dem"]); preq = np.array([bornes[t] for t in term])
    X = np.full((npipe, nscen), np.nan); CH = np.full((npipe, nscen), np.nan); U = np.full((A.shape[0], nscen), np.nan)
    SAT = np.full((len(term), nscen), np.nan); served = np.zeros((len(term), nscen), bool); iters = np.full(nscen, -1); ok = np.zeros(nscen, bool)
    tidx = np.flatnonzero(tmask)
    for k in range(nscen):
        d = dem[:, k] / 1000.0; c = cnt[:, k]; act = d > 0
        alpha = np.zeros(npipe)
        if uniform and not bounded:
            base = np.array([pipes[i]["a_com" if int(P["orif_com"]) == 1 else "a_id"] for i in tidx])
            alpha[tidx[act]] = base[act] / c[act].astype(float) ** 2
        else:
            alpha[tidx[act]] = preq[act] / d[act] ** 2
        tot = float(np.sum(d)); gub = tot if bounded else 20.0 * tot; tub = d if bounded else 20.0 * d
        up = np.full(npipe, gub); lo = -up.copy(); up[tmask] = tub; lo[tmask] = 0.0; lo[smask] = 0.0
        op = ~(lo == up); served[:, k] = act
        x, u, ch, it, conv = ipm(A[:, op], lo[op], up[op], alpha[op], beta[op], g[op], tmask[op], bmask[op], smask[op], bounded, lbd)
        iters[k] = it
        if not conv: continue
        full = np.zeros(npipe); full[op] = x; X[:, k] = full; CH[op, k] = ch; U[:, k] = u; ok[k] = True
        sat = np.zeros(len(term)); tf = x[tmask[op]]
        den = d[act] if bounded else c[act].astype(float) * (float(P["targetflow"]) / 1000.0)
        sat[act] = tf / den; SAT[:, k] = sat
    return dict(term=term, X=X, CH=CH, U=U, SAT=SAT, served=served, iters=iters, ok=ok, cnt=cnt, dem=dem,
                draws=draws, batches=batches, pipes=pipes, nodes=nodes, tmask=tmask, smask=smask, bmask=bmask)

def lin_pct(v, q):  # numpy 'linear' percentile of a 1-D finite array
    v = np.sort(v); n = len(v); vi = (n - 1) * (q / 100); lo = math.floor(vi); hi = min(lo + 1, n - 1)
    if vi >= n - 1: lo = hi = n - 1
    gam = vi - lo; a, b = v[lo], v[hi]; diff = b - a
    return b - diff * (1 - gam) if gam >= 0.5 else a + diff * gam

def tail_mean(v, q, upper=False):
    th = lin_pct(v, q); sel = v[v >= th] if upper else v[v <= th]; return float(np.mean(sel))

def stats(R):
    ok = R["ok"]; out = {"terminalSatisfaction": [], "sourceFlow": [], "pipeVelocity": [], "terminalPressure": [], "transitPressure": []}
    for i, t in enumerate(R["term"]):
        m = (R["cnt"][i] > 0)[ok]; v = R["SAT"][i, ok][m]
        rec = {"terminalId": t, "activeScenarioCount": int((R["cnt"][i] > 0).sum()), "evaluatedActiveScenarioCount": int(m.sum()),
               "waterShortfallCount": int((v < 1 - 1e-9).sum())}
        if len(v):
            mean = float(np.mean(v)); sd = float(np.std(v))
            rec.update(minSat=float(v.min()), moySat=mean, maxSat=float(v.max()), cVarSat5=tail_mean(v, 5), cVarSat20=tail_mean(v, 20),
                       stdSat=sd, cvSat=(sd / mean if abs(mean) > 1e-12 else None))
        else:
            rec.update(minSat=None, moySat=None, maxSat=None, cVarSat5=None, cVarSat20=None, stdSat=None, cvSat=None)
        out["terminalSatisfaction"].append(rec)
    srcs = [n["id"] for n in R["nodes"] if n["st"] == 0]
    for sid, row in zip(srcs, R["X"][R["smask"]][:, ok] * 1000.0):
        out["sourceFlow"].append({"sourceId": sid, "minSt": row.min(), "moySt": row.mean(), "maxSt": row.max(),
                                  "cVarSt5": tail_mean(row, 5), "cVarSt95": tail_mean(row, 95, True)})
    im = R["smask"] | R["bmask"]
    for j in np.flatnonzero(im):
        p = R["pipes"][j]; v = np.abs(R["X"][j, ok]) / (math.pi * p["d1"] ** 2 / 4.0)
        out["pipeVelocity"].append({"edgeId": p["eid"], "startNodeId": p["pred"], "endNodeId": p["succ"],
                                    "minVelocityMps": v.min(), "meanVelocityMps": v.mean(), "maxVelocityMps": v.max()})
    for i, t in enumerate(R["term"]):
        j = np.flatnonzero(R["tmask"])[i]; m = R["served"][i, ok]; v = -R["CH"][j, ok][m]
        out["terminalPressure"].append({"terminalId": t, "minPressureM": float(v.min()) if len(v) else 0.0,
                                        "meanPressureM": float(v.mean()) if len(v) else 0.0, "maxPressureM": float(v.max()) if len(v) else 0.0})
    tr = [n for n in R["nodes"] if n["st"] == 1]
    for i, n in enumerate(tr):
        v = R["U"][i, ok] - n["z"]
        out["transitPressure"].append({"transitNodeId": n["id"], "minPressureM": v.min(), "meanPressureM": v.mean(), "maxPressureM": v.max()})
    return out

if __name__ == "__main__":
    path, seed, n, ref = sys.argv[1], int(sys.argv[2]), int(sys.argv[3]), sys.argv[4]
    R = run(path, seed, n); mine = stats(R)
    refs = json.load(open(ref + ".stats.json")); raw = json.load(open(ref + ".raw.json"))
    print("draws/batches", R["draws"], R["batches"], "ref", raw["scenario_meta"]["generatedDrawCount"], raw["scenario_meta"]["generationBatchCount"])
    print("demand identical:", np.array_equal(R["dem"], np.array(raw["demand_by_terminal_lps"])),
          "counts identical:", np.array_equal(R["cnt"], np.array(raw["open_client_count_by_terminal"])))
    print("iterations identical:", np.array_equal(R["iters"], np.array(raw["iterations"])), "ok", R["ok"].sum())
    Xr = np.array(raw["flow_by_pipe_m3s"], dtype=float)
    print("flow max abs diff:", np.nanmax(np.abs(R["X"] - Xr)), "max |flow|", np.nanmax(np.abs(Xr)))
    worst = []
    for tab, recs in mine.items():
        for a, b in zip(recs, refs["statistics"][tab]):
            for k, v in a.items():
                w = b[k]
                if isinstance(v, str) or v is None or w is None:
                    if v != w and not (v is None and w is None): worst.append((float("inf"), tab, k, v, w))
                    continue
                ad = abs(v - w); ok_ = ad <= 1e-10 + 1e-8 * abs(w)
                if not ok_: worst.append((ad / max(abs(w), 1e-300), tab, k, v, w))
    print("violations of |a-b| <= 1e-10 + 1e-8|b|:", len(worst))
    for w in sorted(worst, key=lambda r: -r[0])[:8]: print("  ", w)
