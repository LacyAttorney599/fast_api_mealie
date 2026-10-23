//! `random.Random` de CPython (Mersenne Twister MT19937) et sa méthode
//! `sample()` : pour une même semaine, le planificateur IA tire exactement le
//! même échantillon de recettes que la version Python.

use std::collections::HashSet;

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

pub struct PyRandom {
    mt: [u32; N],
    index: usize,
}

impl PyRandom {
    /// `random.Random(seed)` pour un entier positif ou nul.
    pub fn new(seed: u64) -> Self {
        // CPython découpe l'entier en mots de 32 bits, poids faible d'abord.
        let mut key = vec![seed as u32];
        if seed >> 32 != 0 {
            key.push((seed >> 32) as u32);
        }
        let mut rng = Self {
            mt: [0; N],
            index: N,
        };
        rng.init_by_array(&key);
        rng
    }

    fn init_genrand(&mut self, seed: u32) {
        self.mt[0] = seed;
        for i in 1..N {
            let previous = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(i as u32);
        }
        self.index = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let (mut i, mut j) = (1, 0);
        for _ in 0..N.max(key.len()) {
            let previous = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
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
        }
        for _ in 0..N - 1 {
            let previous = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
        }
        self.mt[0] = 0x8000_0000;
    }

    fn genrand_u32(&mut self) -> u32 {
        let twist = |upper: u32, lower: u32| {
            let y = (upper & UPPER_MASK) | (lower & LOWER_MASK);
            (y >> 1) ^ if y & 1 == 0 { 0 } else { MATRIX_A }
        };
        if self.index >= N {
            for kk in 0..N - M {
                self.mt[kk] = self.mt[kk + M] ^ twist(self.mt[kk], self.mt[kk + 1]);
            }
            for kk in N - M..N - 1 {
                self.mt[kk] = self.mt[kk + M - N] ^ twist(self.mt[kk], self.mt[kk + 1]);
            }
            self.mt[N - 1] = self.mt[M - 1] ^ twist(self.mt[N - 1], self.mt[0]);
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// `_randbelow_with_getrandbits(n)`, pour 0 < n < 2³².
    fn randbelow(&mut self, n: usize) -> usize {
        let bits = usize::BITS - n.leading_zeros();
        assert!((1..=32).contains(&bits), "randbelow: n hors limites");
        loop {
            let r = (self.genrand_u32() >> (32 - bits)) as usize;
            if r < n {
                return r;
            }
        }
    }

    /// `sample(population, k)` de CPython 3.12 : même choix d'algorithme
    /// (tirage dans une copie ou rejet des doublons) et mêmes tirages.
    pub fn sample<T: Clone>(&mut self, population: &[T], k: usize) -> Vec<T> {
        let n = population.len();
        assert!(k <= n, "Sample larger than population or is negative");
        let mut setsize = 21;
        if k > 5 {
            let exponent = ((k * 3) as f64).ln() / 4f64.ln();
            setsize += 4usize.pow(exponent.ceil() as u32);
        }

        let mut result = Vec::with_capacity(k);
        if n <= setsize {
            let mut pool = population.to_vec();
            for i in 0..k {
                let j = self.randbelow(n - i);
                result.push(pool[j].clone());
                pool[j] = pool[n - i - 1].clone();
            }
        } else {
            let mut selected = HashSet::with_capacity(k);
            for _ in 0..k {
                let mut j = self.randbelow(n);
                while !selected.insert(j) {
                    j = self.randbelow(n);
                }
                result.push(population[j].clone());
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Valeurs produites par `random.Random(seed).sample(range(n), k)` sous CPython 3.12.
    #[test]
    fn sample_matches_cpython() {
        let cases: [(u64, usize, usize, &[usize]); 7] = [
            (
                202642,
                30,
                20,
                &[
                    24, 10, 14, 20, 3, 16, 13, 27, 23, 22, 0, 15, 25, 9, 2, 29, 21, 4, 12, 19,
                ],
            ),
            (
                202601,
                100,
                40,
                &[
                    0, 9, 82, 37, 59, 58, 26, 99, 56, 72, 94, 73, 40, 36, 84, 5, 93, 32, 44, 1, 19,
                    28, 48, 54, 96, 74, 62, 75, 66, 69, 67, 4, 88, 95, 91, 33, 49, 29, 15, 31,
                ],
            ),
            (
                202652,
                22,
                20,
                &[
                    19, 4, 15, 6, 0, 18, 8, 14, 2, 9, 20, 1, 5, 12, 13, 21, 3, 11, 7, 10,
                ],
            ),
            (
                202700,
                25,
                21,
                &[
                    0, 17, 20, 1, 19, 14, 10, 23, 5, 15, 9, 11, 12, 4, 13, 21, 22, 3, 7, 16, 2,
                ],
            ),
            (
                202643,
                60,
                26,
                &[
                    59, 44, 31, 23, 48, 1, 34, 17, 41, 50, 0, 24, 5, 4, 10, 54, 37, 43, 8, 36, 42,
                    53, 25, 3, 15, 18,
                ],
            ),
            (0, 10, 3, &[6, 9, 0]),
            (
                5,
                50,
                22,
                &[
                    39, 16, 47, 22, 44, 41, 33, 1, 29, 15, 3, 10, 7, 23, 30, 40, 24, 6, 34, 0, 36,
                    32,
                ],
            ),
        ];
        for (seed, n, k, expected) in cases {
            let population: Vec<usize> = (0..n).collect();
            assert_eq!(
                PyRandom::new(seed).sample(&population, k),
                expected,
                "seed {seed}"
            );
        }
    }
}
