import numpy as np
M32 = 0xFFFFFFFF
INIT_A, MULT_A, INIT_B, MULT_B = 0x43b0d7e5, 0x931e8875, 0x8b51f9dd, 0x58f38ded
MIX_L, MIX_R, XSHIFT, POOL = 0xca01f9dd, 0x4973f715, 16, 4
def seed_words(seed):
    if seed == 0: return [0]
    w = []
    while seed: w.append(seed & M32); seed >>= 32
    return w
def seedseq_pool(seed):
    ent = seed_words(seed); h = [INIT_A]
    def hashmix(v):
        v = (v ^ h[0]) & M32; h[0] = (h[0] * MULT_A) & M32; v = (v * h[0]) & M32; return v ^ (v >> XSHIFT)
    def mix(x, y):
        r = (MIX_L * x - MIX_R * y) & M32; return r ^ (r >> XSHIFT)
    m = [hashmix(ent[i] if i < len(ent) else 0) for i in range(POOL)]
    for s in range(POOL):
        for d in range(POOL):
            if s != d: m[d] = mix(m[d], hashmix(m[s]))
    for s in range(POOL, len(ent)):
        for d in range(POOL): m[d] = mix(m[d], hashmix(ent[s]))
    return m
def generate_state_u64(pool, n):
    h = INIT_B; out32 = []
    for i in range(2 * n):
        v = pool[i % POOL]; v ^= h; h = (h * MULT_B) & M32; v = (v * h) & M32; v ^= v >> XSHIFT; out32.append(v)
    return [out32[2*i] | (out32[2*i+1] << 32) for i in range(n)]
MULT = 0x2360ED051FC65DA44385DF649FCCF645; M128 = (1 << 128) - 1
class PCG64:
    def __init__(self, seed):
        v = generate_state_u64(seedseq_pool(seed), 4)
        initstate = (v[0] << 64) | v[1]; initseq = (v[2] << 64) | v[3]
        self.inc = ((initseq << 1) | 1) & M128; self.state = 0
        self.step(); self.state = (self.state + initstate) & M128; self.step()
    def step(self): self.state = (self.state * MULT + self.inc) & M128
    def next_u64(self):
        self.step(); s = self.state; hi, lo = s >> 64, s & ((1 << 64) - 1)
        x = hi ^ lo; r = hi >> 58
        return ((x >> r) | (x << ((64 - r) & 63))) & ((1 << 64) - 1)
    def random(self): return (self.next_u64() >> 11) * (1.0 / 9007199254740992.0)
if __name__ == "__main__":
    ok = True
    for seed in [0, 1, 42, 12345, 2**32 - 1, 2**32, 2**40 + 7, 2**64 + 3]:
        a = np.random.default_rng(seed).random(1000); p = PCG64(seed); b = np.array([p.random() for _ in range(1000)])
        same = np.array_equal(a, b); ok &= same; print(seed, same, a[:3])
    # order of draws in rng.random((C, D)) is C-order (client-major)
    r = np.random.default_rng(42); M = r.random((3, 5)); p = PCG64(42); flat = [p.random() for _ in range(15)]
    print("C-order:", np.array_equal(M.ravel(), flat)); print("ALL OK" if ok else "MISMATCH")
    print("state words seed 42:", [hex(x) for x in generate_state_u64(seedseq_pool(42), 4)])
    print("first u64 seed 42:", hex(PCG64(42).next_u64()), np.random.default_rng(42).bit_generator.random_raw(1))
