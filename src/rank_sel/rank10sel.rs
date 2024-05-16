use common_traits::SelectInWord;
use epserde::Epserde;
use mem_dbg::{MemDbg, MemSize};

use crate::prelude::*;

#[derive(Epserde, Debug, Clone, MemDbg, MemSize)]
pub struct Rank10Sel<
    const LOG2_LOWER_BLOCK_SIZE: usize,
    const LOG2_ONES_PER_INVENTORY: usize = 12,
    const HINT_BIT_SIZE: usize = 64,
    B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]> = BitVec,
> {
    rank10: Rank10<LOG2_LOWER_BLOCK_SIZE, HINT_BIT_SIZE, B>,
    inventory: Vec<u64>,
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
        B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]>,
    > Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, B>
{
    const LOWER_BLOCK_SIZE: usize = 1 << LOG2_LOWER_BLOCK_SIZE;
    const UPPER_BLOCK_SIZE: usize =
        Rank10::<LOG2_LOWER_BLOCK_SIZE, HINT_BIT_SIZE, B>::UPPER_BLOCK_SIZE;
    const BASIC_BLOCK_SIZE: usize =
        Rank10::<LOG2_LOWER_BLOCK_SIZE, HINT_BIT_SIZE, B>::BASIC_BLOCK_SIZE;
    const ONES_PER_INVENTORY: usize = 1 << LOG2_ONES_PER_INVENTORY;
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
    > Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, BitVec>
{
    pub fn new(bits: BitVec) -> Self {
        let rank10 = Rank10::<LOG2_LOWER_BLOCK_SIZE, HINT_BIT_SIZE>::new(bits);

        let num_bits = rank10.bits.len();
        let num_ones = rank10.bits.count_ones() as usize;

        let inventory_size = (num_ones + Self::ONES_PER_INVENTORY - 1) / Self::ONES_PER_INVENTORY;
        let mut inventory = Vec::<u64>::with_capacity(inventory_size + 1);

        let mut curr_num_ones: usize = 0;
        let mut next_quantum: usize = 0;

        for (i, word) in rank10.bits.as_ref().iter().copied().enumerate() {
            let ones_in_word = word.count_ones() as usize;

            while curr_num_ones + ones_in_word > next_quantum {
                let in_word_index = word.select_in_word((next_quantum - curr_num_ones) as usize);
                let index = (i * u64::BITS as usize) + in_word_index;

                inventory.push(index as u64);

                next_quantum += Self::ONES_PER_INVENTORY;
            }
            curr_num_ones += ones_in_word;
        }
        assert_eq!(num_ones, curr_num_ones);
        inventory.push(num_bits as u64);
        assert_eq!(inventory.len(), inventory_size + 1);

        Self { rank10, inventory }
    }
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
        B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]>,
    > Select for Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, B>
{
    unsafe fn select_unchecked(&self, rank: usize) -> usize {
        let inventory_index = rank / Self::ONES_PER_INVENTORY;
        let jump = (rank % Self::ONES_PER_INVENTORY) / Self::LOWER_BLOCK_SIZE;

        let inv_ref = <Vec<u64> as AsRef<[u64]>>::as_ref(&self.inventory);
        let inv_pos = *inv_ref.get_unchecked(inventory_index) as usize;
        let next_inv_pos = *inv_ref.get_unchecked(inventory_index + 1) as usize;
        let last_lower_block = next_inv_pos / Self::LOWER_BLOCK_SIZE;

        let mut lower_block_idx = inv_pos / Self::LOWER_BLOCK_SIZE + jump;
        let upper_block_idx = lower_block_idx * Self::LOWER_BLOCK_SIZE / Self::UPPER_BLOCK_SIZE;

        let mut hint_rank =
            self.rank10.counts.upper(upper_block_idx) + self.rank10.counts.lower(lower_block_idx);

        let mut next_rank;
        let mut next_upper_block_idx;
        let mut next_lower_block_idx;
        loop {
            if lower_block_idx + 1 >= last_lower_block {
                break;
            }
            next_lower_block_idx = lower_block_idx + 1;
            next_upper_block_idx =
                next_lower_block_idx * Self::LOWER_BLOCK_SIZE / Self::UPPER_BLOCK_SIZE;
            next_rank = self.rank10.counts.upper(next_upper_block_idx)
                + self.rank10.counts.lower(next_lower_block_idx);
            if next_rank > rank as u64 {
                break;
            }
            hint_rank = next_rank;
            lower_block_idx = next_lower_block_idx;
        }

        let hint_pos;
        // second basic block
        let b1 = self.rank10.counts.basic(lower_block_idx, 1);
        if hint_rank + b1 > rank as u64 {
            hint_pos = lower_block_idx * Self::LOWER_BLOCK_SIZE;
            return self
                .rank10
                .bits
                .select_hinted_unchecked(rank, hint_pos, hint_rank as usize);
        }
        // third basic block
        let b2 = self.rank10.counts.basic(lower_block_idx, 2);
        if hint_rank + b2 > rank as u64 {
            hint_pos = lower_block_idx * Self::LOWER_BLOCK_SIZE + 1 * Self::BASIC_BLOCK_SIZE;
            return self.rank10.bits.select_hinted_unchecked(
                rank,
                hint_pos,
                (hint_rank + b1) as usize,
            );
        }
        // fourth basic block
        let b3 = self.rank10.counts.basic(lower_block_idx, 3);
        if hint_rank + b3 > rank as u64 {
            hint_pos = lower_block_idx * Self::LOWER_BLOCK_SIZE + 2 * Self::BASIC_BLOCK_SIZE;
            return self.rank10.bits.select_hinted_unchecked(
                rank,
                hint_pos,
                (hint_rank + b2) as usize,
            );
        }

        hint_pos = lower_block_idx * Self::LOWER_BLOCK_SIZE + 3 * Self::BASIC_BLOCK_SIZE;
        self.rank10
            .bits
            .select_hinted_unchecked(rank, hint_pos, (hint_rank + b3) as usize)
    }
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
        B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]>,
    > Rank for Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, B>
{
    unsafe fn rank_unchecked(&self, pos: usize) -> usize {
        self.rank10.rank_unchecked(pos)
    }
    fn rank(&self, pos: usize) -> usize {
        self.rank10.rank(pos)
    }
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
        B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]>,
    > BitCount for Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, B>
{
    fn count(&self) -> usize {
        self.rank10.count()
    }
}

impl<
        const LOG2_LOWER_BLOCK_SIZE: usize,
        const LOG2_ONES_PER_INVENTORY: usize,
        const HINT_BIT_SIZE: usize,
        B: RankHinted<HINT_BIT_SIZE> + SelectHinted + Rank + Select + BitCount + AsRef<[usize]>,
    > BitLength for Rank10Sel<LOG2_LOWER_BLOCK_SIZE, LOG2_ONES_PER_INVENTORY, HINT_BIT_SIZE, B>
{
    fn len(&self) -> usize {
        self.rank10.len()
    }
}

#[cfg(test)]
mod test_rank10sel {
    use super::*;
    use crate::prelude::BitVec;
    use rand::{rngs::SmallRng, Rng, SeedableRng};

    const TEST_LOG2_LOWER_BLOCK_SIZE: usize = 8;
    const TEST_LOG2_ONES_PER_INVENTORY: usize = 9;

    #[test]
    fn test_rank10sel() {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
        let density = 0.5;
        let lens = (1..1000).chain((1000..10000).step_by(100));
        for len in lens {
            let bits = (0..len).map(|_| rng.gen_bool(density)).collect::<BitVec>();
            let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
                Rank10Sel::new(bits.clone());

            let ones = bits.count_ones();
            let mut pos = Vec::with_capacity(ones);
            for i in 0..len {
                if bits[i] {
                    pos.push(i);
                }
            }

            for i in 0..ones {
                assert_eq!(rank10sel.select(i), Some(pos[i]));
            }
            assert_eq!(rank10sel.select(ones + 1), None);
        }
    }

    #[test]
    fn test_rank10sel_mult_usize() {
        let mut rng = rand::rngs::SmallRng::seed_from_u64(0);
        let density = 0.5;
        for len in (1 << 10..1 << 15).step_by(usize::BITS as _) {
            let bits = (0..len).map(|_| rng.gen_bool(density)).collect::<BitVec>();
            let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
                Rank10Sel::new(bits.clone());

            let ones = bits.count_ones();
            let mut pos = Vec::with_capacity(ones);
            for i in 0..len {
                if bits[i] {
                    pos.push(i);
                }
            }

            for i in 0..ones {
                assert_eq!(rank10sel.select(i), Some(pos[i]));
            }
            assert_eq!(rank10sel.select(ones + 1), None);
        }
    }

    #[test]
    fn test_rank10sel_empty() {
        let bits = BitVec::new(0);
        let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
            Rank10Sel::new(bits.clone());
        assert_eq!(rank10sel.count(), 0);
        assert_eq!(rank10sel.len(), 0);
        assert_eq!(rank10sel.select(0), None);
    }

    #[test]
    fn test_rank10sel_ones() {
        let len = 300_000;
        let bits = (0..len).map(|_| true).collect::<BitVec>();
        let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
            Rank10Sel::new(bits);
        assert_eq!(rank10sel.count(), len);
        assert_eq!(rank10sel.len(), len);
        for i in 0..len {
            assert_eq!(rank10sel.select(i), Some(i));
        }
    }

    #[test]
    fn test_rank10sel_zeros() {
        let len = 300_000;
        let bits = (0..len).map(|_| false).collect::<BitVec>();
        let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
            Rank10Sel::new(bits);
        assert_eq!(rank10sel.count(), 0);
        assert_eq!(rank10sel.len(), len);
        assert_eq!(rank10sel.select(0), None);
    }

    #[test]
    fn test_rank10sel_few_ones() {
        let lens = [1 << 18, 1 << 19, 1 << 20];
        for len in lens {
            for num_ones in [1, 2, 4, 8, 16, 32, 64, 128, 256] {
                let bits = (0..len)
                    .map(|i| i % (len / num_ones) == 0)
                    .collect::<BitVec>();
                let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
                    Rank10Sel::new(bits);
                assert_eq!(rank10sel.count(), num_ones);
                assert_eq!(rank10sel.len(), len);
                for i in 0..num_ones {
                    assert_eq!(rank10sel.select(i), Some(i * (len / num_ones)));
                }
            }
        }
    }

    #[test]
    fn test_rank10sel_non_uniform() {
        let lens = [1 << 18, 1 << 19, 1 << 20, 1 << 25];

        let mut rng = SmallRng::seed_from_u64(0);
        for len in lens {
            for density in [0.5] {
                let density0 = density * 0.01;
                let density1 = density * 0.99;

                let len1;
                let len2;
                if len % 2 != 0 {
                    len1 = len / 2 + 1;
                    len2 = len / 2;
                } else {
                    len1 = len / 2;
                    len2 = len / 2;
                }

                let first_half = loop {
                    let b = (0..len1)
                        .map(|_| rng.gen_bool(density0))
                        .collect::<BitVec>();
                    if b.count_ones() > 0 {
                        break b;
                    }
                };
                let num_ones_first_half = first_half.count_ones();
                let second_half = (0..len2)
                    .map(|_| rng.gen_bool(density1))
                    .collect::<BitVec>();
                let num_ones_second_half = second_half.count_ones();

                assert!(num_ones_first_half > 0);
                assert!(num_ones_second_half > 0);

                let bits = first_half
                    .into_iter()
                    .chain(second_half.into_iter())
                    .collect::<BitVec>();

                assert_eq!(
                    num_ones_first_half + num_ones_second_half,
                    bits.count_ones()
                );

                assert_eq!(bits.len(), len as usize);

                let ones = bits.count_ones();
                let mut pos = Vec::with_capacity(ones);
                for i in 0..(len as usize) {
                    if bits[i] {
                        pos.push(i);
                    }
                }

                let rank10sel: Rank10Sel<TEST_LOG2_LOWER_BLOCK_SIZE, TEST_LOG2_ONES_PER_INVENTORY> =
                    Rank10Sel::new(bits);

                for i in 0..ones {
                    assert!(rank10sel.select(i) == Some(pos[i]));
                }
                assert_eq!(rank10sel.select(ones + 1), None);
            }
        }
    }
}
