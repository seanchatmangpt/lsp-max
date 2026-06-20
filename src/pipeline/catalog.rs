/// All breed names available in wasm4pm-cognition.
/// Source: wasm4pm-cognition/src/breeds/*.rs (excluding mod.rs, dispatch.rs, registration.rs,
/// registration.rs.backup, bayesian_network_test_script.rs, and the support/ subdirectory)
pub static KNOWN_BREEDS: &[&str] = &[
    "abductive_ibe",
    "abductive_lp",
    "act_r",
    "allen_temporal",
    "analogy_sme",
    "asp",
    "autoinstinct_learning",
    "autoinstinct_neurosis",
    "autoinstinct_semantics",
    "autoinstinct_vision",
    "bayesian_network",
    "belief_merging",
    "cbr",
    "circumscription",
    "clp",
    "construction_grammar",
    "contingent_plan",
    "csp_ac3",
    "ctl_check",
    "default_logic",
    "dempster_shafer",
    "dendral",
    "description_logic",
    "ebl",
    "episodic_memory",
    "event_calculus",
    "frame",
    "frames_inheritance",
    "fuzzy_logic",
    "gps",
    "hearsay",
    "htn_planning",
    "ilp",
    "ltl_monitor",
    "markov_logic",
    "mdp",
    "meta_reasoning",
    "morphological",
    "naive_physics",
    "ocpm_route_discoverer",
    "oracle_chain",
    "partial_order_plan",
    "pomdp",
    "problog",
    "production_rules",
    "prolog",
    "qualitative_reason",
    "rl_symbolic",
    "sat_cdcl",
    "script_sam",
    "situation_calculus",
    "soar",
    "standing",
    "strips",
    "tableaux",
    "triz",
    "version_space",
];

/// Breed category for structured search space partitioning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BreedCategory {
    LogicBased,
    RuleBased,
    PlanningBased,
    Probabilistic,
    Temporal,
    MemoryBased,
    MetaBased,
}

pub fn breed_category(breed: &str) -> BreedCategory {
    match breed {
        "asp" | "prolog" | "description_logic" | "circumscription" | "default_logic" |
        "sat_cdcl" | "tableaux" | "abductive_ibe" | "abductive_lp" | "clp" => BreedCategory::LogicBased,

        "production_rules" | "dendral" | "cbr" | "analogy_sme" | "version_space" |
        "ebl" | "ilp" | "markov_logic" | "problog" => BreedCategory::RuleBased,

        "strips" | "htn_planning" | "gps" | "partial_order_plan" | "contingent_plan" |
        "situation_calculus" | "event_calculus" | "mdp" | "pomdp" | "rl_symbolic" => BreedCategory::PlanningBased,

        "bayesian_network" | "dempster_shafer" | "fuzzy_logic" | "qualitative_reason" => BreedCategory::Probabilistic,

        "ltl_monitor" | "ctl_check" | "allen_temporal" | "naive_physics" => BreedCategory::Temporal,

        "frame" | "frames_inheritance" | "hearsay" | "soar" | "act_r" |
        "episodic_memory" | "script_sam" | "construction_grammar" | "morphological" => BreedCategory::MemoryBased,

        "meta_reasoning" | "belief_merging" | "triz" | "csp_ac3" => BreedCategory::MetaBased,

        _ => BreedCategory::MetaBased,
    }
}
