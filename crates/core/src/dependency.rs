use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use crate::error::ScoopResult;

#[derive(Debug)]
struct Dependencies<T>(HashSet<T>);

/// This is the dependencies DAG (Direct Acyclic Graph). The type `T` is
/// intended to be a low cost small type such as string or number.
#[derive(Debug)]
pub struct DepGraph<T> {
    /// Each entry in this HashMap represents a node with its dependencies in
    /// this DepGraph.
    inner: HashMap<T, Dependencies<T>>,
}

impl<T> std::ops::Deref for Dependencies<T> {
    type Target = HashSet<T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Dependencies<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> Dependencies<T>
where
    T: Hash,
{
    #[inline]
    fn new() -> Dependencies<T> {
        Dependencies(HashSet::new())
    }
}

impl<T> DepGraph<T>
where
    T: Hash + Eq + Clone,
{
    /// Create a new dependencies DAG.
    #[inline]
    pub fn new() -> DepGraph<T> {
        DepGraph {
            inner: HashMap::new(),
        }
    }

    /// Register a `node` in the graph, with the `dep_node` to be one of its
    /// dependencies. `dep_node` will also be registered in if it didn't exist.
    #[inline]
    pub fn register_dep<N: Into<T>>(&mut self, node: N, dep_node: N) {
        self._register_dep(node.into(), dep_node.into())
    }

    /// Register a `node` in the graph, with the `dep_nodes` to be a group of
    /// its dependencies. Each node from `dep_nodes` will also be registered in
    /// if it didn't exist.
    #[inline]
    pub fn register_deps<N: Into<T>>(&mut self, node: N, dep_nodes: Vec<N>) {
        let node = node.into();
        dep_nodes
            .into_iter()
            .for_each(|dep_node| self._register_dep(node.clone(), dep_node.into()))
    }

    /// Pop a node which does not have any dependency and can be resolved.
    /// `None` will be returned if all nodes have dependencies.
    ///
    /// If `pop` returns `None` and `len` is not 0, there is cyclic dependencies.
    pub fn pop(&mut self) -> Option<T> {
        let node = self
            .inner
            .iter()
            .filter(|(_, deps)| deps.len() == 0)
            .map(|(node, _)| node)
            .next()
            .map(|node| node.clone());

        if node.is_some() {
            self._remove(node.as_ref().unwrap());
        }

        node
    }

    /// Pop a vector of nodes which do not have any dependency and can be
    /// resolved. `None` will be returned if all nodes have dependencies.
    ///
    /// If `pop_many` returns `None` and `len` is not 0, there is cyclic
    /// dependencies.
    pub fn pop_many(&mut self) -> Vec<T> {
        let nodes = self
            .inner
            .iter()
            .filter(|(_, deps)| deps.len() == 0)
            .map(|(node, _)| node.clone())
            .collect::<Vec<_>>();

        nodes.iter().for_each(|node| {
            self._remove(node);
        });

        nodes
    }

    /// Walk the whole graph and pop all of the resolved nodes. An error will
    /// be returned when cyclic dependencies is detected.
    pub fn walk(&mut self) -> ScoopResult<Vec<Vec<T>>> {
        let mut res = vec![];
        while self.len() > 0 {
            let step = self.pop_many();
            if step.len() == 0 {
                anyhow::bail!("cyclic dependencies detected");
            }
            res.push(step);
        }

        Ok(res)
    }

    /// Walk the whole graph and pop all of the resolved nodes. Nodes will be
    /// flattened into a single vector. An error will be returned when cyclic
    /// dependencies is detected.
    #[inline]
    pub fn flat_walk(&mut self) -> ScoopResult<Vec<T>> {
        self.walk()
            .map(|v| v.into_iter().flatten().collect::<Vec<_>>())
    }

    /// Return the count of unsolved nodes in this DepGraph.
    #[inline]
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Reinitialize this dependencies DAG.
    #[inline]
    pub fn reset(&mut self) {
        self.inner = HashMap::new();
    }

    fn _register_dep(&mut self, node: T, dep_node: T) {
        // register node
        match self.inner.entry(node.clone()) {
            Entry::Vacant(e) => {
                let mut dep = Dependencies::new();
                // Avoid self cyclic dependencies
                if node.ne(&dep_node) {
                    drop(dep.insert(dep_node.clone()));
                }
                drop(e.insert(dep));
            }
            Entry::Occupied(e) => {
                // Avoid self cyclic dependencies
                if node.ne(&dep_node) {
                    e.into_mut().insert(dep_node.clone());
                }
            }
        }

        // register dep_node
        match self.inner.entry(dep_node) {
            Entry::Vacant(e) => {
                drop(e.insert(Dependencies::new()));
            }
            Entry::Occupied(_) => {
                // no-op
            }
        }
    }

    fn _remove(&mut self, node: &T) {
        // 1. remove this node from DepGraph.
        drop(self.inner.remove(node));
        // 2. remove this node from other nodes' dependencies.
        self.inner.iter_mut().for_each(|(_, deps)| {
            drop(deps.remove(node));
        });
    }
}

#[cfg(test)]
mod test {
    use super::DepGraph;

    #[test]
    fn add_dep() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("npm", "node");
        graph.register_dep("yarn", "node");
        graph.register_dep("i", "you");
        graph.register_dep("genshin", "money");
        assert!(graph.walk().is_ok());
    }

    #[test]
    fn add_deps() {
        let mut graph = DepGraph::<String>::new();
        graph.register_deps(
            "vc_bundle",
            vec!["vs2005", "vs2008", "vs2010", "vs2013", "vs2019"],
        );
        graph.register_deps(
            "games",
            vec!["minecraft", "war3", "csgo", "genshin", "pokemon"],
        );
        assert!(graph.walk().is_ok());
    }

    #[test]
    fn cyclic_dependencies() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("app_a", "app_b");
        graph.register_dep("app_b", "app_c");
        graph.register_dep("app_c", "app_a");
        assert!(graph.walk().is_err());
    }
}
