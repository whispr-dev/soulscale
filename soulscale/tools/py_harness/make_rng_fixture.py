#!/usr/bin/env python3
"""Generate the CPython RNG reference fixture used by crates/soulscale-core/tests/rng_parity.rs."""
import json
import random
import sys

SEEDS = [0, 1, 2, 7, 42, 12345, 2**31 - 1, 2**32 - 1, 2**32, 2**40 + 123, 2**63 + 5, 2**64 - 1]
out = []
for seed in SEEDS:
    r = random.Random(seed)
    rec = {"seed": str(seed)}
    rec["u32"] = [r.getrandbits(32) for _ in range(5)]
    rec["randint16"] = [r.randint(1, 6) for _ in range(40)]
    rec["randbelow"] = [r.randrange(n) for n in (1, 2, 3, 5, 7, 10, 100, 999, 65537)]
    rec["random"] = [r.random() for _ in range(6)]
    rec["uniform"] = [r.uniform(-2.2, 2.2) for _ in range(6)]
    shuf = []
    for _ in range(3):
        xs = list(range(10))
        r.shuffle(xs)
        shuf.append(xs)
    rec["shuffle10"] = shuf
    rec["choice6"] = [r.choice(range(6)) for _ in range(8)]
    out.append(rec)

sums = []
r = random.Random(99)
for n in (1, 2, 3, 7, 12, 30):
    for _ in range(20):
        xs = [max(0.1, r.uniform(-12, 12) + 8.0) for _ in range(n)]
        sums.append({"xs": xs, "sum": sum(xs)})

json.dump({"python": sys.version.split()[0], "streams": out, "sums": sums},
          open(sys.argv[1], "w"), indent=1)
print("ok", sys.version.split()[0])
