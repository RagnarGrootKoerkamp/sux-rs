# `sux-rs`

A pure Rust implementation of succinct data structures from the [Sux](https://sux.di.unimi.it/) project.

This create is a work in progress: new succinct data structures will be added over time. Presently,
we provide:

- the [`BitFieldSlice`](crate::traits::bit_field_slice::BitFieldSlice) trait---an
  alternative to [`Index`](core::ops::Index) returning values of fixed bit width;
- traits for building blocks and structures like [`Rank`](crate::traits::rank_sel::Rank) , 
  [`Select`](crate::traits::rank_sel::Select), and [`IndexedDict`](crate::traits::indexed_dict::IndexedDict);
- an implementation of the [Elias--Fano representation of monotone sequences](crate::dict::elias_fano::EliasFano);
- an implementation of list of [strings compressed by rear-coded prefix omission](crate::dict::rear_coded_list::RearCodedList);
- some support for reading static ([minimal perfect hash](crate::mph::gov::GOVMPH)) [functions](crate::sf::gov3::GOV3)
  generated by [Sux4J](<http://sux4j.di.unimi.it/>).
