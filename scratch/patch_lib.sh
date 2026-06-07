sed -i '' 's/pub mod receipt;/pub mod receipt;\npub mod protocol;/' /Users/sac/ggen/crates/ggen-projection/src/lib.rs
sed -i '' 's/pub use receipt::{/pub use protocol::{PackObservation, PackFinding, ProjectionSignature, CustomizationPoint, PackActionIntent, GgenObservedDiagnostic};\npub use receipt::{/' /Users/sac/ggen/crates/ggen-projection/src/lib.rs
