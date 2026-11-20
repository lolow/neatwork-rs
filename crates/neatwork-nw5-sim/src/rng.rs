//! numpy `default_rng(seed)`: `SeedSequence` + PCG64 (XSL-RR 128/64) +
//! `random()` doubles, bit for bit (spec section 3.1).

const INIT_A: u32 = 0x43b0_d7e5;
const MULT_A: u32 = 0x931e_8875;
const INIT_B: u32 = 0x8b51_f9dd;
const MULT_B: u32 = 0x58f3_8ded;
const MIX_L: u32 = 0xca01_f9dd;
const MIX_R: u32 = 0x4973_f715;
const XSHIFT: u32 = 16;
const POOL: usize = 4;
const PCG_MULT: u128 = 0x2360_ED05_1FC6_5DA4_4385_DF64_9FCC_F645;

/// Little-endian 32-bit words of the seed (`[0]` for 0).
fn seed_words(seed: u64) -> Vec<u32> {
    if seed == 0 {
        return vec![0];
    }
    let mut words = Vec::new();
    let mut s = seed;
    while s != 0 {
        words.push((s & 0xFFFF_FFFF) as u32);
        s >>= 32;
    }
    words
}

/// `SeedSequence.mix_entropy` with an empty spawn key.
fn seed_pool(seed: u64) -> [u32; POOL] {
    let entropy = seed_words(seed);
    let mut hash_const = INIT_A;
    let mut hashmix = |v: u32| -> u32 {
        let mut v = v ^ hash_const;
        hash_const = hash_const.wrapping_mul(MULT_A);
        v = v.wrapping_mul(hash_const);
        v ^ (v >> XSHIFT)
    };
    let mix = |x: u32, y: u32| -> u32 {
        let r = MIX_L.wrapping_mul(x).wrapping_sub(MIX_R.wrapping_mul(y));
        r ^ (r >> XSHIFT)
    };
    let mut pool = [0u32; POOL];
    for (i, p) in pool.iter_mut().enumerate() {
        *p = hashmix(entropy.get(i).copied().unwrap_or(0));
    }
    for s in 0..POOL {
        for d in 0..POOL {
            if s != d {
                let h = hashmix(pool[s]);
                pool[d] = mix(pool[d], h);
            }
        }
    }
    for &e in entropy.iter().skip(POOL) {
        for p in pool.iter_mut() {
            let h = hashmix(e);
            *p = mix(*p, h);
        }
    }
    pool
}

/// `SeedSequence.generate_state(n, uint64)`.
fn generate_state(pool: &[u32; POOL], n: usize) -> Vec<u64> {
    let mut hash_const = INIT_B;
    let mut out32 = Vec::with_capacity(2 * n);
    for i in 0..2 * n {
        let mut v = pool[i % POOL] ^ hash_const;
        hash_const = hash_const.wrapping_mul(MULT_B);
        v = v.wrapping_mul(hash_const);
        v ^= v >> XSHIFT;
        out32.push(v);
    }
    out32
        .chunks(2)
        .map(|c| u64::from(c[0]) | (u64::from(c[1]) << 32))
        .collect()
}

/// numpy's PCG64 bit generator.
#[derive(Clone, Debug)]
pub(crate) struct Pcg64 {
    state: u128,
    inc: u128,
}

impl Pcg64 {
    /// `numpy.random.default_rng(seed)`.
    pub(crate) fn new(seed: u64) -> Pcg64 {
        let v = generate_state(&seed_pool(seed), 4);
        let initstate = (u128::from(v[0]) << 64) | u128::from(v[1]);
        let initseq = (u128::from(v[2]) << 64) | u128::from(v[3]);
        let mut rng = Pcg64 {
            state: 0,
            inc: (initseq << 1) | 1,
        };
        rng.step();
        rng.state = rng.state.wrapping_add(initstate);
        rng.step();
        rng
    }

    fn step(&mut self) {
        self.state = self.state.wrapping_mul(PCG_MULT).wrapping_add(self.inc);
    }

    /// Next 64-bit output (the state advances first).
    pub(crate) fn next_u64(&mut self) -> u64 {
        self.step();
        let hi = (self.state >> 64) as u64;
        let lo = self.state as u64;
        (hi ^ lo).rotate_right((hi >> 58) as u32)
    }

    /// `Generator.random()`: a double in [0, 1).
    pub(crate) fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seed_42_vectors() {
        let v = generate_state(&seed_pool(42), 4);
        assert_eq!(
            v,
            vec![
                0x9f1e2e6dcd540ab7,
                0xd57873dc79fb94b6,
                0x7d282a1b64d420b7,
                0x336579714692d5ff
            ]
        );
        let mut r = Pcg64::new(42);
        assert_eq!(r.next_u64(), 0xc621fbcd16d92688);
        let mut r = Pcg64::new(42);
        assert_eq!(r.next_f64(), 0.7739560485559633);
        assert_eq!(r.next_f64(), 0.4388784397520523);
        assert_eq!(r.next_f64(), 0.8585979199113825);
    }

    #[test]
    fn numpy_draws_for_other_seeds() {
        let cases: [(u64, [f64; 3]); 6] = [
            (
                0,
                [0.6369616873214543, 0.2697867137638703, 0.04097352393619469],
            ),
            (
                1,
                [0.5118216247002567, 0.9504636963259353, 0.14415961271963373],
            ),
            (
                12345,
                [0.22733602246716966, 0.31675833970975287, 0.7973654573327341],
            ),
            (
                (1 << 32) - 1,
                [0.2519435680449965, 0.21906579628685663, 0.28728472043517017],
            ),
            (
                1 << 32,
                [0.8897387912781343, 0.5571380502062263, 0.8009080868919721],
            ),
            (
                (1 << 40) + 7,
                [0.829812891829413, 0.25530515834910417, 0.15379071585298754],
            ),
        ];
        for (seed, expected) in cases {
            let mut r = Pcg64::new(seed);
            for e in expected {
                assert_eq!(r.next_f64(), e, "seed {seed}");
            }
        }
    }

    #[test]
    fn large_seed_uses_two_words() {
        assert_eq!(seed_words(1 << 32), vec![0, 1]);
        assert_eq!(seed_words(0), vec![0]);
    }
}
