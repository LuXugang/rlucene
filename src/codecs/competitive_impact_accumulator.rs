/*
 * Licensed to the Apache Software Foundation (ASF) under one or more
 * contributor license agreements.  See the NOTICE file distributed with
 * this work for additional information regarding copyright ownership.
 * The ASF licenses this file to You under the Apache License, Version 2.0
 * (the "License"); you may not use this file except in compliance with
 * the License.  You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
use crate::index::impact::Impact;
use std::collections::BTreeSet;

/// This struct accumulates the (freq, norm) pairs that may produce competitive scores.
pub struct CompetitiveImpactAccumulator {
    /// We speed up accumulation for common norm values with this array that maps
    /// norm values in -128..127 to the maximum frequency observed for these norm values.
    pub max_freqs: [i32; 256],
    /// Stores competitive (freq, norm) pairs for norm values that fall
    /// outside of -128..127. It is always empty with the default similarity,
    /// which encodes norms as bytes.
    pub other_freq_norm_pairs: BTreeSet<Impact>,
}

impl CompetitiveImpactAccumulator {
    /// Sole constructor.
    pub fn new() -> Self {
        CompetitiveImpactAccumulator {
            max_freqs: [0; 256],
            other_freq_norm_pairs: BTreeSet::new(),
        }
    }
    /// Reset to the same state it was in after creation.
    pub fn clear(&mut self) {
        self.max_freqs = [0; 256];
        self.other_freq_norm_pairs.clear();
        debug_assert!(self.assert_consistent());
    }

    /// Accumulate a (freq,norm) pair.updating this structure if there is no equivalent or more
    /// competitive entry already.
    pub fn add(&mut self, freq: i32, norm: i64) {
        if (i8::MIN as i64..=i8::MAX as i64).contains(&norm) {
            let idx = (norm as i8) as u8 as usize;
            self.max_freqs[idx] = self.max_freqs[idx].max(freq);
        } else {
            let entry = Impact::new(freq, norm);
            Self::add_entry(entry, &mut self.other_freq_norm_pairs);
        }
        debug_assert!(self.assert_consistent());
    }

    /// Merge `acc` into this.
    pub fn add_all(&mut self, acc: &Self) {
        for i in 0..256 {
            self.max_freqs[i] = self.max_freqs[i].max(acc.max_freqs[i]);
        }
        for entry in &acc.other_freq_norm_pairs {
            Self::add_entry(entry.clone(), &mut self.other_freq_norm_pairs);
        }
        debug_assert!(self.assert_consistent());
    }
    /// Replace the content of this with the provided `acc`.
    pub fn copy_from(&mut self, acc: &Self) {
        self.max_freqs.copy_from_slice(&acc.max_freqs);
        self.other_freq_norm_pairs = acc.other_freq_norm_pairs.clone();
        debug_assert!(self.assert_consistent());
    }

    /// Get the set of competitive (freq,norm) pairs, ordered by freq→norm.
    pub fn get_competitive_freq_norm_pairs(&self) -> Vec<Impact> {
        let mut impacts = Vec::new();
        let mut max_freq_for_lower_norms = 0;
        for (i, &freq) in self.max_freqs.iter().enumerate() {
            if freq > max_freq_for_lower_norms {
                impacts.push(Impact::new(freq, i as i8 as i64));
                max_freq_for_lower_norms = freq;
            }
        }

        if self.other_freq_norm_pairs.is_empty() {
            return impacts;
        }

        let mut freq_norm_pairs = self.other_freq_norm_pairs.clone();
        for imp in &impacts {
            Self::add_entry(imp.clone(), &mut freq_norm_pairs);
        }
        freq_norm_pairs.into_iter().collect()
    }

    fn add_entry(new_entry: Impact, freq_norm_pairs: &mut BTreeSet<Impact>) {
        if let Some(next) = freq_norm_pairs.range(&new_entry..).next() {
            if (next.norm as u64) <= (new_entry.norm as u64) {
                return;
            }
        }
        freq_norm_pairs.insert(new_entry.clone());
        // TODO: drain_filter is not stable in Rust 1.86.0
        let mut to_remove = Vec::new();
        for e in freq_norm_pairs.range(..&new_entry) {
            if (e.norm as u64) >= (new_entry.norm as u64) {
                to_remove.push(e.clone());
            } else {
                break;
            }
        }
        for e in to_remove {
            freq_norm_pairs.remove(&e);
        }
    }

    fn assert_consistent(&self) -> bool {
        let mut prev_freq = 0;
        let mut prev_norm = 0u64;
        for imp in &self.other_freq_norm_pairs {
            debug_assert!(imp.norm < i8::MIN as i64 || imp.norm > i8::MAX as i64);
            debug_assert!(prev_freq < imp.freq);
            debug_assert!(prev_norm < imp.norm as u64);
            prev_freq = imp.freq;
            prev_norm = imp.norm as u64;
        }
        true
    }
}

impl std::fmt::Display for CompetitiveImpactAccumulator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.get_competitive_freq_norm_pairs())
    }
}
#[cfg(test)]
mod tests {
    use crate::codecs::competitive_impact_accumulator::CompetitiveImpactAccumulator;
    use crate::index::impact::Impact;
    #[allow(dead_code)] // for quick search
    struct TestCompetitiveImpactAccumulator;
    #[test]
    fn test_basics() {
        let mut acc = CompetitiveImpactAccumulator::new();

        acc.add(3, 5);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(3, 5)]
        );
        acc.add(6, 11);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(3, 5), Impact::new(6, 11)]
        );
        acc.add(10, 13);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(3, 5), Impact::new(6, 11), Impact::new(10, 13)]
        );
        acc.add(1, 2);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![
                Impact::new(1, 2),
                Impact::new(3, 5),
                Impact::new(6, 11),
                Impact::new(10, 13)
            ]
        );

        acc.add(7, 9);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![
                Impact::new(1, 2),
                Impact::new(3, 5),
                Impact::new(7, 9),
                Impact::new(10, 13)
            ]
        );

        acc.add(8, 2);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(8, 2), Impact::new(10, 13)]
        );
    }
    #[test]
    fn test_extreme_norms() {
        let mut acc = CompetitiveImpactAccumulator::new();
        let mut expected = Vec::new();

        acc.add(3, 5);
        expected.push(Impact::new(3, 5));
        assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

        acc.add(10, 10000);
        expected.push(Impact::new(10, 10000));
        assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

        acc.add(5, 200);
        expected.insert(1, Impact::new(5, 200));
        assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

        acc.add(20, -100);
        expected.push(Impact::new(20, -100));
        assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);

        acc.add(30, -3);
        expected.push(Impact::new(30, -3));
        assert_eq!(acc.get_competitive_freq_norm_pairs(), expected);
    }

    #[test]
    fn test_copy_and_merge() {
        let mut acc = CompetitiveImpactAccumulator::new();
        let mut copied_acc = CompetitiveImpactAccumulator::new();
        let mut merged_acc = CompetitiveImpactAccumulator::new();

        acc.add(3, 5);
        copied_acc.copy_from(&acc);
        assert_eq!(
            copied_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        merged_acc.add_all(&acc);
        assert_eq!(
            merged_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        acc.add(10, 10000);
        copied_acc.copy_from(&acc);
        assert_eq!(
            copied_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        merged_acc.clear();
        merged_acc.add_all(&acc);
        assert_eq!(
            merged_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        acc.add(5, 200);
        copied_acc.copy_from(&acc);
        assert_eq!(
            copied_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        merged_acc.clear();
        merged_acc.add_all(&acc);
        assert_eq!(
            merged_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        acc.add(20, -100);
        copied_acc.copy_from(&acc);
        assert_eq!(
            copied_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        merged_acc.clear();
        merged_acc.add_all(&acc);
        assert_eq!(
            merged_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        acc.add(30, -3);
        copied_acc.copy_from(&acc);
        assert_eq!(
            copied_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );

        merged_acc.clear();
        merged_acc.add_all(&acc);
        assert_eq!(
            merged_acc.get_competitive_freq_norm_pairs(),
            acc.get_competitive_freq_norm_pairs()
        );
    }

    #[test]
    fn test_omit_freqs() {
        let mut acc = CompetitiveImpactAccumulator::new();
        acc.add(1, 5);
        acc.add(1, 7);
        acc.add(1, 4);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(1, 4)]
        );
    }

    #[test]
    fn test_omit_norms() {
        let mut acc = CompetitiveImpactAccumulator::new();
        acc.add(5, 1);
        acc.add(7, 1);
        acc.add(4, 1);
        assert_eq!(
            acc.get_competitive_freq_norm_pairs(),
            vec![Impact::new(7, 1)]
        );
    }
}
