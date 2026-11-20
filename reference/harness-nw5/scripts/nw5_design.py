"""Independent re-implementation of NW5 makeReinfDsg (uniform demand, bare TPO network).
Used only to check whether example .dsg files are reproducible."""
import sys, math
import numpy as np
from scipy.optimize import linprog
from scipy.stats import binom


def parse_csv_section(lines, name):
    out, on = [], False
    for ln in lines:
        s = ln.strip()
        if s.startswith('!'):
            on = s.startswith('!' + name)
            continue
        if on and s:
            out.append([f.strip() for f in s.split(',')])
    return out


def run(tpo, diamdb, orifdb, overrides):
    lines = open(tpo, encoding='latin-1').read().splitlines()
    nodes = parse_csv_section(lines, 'Nodes')
    pipes = parse_csv_section(lines, 'Pipes')
    props = {r[0]: float(r[1]) for r in parse_csv_section(lines, 'Default properties')}
    props.update(overrides)
    P = dict(targetflow=0.0, opentaps=1, pump_pressure=0, pressure=2, secu=1.0, vmax=1.5,
             vmax_secu=0.95, servicequal=0.65, pipelength=6.0, uniform_dem=1, bounded_flow=0,
             orifcoef=1.0, faucetcoef=1.0, **{'lambda': 1.781}, nu=4.781, bescal=8.9396e-4)
    P.update(props)
    lam, nu, bescal = P['lambda'], P['nu'], P['bescal']
    # catalogue
    D = []
    for ln in open(diamdb, encoding='latin-1'):
        f = [x.strip() for x in ln.strip().split(',')]
        if len(f) < 7:
            continue
        D.append(dict(nom=f[0], sdr=f[1], diam=float(f[2]), cost=float(f[3]), pressure=float(f[4])))
    D.sort(key=lambda d: d['diam'])
    for i, d in enumerate(D):
        d['ID'] = 'D%03d' % (i + 1)
        d['resist'] = bescal * d['diam'] ** (-nu)
    orifs = [float(x) for x in open(orifdb) if x.strip()]
    # nodes
    N = {r[0]: dict(haut=float(r[1]), nbrob=int(float(r[4])), statut=int(r[5])) for r in nodes}
    src = [k for k, v in N.items() if v['statut'] == 0][0]
    Hs = N[src]['haut'] + P['pump_pressure']
    A = [dict(pred=r[0], succ=r[1], lon=max(float(r[2]), 0.01)) for r in pipes]
    for a in A:
        a['pmax'] = Hs - min(N[a['pred']]['haut'], N[a['succ']]['haut'])
        a['haut'] = N[a['succ']]['haut'] - Hs
        a['statut'] = N[a['succ']]['statut']
        a['nbrob'] = N[a['succ']]['nbrob']
        a['pressure'] = P['pressure'] if a['statut'] == 2 else 0.0
    A.sort(key=lambda a: (a['statut'], a['succ']))
    nA = len(A)
    idx = {a['succ']: i for i, a in enumerate(A)}
    # path matrix M[i,j]=1 if arc i on path to succ(j)
    M = np.zeros((nA, nA))
    for j, a in enumerate(A):
        k = j
        while True:
            M[k, j] = 1
            p = A[k]['pred']
            if p not in idx:
                break
            k = idx[p]
    nbrob = np.array([a['nbrob'] for a in A], float)
    nbdep = M @ nbrob

    def loads(qos):
        p = P['opentaps']
        ld = np.ones(nA)
        for i in range(nA):
            if nbdep[i] <= 1:
                continue
            n = nbdep[i] - 1
            if p == 1:
                ld[i] = 1 + n
                continue
            k = binom.ppf(qos, n, p)
            p2 = binom.cdf(k, n, p)
            p1 = binom.cdf(k - 1, n, p)
            if p2 > p1 and p1 > 0:
                ld[i] = k + (qos - p1) / (p2 - p1)
            elif p2 == p1 and p1 > 0:
                ld[i] = k
            else:
                ld[i] = 1
        return ld
    lf = loads(P['servicequal'])
    lfv = np.maximum(loads(P['vmax_secu']), lf)
    Q = np.maximum(lf * P['targetflow'], 1e-12) / 1000  # m3/s, Qmax per arc (TL constant per row)
    Qv = np.maximum(lfv * P['targetflow'], 1e-12) / 1000
    nD = len(D)
    minDiam = 2 * np.sqrt(Qv / (math.pi * P['vmax']))
    allowed = np.array([[d['pressure'] >= A[i]['pmax'] and d['diam'] >= minDiam[i] for i in range(nA)] for d in D])  # nD x nA
    c = np.zeros(nA * nD + nA)
    for i in range(nA):
        for j in range(nD):
            c[i * nD + j] = A[i]['lon'] * D[j]['cost']
    Aeq = np.zeros((nA, nA * nD + nA)); beq = np.zeros(nA)
    for i in range(nA):
        Aeq[i, i * nD:(i + 1) * nD] = 1
        Aeq[i, nA * nD + i] = -1
    term = np.array([a['statut'] == 2 for a in A])
    b = -np.array([a['haut'] for a in A])
    b[term] /= max(P['secu'], 2.2e-16)
    cible = P['targetflow'] / 1000
    if P['bounded_flow'] == 1:
        b[term] -= np.array([a['pressure'] for a in A])[term]
    if P['uniform_dem'] == 1:
        b[term] -= cible ** 2 / P['faucetcoef']
    loc = np.zeros((nA, nA * nD))
    for i in range(nA):
        for j in range(nD):
            loc[i, i * nD + j] = D[j]['resist'] * Q[i] ** lam * A[i]['lon']
    Aub = np.zeros((nA, nA * nD + nA))
    Aub[:, :nA * nD] = M.T @ loc
    bounds = []
    for i in range(nA):
        for j in range(nD):
            bounds.append((0, 1 if allowed[j, i] else 0))
    bounds += [(1, 1)] * nA
    res = linprog(c, A_ub=Aub, b_ub=b, A_eq=Aeq, b_eq=beq, bounds=bounds, method='highs')
    assert res.status == 0, res.message
    X = res.x[:nA * nD].reshape(nA, nD).T.copy()  # nD x nA
    X[np.abs(X) < 1e-7] = 0
    X = X / X.sum(axis=0)
    out = []
    rnew = np.zeros(nA)
    for i in range(nA):
        J = [j for j in range(nD) if X[j, i] > 0]
        if len(J) > 2:
            raise SystemExit('mixture >2: implement reducePipeMixture')
        J.sort(key=lambda j: -D[j]['diam'])
        L = A[i]['lon']
        if len(J) == 1:
            segs = [(L, J[0])]
        else:
            L1 = L * X[J[0], i]
            if P['pipelength'] > 0.1:
                L1 = min(L, math.ceil(L1 / P['pipelength']) * P['pipelength'])
            segs = [(L1, J[0]), (L - L1, J[1])]
            X[J[0], i] = L1 / L; X[J[1], i] = (L - L1) / L
        rnew[i] = sum(X[j, i] * D[j]['resist'] for j in range(nD))
        out.append((A[i]['pred'], A[i]['succ'], L, [(round(l, 6), D[j]['ID']) for l, j in segs], X[J[0], i] * L, res.x[i*nD:(i+1)*nD]))
    pathLoss = M.T @ (np.array([a['lon'] for a in A]) * rnew * Q ** lam)
    avail = -np.array([a['haut'] for a in A]); avail[term] /= P['secu']
    req = np.zeros(nA)
    if P['bounded_flow'] == 1:
        req[term] = np.array([a['pressure'] for a in A])[term]
    if P['uniform_dem'] == 1:
        req[term] += cible ** 2 / P['faucetcoef']
    margin = avail - pathLoss - req
    print('LP cost', res.fun, ' final cost', sum(A[i]['lon'] * sum(X[j, i] * D[j]['cost'] for j in range(nD)) for i in range(nA)))
    for i, o in enumerate(out):
        lp_l = {D[j]['ID']: round(o[5][j]*o[2], 4) for j in range(nD) if o[5][j] > 1e-9}
        print(o[0], o[1], o[2], o[3], 'LP lengths', lp_l, 'lf=%.10g' % lf[i], 'margin=%.6g' % margin[i])
    if P['uniform_dem'] == 1 and P['bounded_flow'] == 0:
        for i in range(nA):
            if term[i] and margin[i] >= 0.1:
                ideal = min(1, P['orifcoef'] * cible ** 0.5 * margin[i] ** -0.25)
                cand = orifs + [1.0]
                com = min(cand, key=lambda d: abs(ideal ** -4 - d ** -4))
                print('orifice', A[i]['succ'], repr(ideal), com)


if __name__ == '__main__':
    tpo, db, orf = sys.argv[1:4]
    ov = {}
    for kv in sys.argv[4:]:
        k, v = kv.split('=')
        ov[k] = float(v)
    run(tpo, db, orf, ov)
