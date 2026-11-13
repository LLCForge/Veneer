//! Walker/Vose alias table: O(n) to build, O(1) per weighted draw.
//!
//! Integer-only, so a table built from the same weights draws the same index for the same RNG state on
//! every platform (native and WASM).

use rand_chacha::rand_core::RngCore;
use rand_chacha::ChaCha8Rng;

#[derive(Debug)]
pub(crate) struct Alias {
    /// Acceptance threshold per column, in `0..=total`.
    prob: Vec<u64>,
    alias: Vec<usize>,
    total: u64,
}

impl Alias {
    /// `None` when every weight is zero or the weights overflow `u64`.
    pub fn new(weights: impl IntoIterator<Item = u64>) -> Option<Self> {
        let weights: Vec<u64> = weights.into_iter().collect();
        let total = weights
            .iter()
            .try_fold(0u64, |acc, w| acc.checked_add(*w))?;
        if total == 0 {
            return None;
        }
        let n = weights.len() as u128;
        let full = u128::from(total);
        // Column i holds weight w_i * n; a full column holds `total`.
        let mut scaled: Vec<u128> = weights.iter().map(|w| u128::from(*w) * n).collect();
        let mut prob = vec![total; weights.len()];
        let mut alias: Vec<usize> = (0..weights.len()).collect();
        let (mut small, mut large): (Vec<usize>, Vec<usize>) =
            (0..weights.len()).partition(|&i| scaled[i] < full);
        while let (Some(s), Some(l)) = (small.pop(), large.pop()) {
            // scaled[s] < full <= u64::MAX + 1, and is below total, so it fits u64.
            prob[s] = u64::try_from(scaled[s]).ok()?;
            alias[s] = l;
            scaled[l] = scaled[l] + scaled[s] - full;
            if scaled[l] < full {
                small.push(l);
            } else {
                large.push(l);
            }
        }
        Some(Alias { prob, alias, total })
    }

    pub fn sample(&self, rng: &mut ChaCha8Rng) -> usize {
        // shortcut: modulo bias is n / 2^64 and total / 2^64, invisible at dataset scale; switch to rejection sampling if weights ever approach 2^32
        let column = (rng.next_u64() % self.prob.len() as u64) as usize;
        let coin = rng.next_u64() % self.total;
        if coin < self.prob[column] {
            column
        } else {
            self.alias[column]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand_chacha::rand_core::SeedableRng;

    /// The table encodes every weight exactly: summing each index's share over all columns gives w_i * n.
    #[test]
    fn table_is_exact() {
        for weights in [
            vec![1u64, 2, 3, 0, 10],
            vec![7],
            vec![0, 0, 5],
            vec![1_000_000, 1, 1, 999],
            (1..=97).collect(),
        ] {
            let t = Alias::new(weights.iter().copied()).unwrap();
            let n = weights.len() as u128;
            let mut share = vec![0u128; weights.len()];
            for col in 0..weights.len() {
                share[col] += u128::from(t.prob[col]);
                share[t.alias[col]] += u128::from(t.total - t.prob[col]);
            }
            for (i, w) in weights.iter().enumerate() {
                assert_eq!(share[i], u128::from(*w) * n, "{weights:?} index {i}");
            }
        }
    }

    #[test]
    fn zero_weights_are_never_drawn() {
        let t = Alias::new([0, 3, 0, 1]).unwrap();
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        let mut seen = [0u32; 4];
        for _ in 0..40_000 {
            seen[t.sample(&mut rng)] += 1;
        }
        assert_eq!((seen[0], seen[2]), (0, 0));
        assert!((28_000..32_000).contains(&seen[1]), "{seen:?}");
    }

    #[test]
    fn empty_or_overflowing_weights_are_rejected() {
        assert!(Alias::new([0, 0]).is_none());
        assert!(Alias::new(std::iter::empty()).is_none());
        assert!(Alias::new([u64::MAX, 1]).is_none());
    }
}
