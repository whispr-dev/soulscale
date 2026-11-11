//! Bit-exact RNG parity with CPython (fixture from tools/py_harness/make_rng_fixture.py).

use serde_json::Value;
use soulscale_core::rng::{PyRandom, py_sum_floats};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/rng_cpython.json")).unwrap()
}

fn ints(v: &Value) -> Vec<i64> {
    v.as_array().unwrap().iter().map(|x| x.as_i64().unwrap()).collect()
}

fn floats(v: &Value) -> Vec<f64> {
    v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()
}

#[test]
fn every_operation_matches_cpython_for_every_seed() {
    let fx = fixture();
    for rec in fx["streams"].as_array().unwrap() {
        let seed: u64 = rec["seed"].as_str().unwrap().parse().unwrap();
        let mut r = PyRandom::new(seed);
        let u32s: Vec<i64> = (0..5).map(|_| r.next_u32() as i64).collect();
        assert_eq!(u32s, ints(&rec["u32"]), "u32 stream, seed {seed}");
        let d: Vec<i64> = (0..40).map(|_| r.randint(1, 6) as i64).collect();
        assert_eq!(d, ints(&rec["randint16"]), "randint, seed {seed}");
        let rb: Vec<i64> = [1u32, 2, 3, 5, 7, 10, 100, 999, 65537].iter().map(|&n| r.randbelow(n) as i64).collect();
        assert_eq!(rb, ints(&rec["randbelow"]), "randbelow, seed {seed}");
        let rnd: Vec<f64> = (0..6).map(|_| r.random()).collect();
        assert_eq!(rnd, floats(&rec["random"]), "random, seed {seed}");
        let uni: Vec<f64> = (0..6).map(|_| r.uniform(-2.2, 2.2)).collect();
        assert_eq!(uni, floats(&rec["uniform"]), "uniform, seed {seed}");
        for (i, want) in rec["shuffle10"].as_array().unwrap().iter().enumerate() {
            let mut xs: Vec<i64> = (0..10).collect();
            r.shuffle(&mut xs);
            assert_eq!(xs, ints(want), "shuffle #{i}, seed {seed}");
        }
        let ch: Vec<i64> = (0..8).map(|_| r.choice_index(6) as i64).collect();
        assert_eq!(ch, ints(&rec["choice6"]), "choice, seed {seed}");
    }
}

#[test]
fn float_sum_matches_cpython_neumaier() {
    let fx = fixture();
    for s in fx["sums"].as_array().unwrap() {
        let xs = floats(&s["xs"]);
        assert_eq!(py_sum_floats(&xs), s["sum"].as_f64().unwrap(), "sum of {xs:?}");
    }
}
