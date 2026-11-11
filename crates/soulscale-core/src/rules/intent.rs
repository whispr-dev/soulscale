//! Intent formulas (rulebook §11 / quick reference).
//!
//! Python uses floor division (`//`) and `math.ceil` on non-negative values; the helpers
//! below reproduce those exactly, including negative numerators for Press.

/// Press bonus = max(0, floor((friendly + harmony - floor(enemy / 2)) / 3)).
pub fn press_bonus(own: i32, harmony: i32, enemy: i32) -> i32 {
    (own + harmony - enemy.div_euclid(2)).div_euclid(3).max(0)
}

/// Anchor stack = card stack + intensity + 1 if same-aspect harmony exists.
pub fn anchor_stack(stack_value: i32, intensity: i32, harmony: i32) -> i32 {
    stack_value + intensity + i32::from(harmony > 0)
}

/// Release bonus = min(5, ceil(spent × intensity / 4)).
pub fn release_bonus(spent: i32, intensity: i32) -> i32 {
    let x = (spent * intensity).max(0);
    ((x + 3) / 4).min(5)
}

/// Sever cut = card stack + intensity + 1 if the card is disruptive.
pub fn sever_cut(stack_value: i32, intensity: i32, disruptive: bool) -> i32 {
    stack_value + intensity + i32::from(disruptive)
}

/// Friendly stack added by Sever = max(1, card stack - 1).
pub fn sever_stack_add(stack_value: i32) -> i32 {
    (stack_value - 1).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_matches_python_floor_semantics() {
        assert_eq!(press_bonus(0, 0, 0), 0);
        assert_eq!(press_bonus(3, 0, 0), 1);
        assert_eq!(press_bonus(5, 1, 0), 2);
        assert_eq!(press_bonus(4, 0, 3), 1); // (4 + 0 - 1) // 3
        assert_eq!(press_bonus(0, 0, 9), 0); // negative numerator floors, then clamps
    }

    #[test]
    fn release_caps_at_five_and_rounds_up() {
        assert_eq!(release_bonus(1, 1), 1);
        assert_eq!(release_bonus(3, 2), 2);
        assert_eq!(release_bonus(4, 3), 3);
        assert_eq!(release_bonus(20, 3), 5);
        assert_eq!(release_bonus(0, 3), 0);
    }

    #[test]
    fn anchor_and_sever() {
        assert_eq!(anchor_stack(2, 3, 1), 6);
        assert_eq!(anchor_stack(2, 3, 0), 5);
        assert_eq!(sever_cut(1, 2, true), 4);
        assert_eq!(sever_stack_add(1), 1);
        assert_eq!(sever_stack_add(3), 2);
    }
}
