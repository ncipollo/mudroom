// Scales the injected `effect`'s damage with the caster's strength (see
// src/game/script/execution.rs for how `character`/`effect` are injected). The `| 0` forces
// a ToInt32 coercion — Boa's arithmetic otherwise hands back a float, which the Rust side's
// `i64` effect value field can't deserialize.
effect.effect_type.value = -(character.attributes.strength.current_value * 2) | 0;
