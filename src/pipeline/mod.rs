/// Static breed catalog from wasm4pm-cognition.
pub mod catalog;
/// OCEL-based fitness evaluation for TPOT2 breed pipeline optimization.
pub mod fitness;
/// Object-centric process-mining grounding: OCEL 2.0 reader, OC-DFG, and a
/// log-grounded fitness evaluator.
pub mod ocel;
/// Pareto / multi-objective variant of the TPOT2 breed-pipeline search.
pub mod pareto;
/// Phase-shift model: conformance state as a water/steam phase transition with
/// autonomic-mesh expansion.
pub mod phase;
/// TPOT2-style genetic programming search engine for breed pipeline optimization.
pub mod search;
/// Core TPOT2-style breed pipeline types.
pub mod types;
