#!/usr/bin/env python3

import subprocess
from tqdm.auto import tqdm

benches = [
    "rank9",
    "rank11",
    "simple_select",
    "rank9sel",
    "simple_select_non_uniform",
    "rank9sel_non_uniform",
    "rng",
    "rng_non_uniform",
]

if __name__ == "__main__":
    with tqdm(total=len(benches)) as pbar:
        for bench in benches:
            pbar.write("Running bench: {}...".format(bench))
            subprocess.run(
                "cargo bench --bench my_benchmarks -- {} --noplot --quiet --exact --nocapture".format(
                    bench),
                shell=True,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.STDOUT)
            pbar.update(1)
