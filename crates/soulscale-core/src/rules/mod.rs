//! Rules, one concern per module (per the design manifesto):
//!
//! | module      | concern                                                        |
//! |-------------|----------------------------------------------------------------|
//! | `intensity` | d6 → intensity                                                 |
//! | `intent`    | Press / Anchor / Release / Sever formulas                      |
//! | `pending`   | building a play: bonuses, conditions, price keywords, intents  |
//! | `clash`     | guards, traps, Fragile, Calm/Tangle, clash halving             |
//! | `movement`  | v1.0.1 movement resolver (declared movement types)             |
//! | `effects`   | applying a play: stacks, card specials, Delayed/Echo, Binding  |
//! | `ongoing`   | ongoing board effects each round                               |
//! | `strain`    | Strain and Purgatory Events                                    |

pub mod clash;
pub mod effects;
pub mod intensity;
pub mod intent;
pub mod movement;
pub mod ongoing;
pub mod pending;
pub mod strain;
