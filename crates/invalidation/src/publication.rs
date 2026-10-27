//! In-memory generation fencing for consumers of the invalidation planner.
//! Publication is atomic only within this process. This is not a durable cache.
use crate::{Graph, GraphError, Plan, plan};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Mutex;

#[derive(Debug)]
pub enum Error {
    Graph(GraphError),
    Poisoned,
    GenerationExhausted,
    StaleGeneration,
    NoPendingBuild,
    IncompleteResults,
}
impl From<GraphError> for Error {
    fn from(value: GraphError) -> Self {
        Self::Graph(value)
    }
}
struct State {
    graph: Graph,
    generation: u64,
    outputs: BTreeMap<String, Vec<u8>>,
    pending: Option<BTreeSet<String>>,
}
pub struct Coordinator {
    state: Mutex<State>,
}
pub struct Build {
    pub generation: u64,
    pub plan: Plan,
}
impl Coordinator {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(State {
                graph: Graph::new(BTreeMap::new()).expect("empty graph is valid"),
                generation: 0,
                outputs: BTreeMap::new(),
                pending: None,
            }),
        }
    }

    /// Install a new graph and withdraw dirty results before any work begins.
    /// Unfinished prior builds are fenced and their missing results remain dirty.
    pub fn begin(&self, graph: Graph, changed: &BTreeSet<String>) -> Result<Build, Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        let generation = state
            .generation
            .checked_add(1)
            .ok_or(Error::GenerationExhausted)?;
        let mut forced = changed.clone();
        forced.extend(
            state
                .graph
                .nodes
                .keys()
                .filter(|key| !state.outputs.contains_key(*key))
                .cloned(),
        );
        let result = plan(&state.graph, &graph, &forced)?;
        let pending: BTreeSet<String> = result.rebuild.iter().cloned().collect();
        state
            .outputs
            .retain(|name, _| !pending.contains(name) && !result.removed.contains(name));
        state.generation = generation;
        state.graph = graph;
        state.pending = Some(pending);
        Ok(Build {
            generation,
            plan: result,
        })
    }

    /// Publish a complete set at once. Outdated generations and partial sets fail
    /// before mutating visible results. Output validity remains the caller's duty.
    pub fn publish(
        &self,
        generation: u64,
        outputs: BTreeMap<String, Vec<u8>>,
    ) -> Result<(), Error> {
        let mut state = self.state.lock().map_err(|_| Error::Poisoned)?;
        if state.generation != generation {
            return Err(Error::StaleGeneration);
        }
        let pending = state.pending.as_ref().ok_or(Error::NoPendingBuild)?;
        if outputs.keys().ne(pending.iter()) {
            return Err(Error::IncompleteResults);
        }
        state.outputs.extend(outputs);
        state.pending = None;
        Ok(())
    }
    pub fn output(&self, name: &str) -> Result<Option<Vec<u8>>, Error> {
        let state = self.state.lock().map_err(|_| Error::Poisoned)?;
        Ok(state.outputs.get(name).cloned())
    }
}
impl Default for Coordinator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Node;
    fn graph(value: u8) -> Graph {
        Graph::new(BTreeMap::from([
            (
                "source".into(),
                Node {
                    fingerprint: [value; 32],
                    dependencies: BTreeSet::new(),
                },
            ),
            (
                "binary".into(),
                Node {
                    fingerprint: [0; 32],
                    dependencies: BTreeSet::from(["source".into()]),
                },
            ),
        ]))
        .unwrap()
    }
    fn results() -> BTreeMap<String, Vec<u8>> {
        BTreeMap::from([("source".into(), vec![1]), ("binary".into(), vec![2])])
    }
    #[test]
    fn stale_and_partial_results_cannot_be_published() {
        let coordinator = Coordinator::new();
        let first = coordinator.begin(graph(0), &BTreeSet::new()).unwrap();
        assert!(matches!(
            coordinator.publish(first.generation, BTreeMap::new()),
            Err(Error::IncompleteResults)
        ));
        assert_eq!(coordinator.output("source").unwrap(), None);
        let second = coordinator.begin(graph(0), &BTreeSet::new()).unwrap();
        assert_eq!(second.plan.rebuild.len(), 2);
        assert!(matches!(
            coordinator.publish(first.generation, results()),
            Err(Error::StaleGeneration)
        ));
        coordinator.publish(second.generation, results()).unwrap();
        assert_eq!(coordinator.output("binary").unwrap(), Some(vec![2]));
        assert!(matches!(
            coordinator.publish(second.generation, results()),
            Err(Error::NoPendingBuild)
        ));
        let third = coordinator.begin(graph(1), &BTreeSet::new()).unwrap();
        assert_eq!(coordinator.output("binary").unwrap(), None);
        assert_eq!(third.plan.rebuild.len(), 2);
    }
    #[test]
    fn failed_begin_does_not_change_current_generation() {
        let coordinator = Coordinator::new();
        let build = coordinator.begin(graph(0), &BTreeSet::new()).unwrap();
        assert!(matches!(
            coordinator.begin(graph(0), &BTreeSet::from(["unknown".into()])),
            Err(Error::Graph(GraphError::UnknownChange(_)))
        ));
        coordinator.publish(build.generation, results()).unwrap();
    }
}
