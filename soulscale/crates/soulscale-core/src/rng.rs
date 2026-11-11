//! CPython-compatible random number generator.
//!
//! This is a bit-exact port of CPython's `random.Random` (MT19937 + the pure-Python
//! helpers in `Lib/random.py`) for every call the v1.0.1 engine makes:
//! `randint`, `random`, `uniform`, `shuffle`, `choice`, and the builtin `sum()` of floats
//! (CPython 3.12+ uses Neumaier compensated summation).
//!
//! Why bother? Because with the same integer seed, the Rust engine and the original
//! Python engine draw identical dice, shuffles and AI noise. That lets the test-suite
//! replay hundreds of seeded games through both and diff the logs line by line,
//! proving the port is faithful rather than hoping it is.
//!
//! No `std::time`, no OS calls: safe for wasm32 as well as native.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

/// Mersenne Twister with CPython's seeding and helper semantics.
#[derive(Clone)]
pub struct PyRandom {
    mt: [u32; N],
    index: usize,
}

impl std::fmt::Debug for PyRandom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PyRandom").field("index", &self.index).finish_non_exhaustive()
    }
}

impl PyRandom {
    /// Equivalent to `random.Random(seed)` for a non-negative integer seed.
    pub fn new(seed: u64) -> Self {
        // CPython splits abs(seed) into little-endian 32-bit words; zero uses one word [0].
        let lo = (seed & 0xffff_ffff) as u32;
        let hi = (seed >> 32) as u32;
        let key: Vec<u32> = if hi == 0 { vec![lo] } else { vec![lo, hi] };
        let mut r = PyRandom { mt: [0; N], index: N + 1 };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(prev ^ (prev >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let mut i: usize = 1;
        let mut j: usize = 0;
        let mut k = N.max(key.len());
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    /// Raw 32-bit output (`genrand_uint32`).
    pub fn next_u32(&mut self) -> u32 {
        if self.index >= N {
            for kk in 0..N {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[(kk + 1) % N] & LOWER_MASK);
                let mag = if y & 1 == 1 { MATRIX_A } else { 0 };
                self.mt[kk] = self.mt[(kk + M) % N] ^ (y >> 1) ^ mag;
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `getrandbits(k)` for 1 <= k <= 32.
    fn getrandbits(&mut self, k: u32) -> u32 {
        debug_assert!((1..=32).contains(&k));
        self.next_u32() >> (32 - k)
    }

    /// `_randbelow_with_getrandbits(n)`: uniform in `0..n`, `n >= 1`.
    pub fn randbelow(&mut self, n: u32) -> u32 {
        assert!(n >= 1, "randbelow(0)");
        let k = 32 - n.leading_zeros(); // n.bit_length()
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// `randint(a, b)` inclusive.
    pub fn randint(&mut self, a: i32, b: i32) -> i32 {
        assert!(b >= a, "randint with empty range");
        a + self.randbelow((b - a + 1) as u32) as i32
    }

    /// One six-sided die.
    pub fn d6(&mut self) -> u8 {
        self.randint(1, 6) as u8
    }

    /// `random()` in `[0, 1)` with 53-bit resolution.
    pub fn random(&mut self) -> f64 {
        let a = (self.next_u32() >> 5) as f64;
        let b = (self.next_u32() >> 6) as f64;
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `uniform(a, b)`.
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// `shuffle(x)` in place.
    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        if x.len() < 2 {
            return;
        }
        for i in (1..x.len()).rev() {
            let j = self.randbelow((i + 1) as u32) as usize;
            x.swap(i, j);
        }
    }

    /// `choice(seq)` returning an index. Panics on an empty slice, as CPython raises.
    pub fn choice_index(&mut self, len: usize) -> usize {
        assert!(len > 0, "choice from empty sequence");
        self.randbelow(len as u32) as usize
    }
}

/// The builtin `sum()` over floats starting from int 0, as CPython 3.12+ computes it
/// (Neumaier compensated summation). Used where the AI totals its choice weights.
pub fn py_sum_floats(values: &[f64]) -> f64 {
    let mut iter = values.iter();
    let Some(&first) = iter.next() else {
        return 0.0;
    };
    // int 0 + first float == first (exact).
    let mut f = first;
    let mut c = 0.0f64;
    for &x in iter {
        let t = f + x;
        if f.abs() >= x.abs() {
            c += (f - t) + x;
        } else {
            c += (x - t) + f;
        }
        f = t;
    }
    if c != 0.0 && c.is_finite() {
        f += c;
    }
    f
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values produced by CPython 3.13:
    /// `r = random.Random(12345); [r.getrandbits(32) for _ in range(3)]`.
    #[test]
    fn matches_cpython_raw_stream_seed_12345() {
        let mut r = PyRandom::new(12345);
        let got: Vec<u32> = (0..3).map(|_| r.next_u32()).collect();
        assert_eq!(got, vec![1789368711, 3146859322, 43676229]);
    }

    #[test]
    fn randint_stays_in_range() {
        let mut r = PyRandom::new(7);
        for _ in 0..10_000 {
            let v = r.randint(1, 6);
            assert!((1..=6).contains(&v));
        }
    }

    #[test]
    fn neumaier_matches_naive_on_simple_input() {
        assert_eq!(py_sum_floats(&[1.0, 2.0, 3.5]), 6.5);
        assert_eq!(py_sum_floats(&[]), 0.0);
    }
}
