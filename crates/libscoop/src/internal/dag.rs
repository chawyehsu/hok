#![allow(dead_code)]
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::error::Error;
use std::fmt::Display;
use std::hash::Hash;

#[derive(Debug, Clone)]
struct Dependencies<T>(HashSet<T>);
type Nodes<T> = HashMap<T, Dependencies<T>>;
/// The DAG (Direct Acyclic Graph) used to represent package dependencies.
#[derive(Debug)]
pub struct DepGraph<T: Hash + Eq + Clone + Display> {
    /// [`Nodes`] in the DepGraph.
    ///
    /// Each entry in this HashMap represents a node with its dependencies in
    /// the DepGraph.
    nodes: Nodes<T>,
}
/// Cyclic dependency error.
#[derive(Debug, Clone)]
pub struct CyclicError(String);
/// Wrapper of std Result with the [`CyclicError`] failure.
pub type Result<T> = std::result::Result<T, CyclicError>;

impl Error for CyclicError {}

impl Display for CyclicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "cyclic dependency: {}", self.0)
    }
}

impl CyclicError {
    fn from<T: Display>(nodes: &Nodes<T>) -> CyclicError {
        let mut s = vec![];
        nodes
            .iter()
            .for_each(|(k, v)| v.0.iter().for_each(|d| s.push(format!("{} -> {}", k, d))));
        let e = format!("[{}]", s.join(", "));
        CyclicError(e)
    }
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

impl<T> Dependencies<T> {
    #[inline]
    fn new() -> Dependencies<T> {
        Dependencies(HashSet::new())
    }
}

impl<T> DepGraph<T>
where
    T: Hash + Eq + Clone + Display,
{
    /// Create a new [`DepGraph`].
    #[inline]
    pub fn new() -> DepGraph<T> {
        DepGraph {
            nodes: HashMap::new(),
        }
    }

    /// Count of unsolved nodes in the [`DepGraph`].
    ///
    /// ## Returns
    ///
    /// The count of unsolved nodes.
    #[inline]
    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    /// Reinitialize the [`DepGraph`].
    #[inline]
    pub fn reset(&mut self) {
        self.nodes = HashMap::new();
    }

    /// Register a `node` in the [`DepGraph`], with no dependencies.
    #[inline]
    pub fn register_node<N: Into<T>>(&mut self, node: N) {
        Self::__register(&mut self.nodes, node.into(), None);
    }

    /// Register a `node` in the [`DepGraph`], with its dependency `dep_node`.
    ///
    /// The `dep_node` will also be registered in if it hasn't been.
    #[inline]
    pub fn register_dep<N: Into<T>>(&mut self, node: N, dep_node: N) {
        Self::__register(&mut self.nodes, node.into(), Some(dep_node.into()))
    }

    /// Register a `node` in the [`DepGraph`] with its dependencies `dep_nodes`.
    ///
    /// Each node from `dep_nodes` will also be registered in if it hasn't been.
    #[inline]
    pub fn register_deps<N: Into<T>>(&mut self, node: N, dep_nodes: Vec<N>) {
        let node = node.into();
        dep_nodes.into_iter().for_each(|dep_node| {
            Self::__register(&mut self.nodes, node.clone(), Some(dep_node.into()));
        })
    }

    /// Unregister a `node` from the [`DepGraph`].
    ///
    /// All paths, i.e. both dependents and dependencies connected to the node
    /// will also be removed.
    ///
    /// ## Returns
    ///
    /// - `true` when the node was found and removed.
    /// - `false` when the node was not in the graph.
    #[inline]
    pub fn unregister_node<N: Into<T>>(&mut self, node: N) -> bool {
        let node = node.into();
        let ret = self.nodes.contains_key(&node);
        if ret {
            Self::__unregister(&mut self.nodes, &node);
        }
        ret
    }

    /// Pop a node which does not have any dependency and can be resolved.
    /// `None` will be returned if all nodes have dependencies.
    ///
    /// If `None` is returned and graph size is not 0, there's cyclic dependency.
    pub fn pop(&mut self) -> Option<T> {
        let node = self
            .nodes
            .iter()
            .filter(|(_, deps)| deps.len() == 0)
            .map(|(node, _)| node)
            .next()
            .cloned();
        if let Some(node) = &node {
            Self::__unregister(&mut self.nodes, node);
        }
        node
    }

    /// Pop a vector of nodes which do not have any dependency and can be
    /// resolved. An empty vector will be returned if all nodes have dependencies.
    ///
    /// If an empty vector is returned and graph size is not 0, there's cyclic
    /// dependencies.
    #[inline]
    pub fn step(&mut self) -> Vec<T> {
        Self::__step(&mut self.nodes)
    }

    /// Check if cyclic dependency exist in the graph. This method does not
    /// remove any nodes from the graph, instead it manipulates a clone of the
    /// graph.
    pub fn check(&self) -> Result<()> {
        let mut nodes = self.nodes.clone();
        while !nodes.is_empty() {
            let step = Self::__step(&mut nodes);
            if step.is_empty() {
                return Err(CyclicError::from(&nodes));
            }
        }
        Ok(())
    }

    /// Walk the whole graph and pop all of the resolved nodes. An error will
    /// be returned when cyclic dependency is detected.
    pub fn walk(&mut self) -> Result<Vec<Vec<T>>> {
        let mut res = vec![];
        while !self.nodes.is_empty() {
            let step = self.step();
            if step.is_empty() {
                return Err(CyclicError::from(&self.nodes));
            }
            res.push(step);
        }
        Ok(res)
    }

    /// Walk the whole graph and pop all of the resolved nodes. Nodes will be
    /// flattened into a single vector. An error will be returned when cyclic
    /// dependencies is detected.
    #[inline]
    pub fn walk_flatten(&mut self) -> Result<Vec<T>> {
        self.walk()
            .map(|v| v.into_iter().flatten().collect::<Vec<_>>())
    }

    fn __step(nodes: &mut Nodes<T>) -> Vec<T> {
        let step = nodes
            .iter()
            .filter(|(_, deps)| deps.len() == 0)
            .map(|(node, _)| node.clone())
            .collect::<Vec<_>>();
        step.iter().for_each(|node| {
            Self::__unregister(nodes, node);
        });
        step
    }

    #[inline]
    fn __register(nodes: &mut Nodes<T>, node: T, dep_node: Option<T>) {
        // register node
        match nodes.entry(node) {
            Entry::Vacant(e) => {
                let mut dep = Dependencies::new();
                if let Some(dep_node) = &dep_node {
                    dep.insert(dep_node.clone());
                }
                e.insert(dep);
            }
            Entry::Occupied(e) => {
                if let Some(dep_node) = &dep_node {
                    e.into_mut().insert(dep_node.clone());
                }
            }
        }
        // register dep_node
        if let Some(dep_node) = dep_node {
            match nodes.entry(dep_node) {
                Entry::Vacant(e) => {
                    e.insert(Dependencies::new());
                }
                Entry::Occupied(_) => {
                    // no-op
                }
            }
        }
    }

    #[inline]
    fn __unregister(nodes: &mut Nodes<T>, node: &T) {
        nodes.remove(node);
        nodes.iter_mut().for_each(|(_, deps)| {
            deps.remove(node);
        });
    }
}

#[cfg(test)]
mod test {
    use super::DepGraph;

    #[test]
    fn test_register_node() {
        let mut graph = DepGraph::<String>::new();
        graph.register_node("loneliness");
        assert!(graph.walk().is_ok());
    }

    #[test]
    fn test_register_dep() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("npm", "node");
        graph.register_dep("yarn", "node");
        graph.register_dep("i", "you");
        graph.register_dep("earth", "sun");
        assert!(graph.walk().is_ok());
    }

    #[test]
    fn test_register_deps() {
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
    fn test_check() {
        let mut graph = DepGraph::<String>::new();
        graph.register_node("one");
        assert!(graph.check().is_ok());
        graph.register_dep("me", "you");
        graph.register_dep("you", "me");
        assert!(graph.check().is_err());
    }

    #[test]
    fn test_pop() {
        let mut graph = DepGraph::<String>::new();
        graph.register_node("are you ok?");
        let ret = graph.pop();
        assert_eq!(ret, Some("are you ok?".to_string()));
        assert_eq!(graph.size(), 0);
    }

    #[test]
    fn test_step() {
        let mut graph = DepGraph::<String>::new();
        graph.register_node("are you ok?");
        graph.register_node("what's your problem?");
        graph.register_dep("what's your problem?", "oh no!");
        let ret = graph.step();
        assert_eq!(ret.len(), 2);
        assert_eq!(graph.size(), 1);
    }

    #[test]
    fn test_unregister_node() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("npm", "node");
        graph.register_dep("yarn", "node");
        graph.unregister_node("node");
        graph.unregister_node("npm");
        graph.unregister_node("yarn");
        assert!(graph.size() == 0);
    }

    #[test]
    fn test_walk_flatten() {
        let mut graph = DepGraph::<String>::new();
        graph.register_node("firefox");
        graph.register_node("chromium");
        graph.register_dep("msedge", "chromium");
        let ret = graph.walk_flatten();
        assert!(ret.is_ok());
        assert_eq!(ret.unwrap().len(), 3);
    }

    #[test]
    fn test_cyclic() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("Death's End", "The Dark Forest");
        graph.register_dep("The Dark Forest", "The Three-Body Problem");
        graph.register_dep("The Three-Body Problem", "Death's End");
        assert!(graph.walk().is_err());
    }

    #[test]
    fn test_self_cyclic() {
        let mut graph = DepGraph::<String>::new();
        graph.register_dep("self", "self");
        assert!(graph.walk().is_err());
    }
}
