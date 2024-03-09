use clap::{arg, Parser, ValueEnum};
use mem_dbg::*;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use sux::{
    bits::BitVec,
    rank_sel::{Rank9Sel, SimpleSelect},
    traits::*,
};

trait SelStruct<B>: Select {
    fn new(bits: B) -> Self;
}
impl SelStruct<BitVec> for SimpleSelect {
    fn new(bits: BitVec) -> Self {
        SimpleSelect::new(bits, 3)
    }
}
impl SelStruct<BitVec> for Rank9Sel {
    fn new(bits: BitVec) -> Self {
        Rank9Sel::new(bits)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum SelType {
    Simpleselect,
    Rank9sel,
}

#[derive(Parser)]
struct Cli {
    len: usize,
    density: f64,
    #[arg(short, long)]
    uniform: bool,
    #[arg(value_enum)]
    sel_type: SelType,
}

fn create_sel_struct<S: SelStruct<BitVec> + MemSize + MemDbg>(
    len: usize,
    density: f64,
    uniform: bool,
) -> S {
    let mut rng = SmallRng::seed_from_u64(0);
    let (density0, density1) = if uniform {
        (density, density)
    } else {
        (density * 0.01, density * 0.99)
    };

    let first_half = loop {
        let b = (0..len / 2)
            .map(|_| rng.gen_bool(density0))
            .collect::<BitVec>();
        if b.count_ones() > 0 {
            break b;
        }
    };
    let second_half = (0..len / 2)
        .map(|_| rng.gen_bool(density1))
        .collect::<BitVec>();

    let bits = first_half
        .into_iter()
        .chain(second_half.into_iter())
        .collect::<BitVec>();

    S::new(bits)
}

fn mem_cost<S: SelStruct<BitVec> + MemSize + MemDbg + BitLength>(sel_struct: &S) -> f64 {
    (((sel_struct.mem_size(SizeFlags::default()) * 8 - sel_struct.len()) * 100) as f64)
        / (sel_struct.len() as f64)
}

fn main() {
    let cli = Cli::parse();

    match cli.sel_type {
        SelType::Simpleselect => {
            let sel_struct = create_sel_struct::<SimpleSelect>(cli.len, cli.density, cli.uniform);
            let mem_cost = mem_cost(&sel_struct);
            println!(
                "BitVec with length: {}, density: {}, uniform: {}",
                cli.len, cli.density, cli.uniform
            );
            println!("Memory cost of SimpleSelect: {}%", mem_cost);
        }
        SelType::Rank9sel => {
            let sel_struct = create_sel_struct::<Rank9Sel>(cli.len, cli.density, cli.uniform);
            let mem_cost = mem_cost(&sel_struct);
            println!(
                "BitVec with length: {}, density: {}, uniform: {}",
                cli.len, cli.density, cli.uniform
            );
            println!("Memory cost of Rank9Sel: {}%", mem_cost);
        }
    }
}
