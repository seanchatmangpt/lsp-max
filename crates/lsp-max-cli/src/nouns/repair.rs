use clap_noun_verb::error::NounVerbError;
use clap_noun_verb::Result;
use clap_noun_verb_macros::verb;
use lsp_max_runtime::AutonomicMesh;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==============================================================================
// 1. Domain Tier
// ==============================================================================
/// A repair plan stored in the mesh's extra state, keyed per instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairPlan {
    pub plan_id: String,
    pub description: String,
    pub actions: Vec<String>,
    pub status: String,
}

/// Summary of a repair plan across all instances.
#[derive(Debug, Clone, Serialize)]
pub struct RepairPlanSummary {
    pub plan_id: String,
    pub instance_id: String,
    pub description: String,
    pub status: String,
    pub action_count: usize,
}

// ==============================================================================
// 2. Service Tier
// ==============================================================================
/// Extra-state key under which per-instance repair plan maps are stored.
const REPAIR_PLANS_KEY: &str = "repair_plans";

pub struct RepairService {
    state_path: String,
}

impl RepairService {
    pub fn new() -> Self {
        Self {
            state_path: crate::nouns::get_state_path(),
        }
    }

    /// Load all repair plans from mesh.extra, grouped by instance_id.
    /// Returns a flat list of (instance_id, RepairPlan) pairs.
    fn load_all_plans(mesh: &AutonomicMesh) -> Vec<(String, RepairPlan)> {
        let Some(raw) = mesh.extra.get(REPAIR_PLANS_KEY) else {
            return Vec::new();
        };
        // extra["repair_plans"] = { "<instance_id>/<plan_id>": RepairPlan, ... }
        let Ok(map) = serde_json::from_value::<HashMap<String, RepairPlan>>(raw.clone()) else {
            return Vec::new();
        };
        map.into_iter()
            .filter_map(|(composite_key, plan)| {
                let slash = composite_key.find('/')?;
                let instance_id = composite_key[..slash].to_string();
                Some((instance_id, plan))
            })
            .collect()
    }

    /// Save a modified plan map back into mesh.extra.
    fn save_plans(mesh: &mut AutonomicMesh, map: &HashMap<String, RepairPlan>) {
        let serialized = serde_json::to_value(map)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        mesh.extra.insert(REPAIR_PLANS_KEY.to_string(), serialized);
    }

    /// Composite key for a plan: "<instance_id>/<plan_id>".
    fn composite_key(instance_id: &str, plan_id: &str) -> String {
        format!("{}/{}", instance_id, plan_id)
    }

    /// Load the raw plan map from mesh.extra.
    fn load_plan_map(mesh: &AutonomicMesh) -> HashMap<String, RepairPlan> {
        let Some(raw) = mesh.extra.get(REPAIR_PLANS_KEY) else {
            return HashMap::new();
        };
        serde_json::from_value(raw.clone()).unwrap_or_default()
    }

    /// List all repair plans, optionally filtered to a single instance.
    pub fn list(
        &self,
        instance_id: Option<&str>,
    ) -> std::result::Result<RepairListResult, String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path)
            .unwrap_or_else(|_| AutonomicMesh::new());

        let all = Self::load_all_plans(&mesh);

        let plans: Vec<RepairPlanSummary> = all
            .into_iter()
            .filter(|(iid, _)| instance_id.map_or(true, |filter| iid == filter))
            .map(|(iid, plan)| RepairPlanSummary {
                action_count: plan.actions.len(),
                plan_id: plan.plan_id,
                instance_id: iid,
                description: plan.description,
                status: plan.status,
            })
            .collect();

        let total = plans.len();
        Ok(RepairListResult { plans, total })
    }

    /// Find a specific plan by plan_id across all instances.
    pub fn find_plan(&self, plan_id: &str) -> std::result::Result<(String, RepairPlan), String> {
        let mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let all = Self::load_all_plans(&mesh);
        for (iid, plan) in all {
            if plan.plan_id == plan_id {
                return Ok((iid, plan));
            }
        }
        Err(format!("PLAN_NOT_FOUND: {}", plan_id))
    }

    /// Apply a repair plan — set its status to "ADMITTED".
    pub fn apply(&self, plan_id: &str) -> std::result::Result<RepairApplyResult, String> {
        let mut mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut map = Self::load_plan_map(&mesh);

        let composite = map
            .keys()
            .find(|k| k.rfind('/').map_or(false, |pos| &k[pos + 1..] == plan_id))
            .cloned()
            .ok_or_else(|| format!("PLAN_NOT_FOUND: {}", plan_id))?;

        let slash_pos = composite.rfind('/').expect("composite key always has slash");
        let instance_id = composite[..slash_pos].to_string();

        let plan = map.get_mut(&composite).expect("key was just found");
        let previous_status = plan.status.clone();
        plan.status = "ADMITTED".to_string();
        let new_status = plan.status.clone();

        Self::save_plans(&mut mesh, &map);
        mesh.save_to_file(&self.state_path).map_err(|e| e.to_string())?;

        Ok(RepairApplyResult {
            plan_id: plan_id.to_string(),
            previous_status,
            new_status,
            instance_id,
        })
    }

    /// Rollback a repair plan from "ADMITTED" back to "OPEN".
    pub fn rollback(&self, plan_id: &str) -> std::result::Result<RepairRollbackResult, String> {
        let mut mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;

        let mut map = Self::load_plan_map(&mesh);

        let composite = map
            .keys()
            .find(|k| k.rfind('/').map_or(false, |pos| &k[pos + 1..] == plan_id))
            .cloned()
            .ok_or_else(|| format!("PLAN_NOT_FOUND: {}", plan_id))?;

        let plan = map.get_mut(&composite).expect("key was just found");
        if plan.status != "ADMITTED" {
            return Err(format!(
                "PLAN_NOT_ADMITTED: cannot rollback plan '{}' in status '{}'",
                plan_id, plan.status
            ));
        }

        let slash_pos = composite.rfind('/').expect("composite key always has slash");
        let instance_id = composite[..slash_pos].to_string();
        let previous_status = plan.status.clone();
        plan.status = "OPEN".to_string();
        let new_status = plan.status.clone();

        Self::save_plans(&mut mesh, &map);
        mesh.save_to_file(&self.state_path).map_err(|e| e.to_string())?;

        Ok(RepairRollbackResult {
            plan_id: plan_id.to_string(),
            previous_status,
            new_status,
            instance_id,
        })
    }

    /// Preview what apply would do WITHOUT modifying state.
    pub fn dry_run(&self, plan_id: &str) -> std::result::Result<RepairDryRunResult, String> {
        let (_, plan) = self.find_plan(plan_id)?;

        let would_modify = plan.status != "ADMITTED";
        Ok(RepairDryRunResult {
            plan_id: plan_id.to_string(),
            current_status: plan.status,
            projected_status: "ADMITTED".to_string(),
            actions: plan.actions,
            would_modify,
        })
    }

    #[cfg(test)]
    pub fn seed_plan(
        &self,
        instance_id: &str,
        plan: RepairPlan,
    ) -> std::result::Result<(), String> {
        let mut mesh = AutonomicMesh::load_from_file(&self.state_path).map_err(|e| e.to_string())?;
        let mut map = Self::load_plan_map(&mesh);
        let key = Self::composite_key(instance_id, &plan.plan_id);
        map.insert(key, plan);
        Self::save_plans(&mut mesh, &map);
        mesh.save_to_file(&self.state_path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for RepairService {
    fn default() -> Self {
        Self::new()
    }
}

// ==============================================================================
// 3. Verb Tier
// ==============================================================================
#[derive(Debug, Clone, Serialize)]
pub struct RepairListResult {
    pub plans: Vec<RepairPlanSummary>,
    pub total: usize,
}

/// List all repair plans in the mesh, optionally filtered to one instance.
#[verb("list")]
pub fn list(instance_id: Option<String>) -> Result<RepairListResult> {
    let svc = RepairService::new();
    svc.list(instance_id.as_deref())
        .map_err(NounVerbError::execution_error)
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairExplainResult {
    pub plan_id: String,
    pub instance_id: String,
    pub description: String,
    pub actions: Vec<String>,
    pub status: String,
}

/// Show the full details of a specific repair plan.
#[verb("explain")]
pub fn explain(plan_id: String) -> Result<RepairExplainResult> {
    let svc = RepairService::new();
    let (instance_id, plan) = svc
        .find_plan(&plan_id)
        .map_err(NounVerbError::execution_error)?;
    Ok(RepairExplainResult {
        plan_id: plan.plan_id,
        instance_id,
        description: plan.description,
        actions: plan.actions,
        status: plan.status,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairApplyResult {
    pub plan_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub instance_id: String,
}

/// Mark a repair plan as applied (status → "ADMITTED").
#[verb("apply")]
pub fn apply(plan_id: String) -> Result<RepairApplyResult> {
    let svc = RepairService::new();
    svc.apply(&plan_id).map_err(NounVerbError::execution_error)
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairRollbackResult {
    pub plan_id: String,
    pub previous_status: String,
    pub new_status: String,
    pub instance_id: String,
}

/// Revert a repair plan from "ADMITTED" back to "OPEN".
#[verb("rollback")]
pub fn rollback(plan_id: String) -> Result<RepairRollbackResult> {
    let svc = RepairService::new();
    svc.rollback(&plan_id)
        .map_err(NounVerbError::execution_error)
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairDryRunResult {
    pub plan_id: String,
    pub current_status: String,
    pub projected_status: String,
    pub actions: Vec<String>,
    pub would_modify: bool,
}

/// Preview what apply would do without modifying state.
#[verb("dry-run")]
pub fn dry_run(plan_id: String) -> Result<RepairDryRunResult> {
    let svc = RepairService::new();
    svc.dry_run(&plan_id)
        .map_err(NounVerbError::execution_error)
}

// ==============================================================================
// 4. Tests
// ==============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use lsp_max_runtime::{AutonomicMesh, LspInstance};
    use std::env;

    fn make_temp_service_with_plan() -> (tempfile::NamedTempFile, RepairService, RepairPlan) {
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-1"));

        let plan = RepairPlan {
            plan_id: "plan-alpha".to_string(),
            description: "Resolve WASM4PM-001 boundary violation".to_string(),
            actions: vec![
                "remove forbidden import".to_string(),
                "run dx-verify".to_string(),
            ],
            status: "OPEN".to_string(),
        };

        let f = tempfile::NamedTempFile::new().expect("tempfile");
        let svc = RepairService {
            state_path: f.path().to_str().unwrap().to_string(),
        };
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();
        svc.seed_plan("inst-1", plan.clone()).unwrap();
        (f, svc, plan)
    }

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        crate::nouns::TEST_ENV_LOCK
            .lock()
            .unwrap_or_else(|p| p.into_inner())
    }

    #[test]
    fn list_returns_empty_when_no_plans() {
        let _guard = lock_env();
        let f = tempfile::NamedTempFile::new().unwrap();
        let mut mesh = AutonomicMesh::new();
        mesh.add_instance(LspInstance::new("inst-x"));
        mesh.save_to_file(f.path().to_str().unwrap()).unwrap();

        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let svc = RepairService::new();
        let result = svc.list(None).unwrap();
        assert_eq!(result.total, 0);
        assert!(result.plans.is_empty());
    }

    #[test]
    fn list_returns_all_plans() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let result = svc.list(None).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.plans[0].plan_id, "plan-alpha");
        assert_eq!(result.plans[0].instance_id, "inst-1");
        assert_eq!(result.plans[0].action_count, 2);
    }

    #[test]
    fn list_filters_by_instance_id() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let filtered = svc.list(Some("inst-1")).unwrap();
        assert_eq!(filtered.total, 1);

        let empty = svc.list(Some("inst-none")).unwrap();
        assert_eq!(empty.total, 0);
    }

    #[test]
    fn explain_finds_plan_by_id() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let (iid, found) = svc.find_plan("plan-alpha").unwrap();
        assert_eq!(iid, "inst-1");
        assert_eq!(found.plan_id, "plan-alpha");
        assert_eq!(found.status, "OPEN");
    }

    #[test]
    fn explain_missing_plan_returns_err() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let result = svc.find_plan("no-such-plan");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("PLAN_NOT_FOUND"));
    }

    #[test]
    fn apply_transitions_status_to_admitted() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let result = svc.apply("plan-alpha").unwrap();
        assert_eq!(result.previous_status, "OPEN");
        assert_eq!(result.new_status, "ADMITTED");
        assert_eq!(result.instance_id, "inst-1");

        // Verify persistence.
        let (_, after) = svc.find_plan("plan-alpha").unwrap();
        assert_eq!(after.status, "ADMITTED");
    }

    #[test]
    fn apply_missing_plan_returns_err() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        assert!(svc.apply("ghost-plan").is_err());
    }

    #[test]
    fn rollback_admitted_plan_to_open() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        svc.apply("plan-alpha").unwrap();
        let result = svc.rollback("plan-alpha").unwrap();
        assert_eq!(result.previous_status, "ADMITTED");
        assert_eq!(result.new_status, "OPEN");
    }

    #[test]
    fn rollback_non_admitted_plan_returns_err() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let result = svc.rollback("plan-alpha");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("PLAN_NOT_ADMITTED"));
    }

    #[test]
    fn dry_run_shows_projected_admitted_without_mutating() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        let result = svc.dry_run("plan-alpha").unwrap();
        assert_eq!(result.current_status, "OPEN");
        assert_eq!(result.projected_status, "ADMITTED");
        assert!(result.would_modify);

        // State must remain unchanged.
        let (_, after) = svc.find_plan("plan-alpha").unwrap();
        assert_eq!(after.status, "OPEN", "dry_run must not mutate state");
    }

    #[test]
    fn dry_run_already_admitted_would_not_modify() {
        let _guard = lock_env();
        let (f, svc, _plan) = make_temp_service_with_plan();
        // SAFETY: protected by TEST_ENV_LOCK
        unsafe { env::set_var("LSP_MAX_STATE_PATH", f.path().to_str().unwrap()) };
        svc.apply("plan-alpha").unwrap();
        let result = svc.dry_run("plan-alpha").unwrap();
        assert!(
            !result.would_modify,
            "plan already ADMITTED should not need re-apply"
        );
    }
}
