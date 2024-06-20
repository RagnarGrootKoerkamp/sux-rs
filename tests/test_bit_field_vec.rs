/*
 * SPDX-FileCopyrightText: 2023 Inria
 * SPDX-FileCopyrightText: 2023 Sebastiano Vigna
 *
 * SPDX-License-Identifier: Apache-2.0 OR LGPL-2.1-or-later
 */

use common_traits::CastableFrom;
use common_traits::CastableInto;
use core::sync::atomic::Ordering;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::Rng;
use rand::SeedableRng;
use sux::prelude::*;

#[test]
fn test() {
    test_param::<u8>();
    test_param::<u16>();
    test_param::<u32>();
    test_param::<u64>();
    test_param::<u128>();
    test_param::<usize>();
}

fn test_param<W: Word + CastableInto<u64> + CastableFrom<u64>>() {
    for bit_width in 0..W::BITS {
        let n = 100;
        let u = W::ONE << bit_width.saturating_sub(1).min(60);
        let mut rng = SmallRng::seed_from_u64(0);

        let mut v = BitFieldVec::<W>::new(bit_width, n);
        assert_eq!(v.bit_width(), bit_width);
        assert_eq!(
            v.mask(),
            if bit_width == 0 {
                W::ZERO
            } else {
                (W::ONE << bit_width) - W::ONE
            }
        );
        for _ in 0..10 {
            let values = (0..n)
                .map(|_| rng.gen_range(0..u.cast()).cast())
                .collect::<Vec<W>>();

            let mut indices = (0..n).collect::<Vec<_>>();
            indices.shuffle(&mut rng);

            for i in indices {
                v.set(i, values[i]);
            }

            for (i, value) in values.iter().enumerate() {
                assert_eq!(v.get(i), *value);
            }

            let mut indices = (0..n).collect::<Vec<_>>();
            indices.shuffle(&mut rng);

            for i in indices {
                assert_eq!(v.get(i), values[i]);
            }

            for from in 0..v.len() {
                let mut iter = v.into_unchecked_iter_from(from);
                for v in &values[from..] {
                    unsafe {
                        assert_eq!(iter.next_unchecked(), *v);
                    }
                }
            }

            for from in 0..v.len() {
                let mut iter = v.into_rev_unchecked_iter_from(from);
                for v in values[..from].iter().rev() {
                    unsafe {
                        assert_eq!(iter.next_unchecked(), *v);
                    }
                }
            }

            for from in 0..v.len() {
                for (i, v) in v.iter_from(from).enumerate() {
                    assert_eq!(v, values[i + from]);
                }
            }

            let (b, w, l) = v.clone().into_raw_parts();
            assert_eq!(unsafe { BitFieldVec::<W>::from_raw_parts(b, w, l) }, v);
        }
    }
}

#[test]
fn test_atomic_bit_field_vec() {
    use sux::traits::bit_field_slice::AtomicBitFieldSlice;

    for bit_width in 0..usize::BITS as usize {
        let n: usize = 100;
        let u: usize = 1 << bit_width;
        let mut rng = SmallRng::seed_from_u64(0);

        let v = AtomicBitFieldVec::new(bit_width, n);
        assert_eq!(v.bit_width(), bit_width);
        for _ in 0..10 {
            let values = (0..n).map(|_| rng.gen_range(0..u)).collect::<Vec<_>>();

            let mut indices = (0..n).collect::<Vec<_>>();
            indices.shuffle(&mut rng);

            for i in indices {
                v.set_atomic(i, values[i], Ordering::Relaxed);
            }

            for (i, value) in values.iter().enumerate() {
                assert_eq!(v.get_atomic(i, Ordering::Relaxed), *value);
            }

            let mut indices = (0..n).collect::<Vec<_>>();
            indices.shuffle(&mut rng);

            for i in indices {
                assert_eq!(v.get_atomic(i, Ordering::Relaxed), values[i]);
            }
        }

        let w: BitFieldVec = v.into();
        let x = w.clone();
        let y: AtomicBitFieldVec = x.into();
        let z: AtomicBitFieldVec = w.into();

        let (b, w, l) = z.into_raw_parts();
        let z = unsafe { AtomicBitFieldVec::<usize>::from_raw_parts(b, w, l) };
        for i in 0..n {
            assert_eq!(
                z.get_atomic(i, Ordering::Relaxed),
                y.get_atomic(i, Ordering::Relaxed)
            );
        }
    }
}

#[test]
fn test_usize() {
    use sux::traits::bit_field_slice::BitFieldSlice;
    use sux::traits::bit_field_slice::BitFieldSliceMut;

    const BITS: usize = core::mem::size_of::<usize>() * 8;
    let mut c = BitFieldVec::<usize>::new(BITS, 4);
    c.set(0, -1_isize as usize);
    c.set(1, 1234567);
    c.set(2, 0);
    c.set(3, -1_isize as usize);
    assert_eq!(c.get(0), -1_isize as usize);
    assert_eq!(c.get(1), 1234567);
    assert_eq!(c.get(2), 0);
    assert_eq!(c.get(3), -1_isize as usize);
}

#[test]
fn test_width_zero() {
    use sux::traits::bit_field_slice::BitFieldSlice;

    let c = BitFieldVec::<usize>::new(0, 1000);
    for i in 0..c.len() {
        assert_eq!(c.get(i), 0);
    }
}

#[test]
fn test_from_slice() {
    use sux::traits::bit_field_slice::BitFieldSlice;
    use sux::traits::bit_field_slice::BitFieldSliceMut;

    let mut c = BitFieldVec::new(12, 1000);
    for i in 0..c.len() {
        c.set(i, i)
    }

    let s = BitFieldVec::<usize>::from_slice(&c).unwrap();
    for i in 0..c.len() {
        assert_eq!({ s.get(i) }, c.get(i));
    }
    let s = BitFieldVec::<u16>::from_slice(&c).unwrap();
    for i in 0..c.len() {
        assert_eq!(s.get(i) as usize, c.get(i));
    }
    assert!(BitFieldVec::<u8>::from_slice(&c).is_err())
}

#[test]
fn test_push() {
    use sux::traits::bit_field_slice::BitFieldSlice;

    let mut c = BitFieldVec::new(12, 0);
    for i in 0..1000 {
        c.push(i);
    }
    for i in 0..1000 {
        assert_eq!(c.get(i), i);
    }
}

#[test]
fn test_resize() {
    use sux::traits::bit_field_slice::BitFieldSlice;

    let mut c = BitFieldVec::new(12, 0);
    c.resize(100, 2_usize);
    for i in 0..100 {
        assert_eq!(c.get(i), 2);
    }
    c.resize(50, 0);
    for i in 0..50 {
        assert_eq!(c.get(i), 2);
    }
    assert_eq!(c.len(), 50);
}

#[test]
fn test_pop() {
    use sux::traits::bit_field_slice::BitFieldSlice;

    let mut c = BitFieldVec::new(12, 0);
    for i in 0..1000 {
        c.push(i);
    }
    for i in (500..1000).rev() {
        assert_eq!(c.pop(), Some(i));
    }
    for i in 0..500 {
        assert_eq!(c.get(i), i);
    }
    for i in (0..500).rev() {
        assert_eq!(c.pop(), Some(i));
    }
    assert_eq!(c.pop(), None);
    assert_eq!(c.pop(), None);
}

#[test]
fn test_unaligned() {
    for bit_width in [50, 56, 57, 58, 60, 64] {
        let mut c = BitFieldVec::new(bit_width, 0);
        for i in 0_usize..10 {
            c.push(i);
        }
        for i in 0_usize..10 {
            assert_eq!(c.get_unaligned(i), i);
        }
    }
}

#[should_panic]
#[test]
fn test_unaligned_59() {
    let c = BitFieldVec::<usize, _>::new(59, 1);
    assert_eq!(c.get_unaligned(0), 0);
}

#[should_panic]
#[test]
fn test_unaligned_61() {
    let c = BitFieldVec::<usize, _>::new(59, 1);
    assert_eq!(c.get_unaligned(0), 0);
}

#[should_panic]
#[test]
fn test_unaligned_62() {
    let c = BitFieldVec::<usize, _>::new(59, 1);
    assert_eq!(c.get_unaligned(0), 0);
}

#[should_panic]
#[test]
fn test_unaligned_63() {
    let c = BitFieldVec::<usize, _>::new(59, 1);
    assert_eq!(c.get_unaligned(0), 0);
}

#[test]
fn test_get_addr() {
    let c = BitFieldVec::<usize, _>::new(3, 100);
    let begin_addr = c.addr_of(0) as usize;
    assert_eq!(c.addr_of(50) as usize - begin_addr, 16);

    let c = BitFieldVec::<u16, _>::new(3, 100);
    let begin_addr = c.addr_of(0) as usize;
    assert_eq!(c.addr_of(50) as usize - begin_addr, 18);
}

#[test]
fn test_eq() {
    let mut b = BitFieldVec::<usize>::new(3, 10);
    let mut c = BitFieldVec::<usize>::new(3, 10);
    assert_eq!(b, c);

    b.push(3);
    assert_ne!(b, c);
    c.push(3);
    assert_eq!(b, c);

    for i in 0..64 {
        b.push(i % 4);
        c.push(i % 4);
        assert_eq!(b, c);
    }
}
