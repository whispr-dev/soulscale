//! Intensity roll: 1–2 → 1, 3–4 → 2, 5–6 → 3.

pub fn intensity_from_d6(raw: u8) -> i32 {
    match raw {
        0..=2 => 1,
        3..=4 => 2,
        _ => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_rulebook() {
        let got: Vec<i32> = (1..=6).map(intensity_from_d6).collect();
        assert_eq!(got, vec![1, 1, 2, 2, 3, 3]);
    }
}
