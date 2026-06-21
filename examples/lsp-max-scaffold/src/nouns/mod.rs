/// Actuation grammar: noun/verb CLI surface (Layer 1).
///
/// Each module below is a "noun" file. The `#[verb("name")]` attribute on
/// a function registers it as a sub-command under the noun.
///
/// Convention: three tiers per file —
///   1. Domain tier — serialisable value types
///   2. Service tier — business logic, no I/O side-effects
///   3. Verb tier — `#[verb]`-annotated entry points
pub mod admit;
pub mod gate;
pub mod serve;
pub mod session;
pub mod verify;
