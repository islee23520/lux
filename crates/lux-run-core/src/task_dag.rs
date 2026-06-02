use std::collections::{HashMap, HashSet};

use chrono::Utc;
use lux_spec_core::{DomainSpec, Requirement, RoadmapTicket, SpecProject};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TaskStatus {
    Pending,
    InProgress,
    /// Dispatched to a team-mode worker; awaiting accepted execution+verification evidence.
    /// A node in this state does NOT count as Done for dependency resolution.
    AwaitingEvidence,
    Done,
    Blocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNode {
    pub id: String,
    pub spec_clause_id: String,
    pub title: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub assignee: Option<String>,
    pub evidence_path: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskNodeProjection {
    pub id: String,
    pub spec_clause_id: String,
    pub title: String,
    pub status: TaskStatus,
    pub dependencies: Vec<String>,
    pub assignee: Option<String>,
    pub evidence_path: Option<String>,
    pub blocked_by: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDAG {
    pub nodes: HashMap<String, TaskNode>,
    pub edges: Vec<(String, String)>,
    pub root_ids: Vec<String>,
}

impl TaskDAG {
    pub fn add_node(&mut self, node: TaskNode) {
        if node.dependencies.is_empty() && !self.root_ids.contains(&node.id) {
            self.root_ids.push(node.id.clone());
        }
        for dependency in &node.dependencies {
            let edge = (dependency.clone(), node.id.clone());
            if !self.edges.contains(&edge) {
                self.edges.push(edge);
            }
        }
        self.nodes.insert(node.id.clone(), node);
        self.rebuild_roots();
    }

    pub fn add_dependency(&mut self, node_id: &str, dependency_id: &str) {
        self.try_add_dependency(node_id, dependency_id)
            .expect("TaskDAG dependency cycle detected");
    }

    pub fn try_add_dependency(&mut self, node_id: &str, dependency_id: &str) -> Result<(), String> {
        if self.would_create_cycle(node_id, dependency_id) {
            return Err(format!(
                "dependency cycle detected: adding {dependency_id} before {node_id} would create a cycle"
            ));
        }
        if let Some(node) = self.nodes.get_mut(node_id) {
            let dependency = dependency_id.to_string();
            if !node.dependencies.contains(&dependency) {
                node.dependencies.push(dependency.clone());
            }
            let edge = (dependency, node_id.to_string());
            if !self.edges.contains(&edge) {
                self.edges.push(edge);
            }
            self.rebuild_roots();
        }
        Ok(())
    }

    pub fn would_create_cycle(&self, node_id: &str, dependency_id: &str) -> bool {
        node_id == dependency_id || self.has_path(node_id, dependency_id)
    }

    pub fn has_path(&self, from_id: &str, to_id: &str) -> bool {
        if from_id == to_id {
            return true;
        }
        let mut stack = vec![from_id.to_string()];
        let mut visited = HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            for (_, dependent) in self
                .edges
                .iter()
                .filter(|(dependency, _)| dependency == &current)
            {
                if dependent == to_id {
                    return true;
                }
                stack.push(dependent.clone());
            }
        }
        false
    }

    pub fn ready_nodes(&self) -> Vec<TaskNode> {
        let mut ready = self
            .topological_ids()
            .into_iter()
            .filter_map(|id| self.nodes.get(&id))
            .filter(|node| node.status == TaskStatus::Pending)
            .filter(|node| {
                node.dependencies.iter().all(|dependency| {
                    self.nodes
                        .get(dependency)
                        .is_some_and(|dep| dep.status == TaskStatus::Done)
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        ready.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        ready
    }

    pub fn mark_done(&mut self, node_id: &str, evidence_path: Option<String>) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = TaskStatus::Done;
            node.evidence_path = evidence_path;
        }
    }

    pub fn mark_blocked(&mut self, node_id: &str, reason: Option<String>) {
        if let Some(node) = self.nodes.get_mut(node_id) {
            node.status = TaskStatus::Blocked;
            node.evidence_path = reason;
        }
    }

    pub fn projection(&self) -> Vec<TaskNodeProjection> {
        self.topological_ids()
            .into_iter()
            .filter_map(|id| self.nodes.get(&id))
            .map(|node| {
                let blocked_by = node
                    .dependencies
                    .iter()
                    .filter(|dependency| {
                        self.nodes
                            .get(*dependency)
                            .is_none_or(|dep| dep.status != TaskStatus::Done)
                    })
                    .cloned()
                    .collect();
                TaskNodeProjection {
                    id: node.id.clone(),
                    spec_clause_id: node.spec_clause_id.clone(),
                    title: node.title.clone(),
                    status: node.status.clone(),
                    dependencies: node.dependencies.clone(),
                    assignee: node.assignee.clone(),
                    evidence_path: node.evidence_path.clone(),
                    blocked_by,
                }
            })
            .collect()
    }

    pub fn from_spec(spec: &SpecProject) -> TaskDAG {
        let mut dag = TaskDAG::default();
        let mut clause_to_task = HashMap::new();

        for (domain_name, domain) in domain_specs(spec) {
            for requirement in &domain.requirements {
                let node = node_from_requirement(&domain_name, requirement);
                clause_to_task.insert(requirement.id.clone(), node.id.clone());
                dag.add_node(node);
            }
        }

        for (domain_name, domain) in domain_specs(spec) {
            for requirement in &domain.requirements {
                let Some(node_id) = clause_to_task.get(&requirement.id).cloned() else {
                    continue;
                };
                for dependency in &requirement.depends_on {
                    let dependency_id = clause_to_task
                        .get(dependency)
                        .cloned()
                        .unwrap_or_else(|| task_id(&domain_name, dependency));
                    dag.add_dependency(&node_id, &dependency_id);
                }
            }
        }

        for ticket in &spec.roadmap.tickets {
            let node = node_from_ticket(ticket);
            let node_id = node.id.clone();
            dag.add_node(node);
            for requirement_ref in &ticket.requirement_refs {
                if let Some(dependency_id) = clause_to_task.get(requirement_ref) {
                    dag.add_dependency(&node_id, dependency_id);
                }
            }
        }

        if dag.nodes.is_empty() {
            dag.add_node(TaskNode {
                id: "task-spec-review".to_string(),
                spec_clause_id: spec.project_id.clone(),
                title: format!("Review spec for {}", spec.project_name),
                status: TaskStatus::Pending,
                dependencies: Vec::new(),
                assignee: None,
                evidence_path: None,
                created_at: Utc::now().to_rfc3339(),
            });
        }

        dag.rebuild_roots();
        dag
    }

    fn rebuild_roots(&mut self) {
        let dependency_targets = self
            .edges
            .iter()
            .map(|(_, target)| target.clone())
            .collect::<HashSet<_>>();
        let mut roots = self
            .nodes
            .keys()
            .filter(|id| !dependency_targets.contains(*id))
            .cloned()
            .collect::<Vec<_>>();
        roots.sort();
        self.root_ids = roots;
    }

    pub fn topological_ids_checked(&self) -> Result<Vec<String>, String> {
        let mut emitted = HashSet::new();
        let mut ordered = Vec::new();
        let mut ids = self.nodes.keys().cloned().collect::<Vec<_>>();
        ids.sort();

        while ordered.len() < self.nodes.len() {
            let before = ordered.len();
            for id in &ids {
                if emitted.contains(id) {
                    continue;
                }
                let dependencies_done = self
                    .nodes
                    .get(id)
                    .map(|node| node.dependencies.iter().all(|dep| emitted.contains(dep)))
                    .unwrap_or(false);
                if dependencies_done {
                    emitted.insert(id.clone());
                    ordered.push(id.clone());
                }
            }
            if ordered.len() == before {
                let remaining = ids
                    .iter()
                    .filter(|id| !emitted.contains(*id))
                    .cloned()
                    .collect::<Vec<_>>();
                return Err(format!(
                    "dependency cycle detected among task node(s): {}",
                    remaining.join(", ")
                ));
            }
        }

        Ok(ordered)
    }

    fn topological_ids(&self) -> Vec<String> {
        self.topological_ids_checked()
            .expect("TaskDAG contains a dependency cycle")
    }
}

fn domain_specs(spec: &SpecProject) -> Vec<(String, &DomainSpec)> {
    let mut domains = Vec::new();
    for (name, domain) in [
        ("design", spec.domains.design.as_ref()),
        ("architecture", spec.domains.architecture.as_ref()),
        ("art-style", spec.domains.art_style.as_ref()),
        ("audio", spec.domains.audio.as_ref()),
        ("narrative", spec.domains.narrative.as_ref()),
        ("levels", spec.domains.levels.as_ref()),
        ("ui-ux", spec.domains.ui_ux.as_ref()),
    ] {
        if let Some(domain) = domain {
            domains.push((name.to_string(), domain));
        }
    }
    for (name, domain) in &spec.domains.custom {
        domains.push((name.clone(), domain));
    }
    domains
}

fn node_from_requirement(domain_name: &str, requirement: &Requirement) -> TaskNode {
    let id = task_id(domain_name, &requirement.id);
    TaskNode {
        id,
        spec_clause_id: requirement.id.clone(),
        title: requirement.text.clone(),
        status: TaskStatus::Pending,
        dependencies: Vec::new(),
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn node_from_ticket(ticket: &RoadmapTicket) -> TaskNode {
    TaskNode {
        id: task_id(ticket.domain.as_deref().unwrap_or("roadmap"), &ticket.id),
        spec_clause_id: ticket.id.clone(),
        title: ticket.title.clone(),
        status: TaskStatus::Pending,
        dependencies: Vec::new(),
        assignee: None,
        evidence_path: None,
        created_at: Utc::now().to_rfc3339(),
    }
}

fn task_id(domain_name: &str, clause_id: &str) -> String {
    let domain = slug(domain_name);
    let clause = slug(clause_id);
    format!("task-{domain}-{clause}")
}

fn slug(value: &str) -> String {
    let slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "unnamed".to_string()
    } else {
        slug
    }
}
