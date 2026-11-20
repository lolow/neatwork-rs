//! MATLAB `twister` generator: MT19937 with `genrand_res53` doubles
//! (`rust/nw5/specs/design.md` section 3.2).
//!
//! `rng(s)` in MATLAB is `init_genrand(s)` for `s > 0` and
//! `init_genrand(5489)` for `s = 0` (`rng('default')`, the state of a new
//! MATLAB session). The stream equals NumPy `RandomState(s)`.

const N: usize = 624;
const M: usize = 397;

/// MT19937 generator seeded as MATLAB `rng(seed, 'twister')`.
#[derive(Clone, Debug)]
pub struct Twister {
    state: Vec<u32>,
    index: usize,
}

impl Twister {
    /// Generator in the state of MATLAB `rng(seed)`.
    pub fn new(seed: u32) -> Twister {
        let s = if seed == 0 { 5489 } else { seed };
        let mut state = vec![0u32; N];
        state[0] = s;
        for i in 1..N {
            let prev = state[i - 1];
            state[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        Twister { state, index: N }
    }

    fn twist(&mut self) {
        for i in 0..N {
            let y = (self.state[i] & 0x8000_0000) | (self.state[(i + 1) % N] & 0x7fff_ffff);
            let mut v = self.state[(i + M) % N] ^ (y >> 1);
            if y & 1 != 0 {
                v ^= 0x9908_b0df;
            }
            self.state[i] = v;
        }
        self.index = 0;
    }

    /// Next 32-bit output (`genrand_int32`).
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// Next double in [0, 1) as MATLAB `rand` (`genrand_res53`).
    pub fn next_f64(&mut self) -> f64 {
        let a = f64::from(self.next_u32() >> 5);
        let b = f64::from(self.next_u32() >> 6);
        (a * 67_108_864.0 + b) / 9_007_199_254_740_992.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_matlab_rng() {
        // MATLAB rng(42); rand  and  rng(0); rand  (= NumPy RandomState(42), (5489))
        let mut t = Twister::new(42);
        assert_eq!(t.next_f64(), 0.3745401188473625);
        assert_eq!(t.next_f64(), 0.9507143064099162);
        let mut t = Twister::new(0);
        assert_eq!(t.next_f64(), 0.8147236863931789);
    }
}
