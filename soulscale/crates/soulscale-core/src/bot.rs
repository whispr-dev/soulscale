//! Bots that drive *human* seats with the AI's own policy. Used by the simulator, the
//! Python-parity tests and the server's "auto-play this round" fallback.
//!
//! The policy mirrors what `tools/py_harness/run_parity.py` does to the Python engine:
//! choose with `ai_choose`, then reroll iff Will > 0 and `ai_should_reroll` says so,
//! using the other seat's committed Chamber when one exists.

use crate::model::AiProfile;
use crate::session::{Controller, SeatStatus, Session, SessionError};
use crate::state::Seat;

/// Take one action for `seat` if it has one pending. Returns whether it acted.
pub fn bot_act(s: &mut Session, seat: Seat, profile: &AiProfile) -> Result<bool, SessionError> {
    if s.controllers[seat.idx()] == Controller::Ai {
        return Ok(false);
    }
    match s.status(seat) {
        SeatStatus::Choose => {
            let choice = s.game.ai_choose(seat, profile, None);
            s.choose(seat, choice)?;
            Ok(true)
        }
        SeatStatus::Reroll { intensity, .. } => {
            let choice = s.turns[seat.idx()].choice.expect("rolled seat has a choice");
            let other = s.turns[seat.other().idx()].choice.map(|c| c.chamber);
            let reroll =
                s.game.st.seat(seat).will > 0 && s.game.ai_should_reroll(profile, &choice, intensity, other);
            s.decide_reroll(seat, reroll)?;
            Ok(true)
        }
        _ => Ok(false),
    }
}

/// Play until the game ends or round `stop_round` has begun (the same stop rule as the
/// Python harness: `while not game_over and round_no < stop_round`). P1 acts before P2.
pub fn play_out(s: &mut Session, profiles: [&AiProfile; 2], stop_round: u32) -> Result<(), SessionError> {
    while !s.game.is_over() && s.game.st.round < stop_round {
        let mut acted = false;
        for seat in Seat::BOTH {
            if bot_act(s, seat, profiles[seat.idx()])? {
                acted = true;
                break;
            }
        }
        if !acted {
            break;
        }
    }
    Ok(())
}
