//! Conservative invalidation for explicit immutable dependency graphs.
//! The caller must discover every input and supply trustworthy fingerprints.
//! This crate does not discover inputs, hash files, or execute builds.
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Node {
    pub fingerprint: [u8; 32],
    pub dependencies: BTreeSet<String>,
}

/// Validated acyclic snapshot, with edges from nodes to dependencies.
#[derive(Clone, Debug)]
pub struct Graph {
    nodes: BTreeMap<String, Node>,
    reverse: BTreeMap<String, BTreeSet<String>>,
    order: Vec<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GraphError {
    MissingDependency { node: String, dependency: String },
    Cycle { blocked: Vec<String> },
    UnknownChange(String),
}
impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDependency { node, dependency } => {
                write!(f, "{node:?} depends on missing node {dependency:?}")
            }
            Self::Cycle { blocked } => write!(f, "cycle detected; blocked nodes: {blocked:?}"),
            Self::UnknownChange(node) => {
                write!(f, "changed node {node:?} is absent from both snapshots")
            }
        }
    }
}
impl std::error::Error for GraphError {}

impl Graph {
    pub fn new(nodes: BTreeMap<String, Node>) -> Result<Self, GraphError> {
        let mut reverse: BTreeMap<String, BTreeSet<String>> = nodes
            .keys()
            .map(|key| (key.clone(), BTreeSet::new()))
            .collect();
        let mut remaining = BTreeMap::new();
        for (name, node) in &nodes {
            remaining.insert(name.clone(), node.dependencies.len());
            for dependency in &node.dependencies {
                let Some(dependents) = reverse.get_mut(dependency) else {
                    return Err(GraphError::MissingDependency {
                        node: name.clone(),
                        dependency: dependency.clone(),
                    });
                };
                dependents.insert(name.clone());
            }
        }
        let mut ready: BTreeSet<String> = remaining
            .iter()
            .filter_map(|(name, count)| (*count == 0).then_some(name.clone()))
            .collect();
        let mut order = Vec::with_capacity(nodes.len());
        while let Some(name) = ready.pop_first() {
            order.push(name.clone());
            for dependent in &reverse[&name] {
                let count = remaining.get_mut(dependent).expect("validated graph");
                *count -= 1;
                if *count == 0 {
                    ready.insert(dependent.clone());
                }
            }
        }
        if order.len() != nodes.len() {
            return Err(GraphError::Cycle {
                blocked: remaining
                    .into_iter()
                    .filter_map(|(name, count)| (count != 0).then_some(name))
                    .collect(),
            });
        }
        Ok(Self {
            nodes,
            reverse,
            order,
        })
    }
    pub fn order(&self) -> &[String] {
        &self.order
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Plan {
    /// Dirty nodes in the new snapshot, dependencies before dependents.
    pub rebuild: Vec<String>,
    /// Removed nodes whose old results must not be published.
    pub removed: BTreeSet<String>,
}

/// Propagate changes across the union of old and new edges, conservatively.
/// Explicit events force invalidation even when fingerprints match.
pub fn plan(old: &Graph, new: &Graph, changed: &BTreeSet<String>) -> Result<Plan, GraphError> {
    for name in changed {
        if !old.nodes.contains_key(name) && !new.nodes.contains_key(name) {
            return Err(GraphError::UnknownChange(name.clone()));
        }
    }
    let all: BTreeSet<_> = old.nodes.keys().chain(new.nodes.keys()).cloned().collect();
    let mut dirty: BTreeSet<String> = all
        .iter()
        .filter(|name| old.nodes.get(*name) != new.nodes.get(*name))
        .cloned()
        .collect();
    dirty.extend(changed.iter().cloned());
    let mut queue: VecDeque<_> = dirty.iter().cloned().collect();
    while let Some(name) = queue.pop_front() {
        for graph in [old, new] {
            if let Some(dependents) = graph.reverse.get(&name) {
                for dependent in dependents {
                    if dirty.insert(dependent.clone()) {
                        queue.push_back(dependent.clone());
                    }
                }
            }
        }
    }
    let rebuild = new
        .order
        .iter()
        .filter(|name| dirty.contains(*name))
        .cloned()
        .collect();
    let removed = old
        .nodes
        .keys()
        .filter(|name| !new.nodes.contains_key(*name))
        .cloned()
        .collect();
    Ok(Plan { rebuild, removed })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn node(value: u8, deps: &[&str]) -> Node {
        Node {
            fingerprint: [value; 32],
            dependencies: deps.iter().map(|x| (*x).to_owned()).collect(),
        }
    }
    fn graph(entries: &[(&str, Node)]) -> Graph {
        Graph::new(
            entries
                .iter()
                .map(|(name, node)| ((*name).to_owned(), node.clone()))
                .collect(),
        )
        .unwrap()
    }
    fn set(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|s| (*s).into()).collect()
    }
    #[test]
    fn changes_propagate_in_dependency_order() {
        let old = graph(&[
            ("a", node(0, &[])),
            ("b", node(0, &["a"])),
            ("c", node(0, &["b"])),
            ("z", node(0, &[])),
        ]);
        let new = graph(&[
            ("a", node(1, &[])),
            ("b", node(0, &["a"])),
            ("c", node(0, &["b"])),
            ("z", node(0, &[])),
        ]);
        assert_eq!(
            plan(&old, &new, &set(&[])).unwrap().rebuild,
            vec!["a", "b", "c"]
        );
        assert!(plan(&old, &old, &set(&[])).unwrap().rebuild.is_empty());
    }
    #[test]
    fn removals_and_changed_edges_invalidate_consumers() {
        let old = graph(&[
            ("a", node(0, &[])),
            ("b", node(0, &["a"])),
            ("c", node(0, &["b"])),
        ]);
        let new = graph(&[("b", node(0, &[])), ("c", node(0, &["b"]))]);
        let result = plan(&old, &new, &set(&[])).unwrap();
        assert_eq!(result.removed, set(&["a"]));
        assert_eq!(result.rebuild, vec!["b", "c"]);
    }
    #[test]
    fn unknown_events_missing_edges_and_cycles_fail() {
        let empty = graph(&[]);
        assert_eq!(
            plan(&empty, &empty, &set(&["missing"])).unwrap_err(),
            GraphError::UnknownChange("missing".into())
        );
        assert!(matches!(
            Graph::new(BTreeMap::from([("a".into(), node(0, &["b"]))])),
            Err(GraphError::MissingDependency { .. })
        ));
        assert!(matches!(
            Graph::new(BTreeMap::from([
                ("a".into(), node(0, &["b"])),
                ("b".into(), node(0, &["a"]))
            ])),
            Err(GraphError::Cycle { .. })
        ));
    }
    #[test]
    fn forced_events_propagate_with_equal_fingerprints() {
        let g = graph(&[("a", node(0, &[])), ("b", node(0, &["a"]))]);
        assert_eq!(plan(&g, &g, &set(&["a"])).unwrap().rebuild, vec!["a", "b"]);
    }
    #[test]
    fn generated_dags_match_independent_reachability_oracle() {
        let mut state: u64 = 0xfeedface;
        for _ in 0..500 {
            let mut nodes = BTreeMap::new();
            for i in 0..12 {
                let mut dependencies = BTreeSet::new();
                for j in 0..i {
                    state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                    if state >> 61 == 0 {
                        dependencies.insert(format!("n{j:02}"));
                    }
                }
                nodes.insert(
                    format!("n{i:02}"),
                    Node {
                        fingerprint: [0; 32],
                        dependencies,
                    },
                );
            }
            let g = Graph::new(nodes.clone()).unwrap();
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let seed = format!("n{:02}", state as usize % 12);
            let mut expected = BTreeSet::from([seed.clone()]);
            loop {
                let before = expected.len();
                for (name, node) in &nodes {
                    if node.dependencies.iter().any(|dep| expected.contains(dep)) {
                        expected.insert(name.clone());
                    }
                }
                if before == expected.len() {
                    break;
                }
            }
            let result = plan(&g, &g, &BTreeSet::from([seed])).unwrap();
            assert_eq!(
                result.rebuild.iter().cloned().collect::<BTreeSet<_>>(),
                expected
            );
            let indexes: BTreeMap<_, _> =
                g.order().iter().enumerate().map(|(i, n)| (n, i)).collect();
            for (name, node) in &nodes {
                for dep in &node.dependencies {
                    assert!(indexes[dep] < indexes[name]);
                }
            }
        }
    }
}
