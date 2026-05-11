use crate::client::Model;
use crate::config::{
    pipeline_stage_admissible, Config, EntityRef, PipelineNode, Role, RoleLike,
};
use crate::function::FunctionDeclaration;

use anyhow::{bail, Result};
use std::collections::HashSet;

/// Pre-flight validation of model capabilities against what the role/input requires.
/// Runs before any API call; all checks are deterministic and zero-token.
/// This can be thought of as the beginnings of our 'aichat' compiler, allowing us to look forward
/// at the target model before submitting the data to the backend.
/// I have an exploration of this idea in docs/analysis/2026-04-16-model-aware-compilation.md
///
/// Returns `Err` for hard mismatches (tools vs. non-function-calling model, images vs.
/// non-vision model). The caller should surface the error as a config error.
pub fn validate_model_capabilities(
    model: &Model,
    role: &Role,
    functions: Option<&[FunctionDeclaration]>,
    has_images: bool,
) -> Result<()> {
    let will_send_tools = functions.map(|f| !f.is_empty()).unwrap_or(false);
    if will_send_tools && !model.data().supports_function_calling {
        bail!(
            "Preflight: role '{}' requires tool calling but model '{}' does not support it. \
             Remove `use_tools` from the role or switch to a function-calling model.",
            role.name(),
            model.id()
        );
    }

    if has_images && !model.data().supports_vision {
        bail!(
            "Preflight: input contains images but model '{}' does not support vision. \
             Switch to a vision-capable model.",
            model.id()
        );
    }

    Ok(())
}

/// Pre-flight validation for a pipeline: each stage's role must exist and its model
/// (explicit or inherited) must support the role's requirements.
///
/// Output-schema/input-schema compatibility between stage N and stage N+1 is a
/// deterministic check we _could_ do here, but JSON-schema compatibility is subtle
/// (subset relations, anyOf/oneOf, etc.) — defer to schema validation at runtime
/// rather than duplicate the logic.
pub fn validate_pipeline_stages(
    config: &Config,
    stages: &[(String, Option<String>)],
) -> Result<()> {
    for (index, (raw_name, model_id)) in stages.iter().enumerate() {
        // Phase 19B/C: classify the stage name first. Agents and macros need
        // different handling than roles.
        let entity = config.classify_entity(raw_name).map_err(|e| {
            anyhow::anyhow!(
                "Preflight: pipeline stage {} references unknown entity '{}': {}",
                index + 1,
                raw_name,
                e
            )
        })?;
        pipeline_stage_admissible(&entity).map_err(|e| {
            anyhow::anyhow!("Preflight: pipeline stage {}: {}", index + 1, e)
        })?;

        // Phase 19C: agent-stage capability validation requires async
        // `Agent::init` and is deferred to stage execution. We've confirmed
        // the agent name exists (classification passed) — that's the
        // strongest sync check we can offer here.
        let role_name = match &entity {
            EntityRef::Role(name) => name.clone(),
            EntityRef::Agent(_) => continue,
            EntityRef::Macro(_) => unreachable!("rejected by pipeline_stage_admissible"),
        };

        let role = config.retrieve_role(&role_name).map_err(|e| {
            anyhow::anyhow!(
                "Preflight: pipeline stage {} failed to load role '{}': {}",
                index + 1,
                role_name,
                e
            )
        })?;

        let model = match model_id {
            Some(id) => {
                let listed = crate::client::list_models(config, crate::client::ModelType::Chat);
                match listed.iter().find(|m| m.id() == *id) {
                    Some(m) => (*m).clone(),
                    None => bail!(
                        "Preflight: pipeline stage {} references unknown model '{}'",
                        index + 1,
                        id
                    ),
                }
            }
            None => role.model().clone(),
        };

        if role.use_tools().is_some() && !model.data().supports_function_calling {
            bail!(
                "Preflight: pipeline stage {} role '{}' requires tool calling but model \
                 '{}' does not support it",
                index + 1,
                role_name,
                model.id()
            );
        }
    }

    Ok(())
}

/// Phase 21D: detect cycles in the pipeline-role reference graph.
/// A pipeline role A whose stages reference another pipeline role B
/// (which itself references A, directly or transitively) would loop
/// infinitely through tool dispatch. Catch the cycle deterministically
/// at preflight before any LLM call.
///
/// `entry` is the name of the role whose pipeline we're about to run.
/// `nodes` is its DAG. We walk every leaf stage; if the stage resolves
/// to another pipeline role, we recurse into that role's pipeline,
/// extending the visit chain. Repeating a name → cycle.
pub fn validate_pipeline_dag_cycles(
    config: &Config,
    entry: &str,
    nodes: &[PipelineNode],
) -> Result<()> {
    let mut chain: Vec<String> = vec![entry.to_string()];
    walk_pipeline_nodes(config, nodes, &mut chain)
}

fn walk_pipeline_nodes(
    config: &Config,
    nodes: &[PipelineNode],
    chain: &mut Vec<String>,
) -> Result<()> {
    for n in nodes {
        for stage in n.all_stages() {
            check_stage_for_cycle(config, &stage.role, chain)?;
        }
        for merger in n.merge_role_names() {
            check_stage_for_cycle(config, &merger, chain)?;
        }
    }
    Ok(())
}

fn check_stage_for_cycle(
    config: &Config,
    stage_role: &str,
    chain: &mut Vec<String>,
) -> Result<()> {
    // Reuse the role classifier so we don't double-error on agents/macros.
    // Pipeline-role cycles only apply to actual roles — agents have their
    // own tool semantics and macros aren't admissible as pipeline stages.
    let entity = match config.classify_entity(stage_role) {
        Ok(e) => e,
        Err(_) => return Ok(()), // unknown — surfaced separately by validate_pipeline_stages
    };
    let resolved_role_name = match entity {
        EntityRef::Role(name) => name,
        _ => return Ok(()),
    };

    if chain.iter().any(|s| s == &resolved_role_name) {
        let mut path = chain.clone();
        path.push(resolved_role_name.clone());
        bail!(
            "Preflight: pipeline cycle detected — {} (a role's pipeline cannot \
             transitively reference itself)",
            path.join(" -> ")
        );
    }

    let role = match config.retrieve_role(&resolved_role_name) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };
    if !role.is_pipeline() {
        return Ok(());
    }
    let nodes = match role.pipeline() {
        Some(n) => n.to_vec(),
        None => return Ok(()),
    };

    chain.push(resolved_role_name);
    let res = walk_pipeline_nodes(config, &nodes, chain);
    chain.pop();
    res
}

/// Phase 21D: walk the DAG and ensure every node's structural invariants
/// hold (delegates to `PipelineNode::structural_check`) and that no
/// switch declares dead branches. Currently `structural_check` covers
/// the empty-branches / double-otherwise cases; we additionally detect
/// `when:` branches placed *after* an `otherwise:` and warn — the
/// runtime order-evaluation makes them reachable, but YAML readers tend
/// to assume order-matters, and putting otherwise last is the
/// universally-clear pattern.
pub fn validate_pipeline_dag_structure(nodes: &[PipelineNode]) -> Result<()> {
    let mut seen: HashSet<usize> = HashSet::new();
    for (i, n) in nodes.iter().enumerate() {
        n.structural_check()?;
        if !seen.insert(i) {
            // Defensive — indexes are unique by construction.
        }
        check_switch_branch_order(n)?;
    }
    Ok(())
}

fn check_switch_branch_order(n: &PipelineNode) -> Result<()> {
    match n {
        PipelineNode::Stage(_) => Ok(()),
        PipelineNode::Parallel(p) => {
            for b in &p.branches {
                check_switch_branch_order(b)?;
            }
            Ok(())
        }
        PipelineNode::Switch(s) => {
            let mut saw_otherwise = false;
            for b in &s.branches {
                if saw_otherwise && b.predicate.is_some() {
                    bail!(
                        "Switch branch order is misleading: a `when:` clause \
                         appears after `otherwise:`. Move `otherwise:` to the \
                         last position so reading order matches evaluation."
                    );
                }
                if b.predicate.is_none() {
                    saw_otherwise = true;
                }
                check_switch_branch_order(&b.node)?;
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_with(tools: bool, vision: bool) -> Model {
        let mut m = Model::new("test", "m");
        m.data_mut().supports_function_calling = tools;
        m.data_mut().supports_vision = vision;
        m
    }

    fn one_tool() -> Vec<FunctionDeclaration> {
        vec![FunctionDeclaration::tool_search()]
    }

    #[test]
    fn passes_when_no_tools_and_no_images() {
        let m = model_with(false, false);
        let r = Role::default();
        assert!(validate_model_capabilities(&m, &r, None, false).is_ok());
    }

    #[test]
    fn rejects_tools_on_non_function_calling_model() {
        let m = model_with(false, false);
        let r = Role::default();
        let decls = one_tool();
        let err = validate_model_capabilities(&m, &r, Some(&decls), false).unwrap_err();
        assert!(err.to_string().contains("does not support it"));
    }

    #[test]
    fn accepts_tools_on_function_calling_model() {
        let m = model_with(true, false);
        let r = Role::default();
        let decls = one_tool();
        assert!(validate_model_capabilities(&m, &r, Some(&decls), false).is_ok());
    }

    #[test]
    fn rejects_images_on_non_vision_model() {
        let m = model_with(false, false);
        let r = Role::default();
        let err = validate_model_capabilities(&m, &r, None, true).unwrap_err();
        assert!(err.to_string().contains("does not support vision"));
    }

    #[test]
    fn accepts_images_on_vision_model() {
        let m = model_with(false, true);
        let r = Role::default();
        assert!(validate_model_capabilities(&m, &r, None, true).is_ok());
    }

    // ----- Phase 21D: DAG structural validation -----

    fn yaml_node(yaml: &str) -> PipelineNode {
        let v: serde_json::Value = serde_yaml::from_str(yaml).unwrap();
        crate::config::role::parse_pipeline_node(&v).unwrap()
    }

    #[test]
    fn dag_structural_rejects_when_after_otherwise() {
        let n = yaml_node(
            r#"
switch:
  - when: { contains: "x" }
    role: a
  - otherwise: true
    role: b
  - when: { contains: "y" }
    role: c
"#,
        );
        let err = validate_pipeline_dag_structure(&[n]).unwrap_err();
        assert!(err.to_string().contains("after `otherwise:`"));
    }

    #[test]
    fn dag_structural_accepts_otherwise_last() {
        let n = yaml_node(
            r#"
switch:
  - when: { contains: "x" }
    role: a
  - when: { contains: "y" }
    role: b
  - otherwise: true
    role: c
"#,
        );
        assert!(validate_pipeline_dag_structure(&[n]).is_ok());
    }

    #[test]
    fn dag_structural_recurses_into_parallel_branches() {
        let n = yaml_node(
            r#"
parallel:
  - role: a
  - switch:
      - when: { contains: "x" }
        role: b
      - otherwise: true
        role: c
      - when: { contains: "y" }
        role: d
"#,
        );
        let err = validate_pipeline_dag_structure(&[n]).unwrap_err();
        assert!(err.to_string().contains("after `otherwise:`"));
    }
}
