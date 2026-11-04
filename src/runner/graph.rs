// === Task graph

/* An edge points from a dependency to the task that depends on it, so a
 * topological order of the whole graph already runs dependencies before
 * dependents; `execution_order` below only has to restrict that order to the
 * tasks one target actually needs. */

use std::collections::{HashMap, HashSet};

use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};

use super::error::RunnerError;
use super::task::Task;

#[derive(Debug)]
pub struct TaskGraph {
    graph: DiGraph<Task, ()>,
    index_of: HashMap<String, NodeIndex>,
}

impl TaskGraph {
    /// Builds the graph, validating every `depends_on` reference and refusing a cycle.
    pub fn build(tasks: Vec<Task>, config_path: &std::path::Path) -> Result<Self, RunnerError> {
        let mut graph = DiGraph::new();
        let mut index_of = HashMap::new();

        for task in &tasks {
            let index = graph.add_node(task.clone());
            index_of.insert(task.name.clone(), index);
        }

        for task in &tasks {
            let dependent = index_of[&task.name];
            for depends_on in &task.depends_on {
                let dependency =
                    index_of
                        .get(depends_on)
                        .ok_or_else(|| RunnerError::UnknownDependency {
                            task: task.name.clone(),
                            depends_on: depends_on.clone(),
                            path: config_path.to_owned(),
                        })?;
                graph.add_edge(*dependency, dependent, ());
            }
        }

        if let Some(cycle) = find_cycle(&graph) {
            return Err(RunnerError::Cycle { cycle });
        }

        Ok(TaskGraph { graph, index_of })
    }

    /// The task named, and every dependency it transitively needs, in an order
    /// that always runs a dependency before whatever depends on it.
    pub fn execution_order(
        &self,
        task: &str,
        config_path: &std::path::Path,
    ) -> Result<Vec<&Task>, RunnerError> {
        self.execution_order_of(&[task], config_path)
    }

    /// Several targets at once, such as `build` in every workspace project:
    /// the union of their closures, each task once, in one valid order.
    pub fn execution_order_of(
        &self,
        targets: &[&str],
        config_path: &std::path::Path,
    ) -> Result<Vec<&Task>, RunnerError> {
        let mut closure = HashSet::new();
        for task in targets {
            let target = self
                .index_of
                .get(*task)
                .ok_or_else(|| RunnerError::UnknownTask {
                    task: (*task).to_owned(),
                    path: config_path.to_owned(),
                })?;
            closure.extend(ancestors_of(&self.graph, *target));
        }
        Ok(self.ordered(&closure))
    }

    /// Every task in the graph, in a valid dependency order. Used by `culm
    /// graph`, which shows the whole workspace rather than one task's
    /// closure.
    pub fn all_in_order(&self) -> Vec<&Task> {
        let every_node: HashSet<NodeIndex> = self.graph.node_indices().collect();
        self.ordered(&every_node)
    }

    /// `--affected`, r0.3: every `seed` task (one whose declared inputs a
    /// changed file matched), each seed's own dependency closure (still
    /// needed to build it, cache or not), and every task that transitively
    /// depends on a seed (what "downstream" means, per
    /// docs/DESIGN.md section 3.5). Unknown seed names are refused the same
    /// way `execution_order` refuses an unknown task, rather than being
    /// silently dropped from the run.
    pub fn affected_order(
        &self,
        seeds: &[&str],
        config_path: &std::path::Path,
    ) -> Result<Vec<&Task>, RunnerError> {
        let mut closure = HashSet::new();
        for seed in seeds {
            let index = self
                .index_of
                .get(*seed)
                .ok_or_else(|| RunnerError::UnknownTask {
                    task: (*seed).to_owned(),
                    path: config_path.to_owned(),
                })?;
            closure.extend(ancestors_of(&self.graph, *index));
            closure.extend(descendants_of(&self.graph, *index));
        }
        /* A persistent task never exits, so pulling one in would turn a check
         * into a run that only Ctrl+C ends. It stays reachable by name. */
        closure.retain(|index| !self.graph[*index].persistent);
        Ok(self.ordered(&closure))
    }

    /// A valid topological order restricted to `closure`. The whole graph is
    /// already known to be acyclic (`build` checked it), so `toposort`
    /// always succeeds; restricting that order to a subset of its nodes is
    /// still a valid topological order for the subgraph they induce.
    fn ordered(&self, closure: &HashSet<NodeIndex>) -> Vec<&Task> {
        let order = toposort(&self.graph, None).expect("cycles are rejected in build");
        order
            .into_iter()
            .filter(|index| closure.contains(index))
            .map(|index| &self.graph[index])
            .collect()
    }

    /// `culm graph --format dot`: the whole graph as Graphviz DOT text, one
    /// node per task labelled by name, one edge per `depends_on` pointing
    /// from the dependency to its dependent, matching this graph's own edge
    /// direction.
    pub fn to_dot(&self) -> String {
        let labelled = self.graph.map(|_, task| task.name.clone(), |_, _| "");
        format!("{}", petgraph::dot::Dot::new(&labelled))
    }
}

/// Every node reachable by walking dependency edges backward from `start`, `start` included.
fn ancestors_of(graph: &DiGraph<Task, ()>, start: NodeIndex) -> HashSet<NodeIndex> {
    let mut seen = HashSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if seen.insert(node) {
            for dependency in graph.neighbors_directed(node, Direction::Incoming) {
                stack.push(dependency);
            }
        }
    }
    seen
}

/// Every node reachable by walking dependency edges forward from `start`,
/// `start` included: everything that (transitively) depends on it, which is
/// what `--affected` means by "downstream".
fn descendants_of(graph: &DiGraph<Task, ()>, start: NodeIndex) -> HashSet<NodeIndex> {
    let mut seen = HashSet::new();
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        if seen.insert(node) {
            for dependent in graph.neighbors_directed(node, Direction::Outgoing) {
                stack.push(dependent);
            }
        }
    }
    seen
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

/// Depth-first search naming every task on a cycle, rather than reporting only that one exists.
fn find_cycle(graph: &DiGraph<Task, ()>) -> Option<Vec<String>> {
    let mut color = vec![Color::White; graph.node_count()];
    let mut stack = Vec::new();

    for start in graph.node_indices() {
        if color[start.index()] == Color::White
            && let Some(cycle) = visit(graph, start, &mut color, &mut stack)
        {
            return Some(cycle);
        }
    }
    None
}

fn visit(
    graph: &DiGraph<Task, ()>,
    node: NodeIndex,
    color: &mut [Color],
    stack: &mut Vec<NodeIndex>,
) -> Option<Vec<String>> {
    color[node.index()] = Color::Gray;
    stack.push(node);

    for next in graph.neighbors(node) {
        match color[next.index()] {
            Color::White => {
                if let Some(cycle) = visit(graph, next, color, stack) {
                    return Some(cycle);
                }
            }
            Color::Gray => {
                let start = stack
                    .iter()
                    .position(|&n| n == next)
                    .expect("marked gray, so on the stack");
                let mut names: Vec<String> = stack[start..]
                    .iter()
                    .map(|&n| graph[n].name.clone())
                    .collect();
                names.push(graph[next].name.clone());
                return Some(names);
            }
            Color::Black => {}
        }
    }

    stack.pop();
    color[node.index()] = Color::Black;
    None
}

// === Tests

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn task(name: &str, depends_on: &[&str]) -> Task {
        Task {
            name: name.to_owned(),
            project: "demo".to_owned(),
            dir: std::path::PathBuf::from("."),
            command: format!("echo {name}"),
            inputs: Vec::new(),
            outputs: Vec::new(),
            depends_on: depends_on.iter().map(|s| (*s).to_owned()).collect(),
            cacheable: false,
            declared_env: Vec::new(),
            persistent: false,
        }
    }

    fn path() -> std::path::PathBuf {
        Path::new("culm.toml").to_owned()
    }

    #[test]
    fn a_task_with_no_dependencies_runs_alone() {
        let graph = TaskGraph::build(vec![task("build", &[])], &path()).unwrap();
        let order = graph.execution_order("build", &path()).unwrap();
        assert_eq!(
            order.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            vec!["build"]
        );
    }

    #[test]
    fn dependencies_run_before_dependents() {
        let graph =
            TaskGraph::build(vec![task("build", &[]), task("test", &["build"])], &path()).unwrap();
        let order = graph.execution_order("test", &path()).unwrap();
        assert_eq!(
            order.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            vec!["build", "test"]
        );
    }

    /* A task unrelated to the one requested must not run, even though it is
     * in the same `culm.toml`: `culm run <task>` runs that task's closure,
     * not every task the file happens to define. */
    #[test]
    fn an_unrelated_task_is_excluded_from_the_order() {
        let graph = TaskGraph::build(
            vec![
                task("build", &[]),
                task("test", &["build"]),
                task("lint", &[]),
            ],
            &path(),
        )
        .unwrap();
        let order = graph.execution_order("test", &path()).unwrap();
        assert_eq!(order.len(), 2);
        assert!(!order.iter().any(|t| t.name == "lint"));
    }

    #[test]
    fn a_diamond_runs_each_dependency_once_before_the_task_that_needs_both() {
        let graph = TaskGraph::build(
            vec![
                task("base", &[]),
                task("left", &["base"]),
                task("right", &["base"]),
                task("top", &["left", "right"]),
            ],
            &path(),
        )
        .unwrap();
        let order = graph.execution_order("top", &path()).unwrap();
        let names: Vec<&str> = order.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names.len(), 4);
        assert_eq!(names.last(), Some(&"top"));
        assert!(
            names.iter().position(|n| *n == "base").unwrap()
                < names.iter().position(|n| *n == "left").unwrap()
        );
        assert!(
            names.iter().position(|n| *n == "base").unwrap()
                < names.iter().position(|n| *n == "right").unwrap()
        );
    }

    #[test]
    fn an_unknown_dependency_is_refused_by_name() {
        let error = TaskGraph::build(vec![task("test", &["build"])], &path()).unwrap_err();
        assert!(matches!(error, RunnerError::UnknownDependency { .. }));
        assert!(error.to_string().contains("build"));
    }

    #[test]
    fn running_an_undefined_task_is_refused() {
        let graph = TaskGraph::build(vec![task("build", &[])], &path()).unwrap();
        let error = graph.execution_order("missing", &path()).unwrap_err();
        assert!(matches!(error, RunnerError::UnknownTask { .. }));
    }

    /* The message must name every task on the cycle, not just say one exists. */
    #[test]
    fn a_direct_cycle_is_reported_by_name() {
        let error =
            TaskGraph::build(vec![task("a", &["b"]), task("b", &["a"])], &path()).unwrap_err();
        match error {
            RunnerError::Cycle { cycle } => {
                assert!(cycle.contains(&"a".to_owned()));
                assert!(cycle.contains(&"b".to_owned()));
            }
            other => panic!("expected a cycle error, got {other}"),
        }
    }

    #[test]
    fn affected_includes_the_seed_its_deps_and_its_dependents() {
        let graph = TaskGraph::build(
            vec![
                task("build", &[]),
                task("test", &["build"]),
                task("deploy", &["test"]),
                task("lint", &[]),
            ],
            &path(),
        )
        .unwrap();

        let order = graph.affected_order(&["test"], &path()).unwrap();
        let names: Vec<&str> = order.iter().map(|t| t.name.as_str()).collect();

        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
        assert!(names.contains(&"deploy"));
        assert!(!names.contains(&"lint"));
        assert!(
            names.iter().position(|n| *n == "build").unwrap()
                < names.iter().position(|n| *n == "test").unwrap()
        );
        assert!(
            names.iter().position(|n| *n == "test").unwrap()
                < names.iter().position(|n| *n == "deploy").unwrap()
        );
    }

    /* The scaffolded `dev` has no inputs, so it is always a seed; without this
     * rule `--affected` would start a server that never exits. */
    #[test]
    fn affected_never_includes_a_persistent_task() {
        let dev = Task {
            persistent: true,
            ..task("dev", &["build"])
        };
        let graph = TaskGraph::build(
            vec![task("build", &[]), dev, task("test", &["build"])],
            &path(),
        )
        .unwrap();

        let order = graph.affected_order(&["build", "dev"], &path()).unwrap();
        let names: Vec<&str> = order.iter().map(|t| t.name.as_str()).collect();

        assert_eq!(names.len(), 2);
        assert!(names.contains(&"build"));
        assert!(names.contains(&"test"));
    }

    #[test]
    fn a_persistent_task_still_runs_when_named() {
        let dev = Task {
            persistent: true,
            ..task("dev", &[])
        };
        let graph = TaskGraph::build(vec![dev], &path()).unwrap();
        let order = graph.execution_order("dev", &path()).unwrap();
        assert_eq!(order.len(), 1);
    }

    #[test]
    fn an_unknown_affected_seed_is_refused() {
        let graph = TaskGraph::build(vec![task("build", &[])], &path()).unwrap();
        let error = graph.affected_order(&["missing"], &path()).unwrap_err();
        assert!(matches!(error, RunnerError::UnknownTask { .. }));
    }

    #[test]
    fn all_in_order_includes_every_task_in_the_graph() {
        let graph = TaskGraph::build(vec![task("build", &[]), task("lint", &[])], &path()).unwrap();
        let names: Vec<&str> = graph
            .all_in_order()
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"build"));
        assert!(names.contains(&"lint"));
    }

    #[test]
    fn to_dot_names_every_task_and_every_dependency_edge() {
        let graph =
            TaskGraph::build(vec![task("build", &[]), task("test", &["build"])], &path()).unwrap();
        let dot = graph.to_dot();
        assert!(dot.contains("build"));
        assert!(dot.contains("test"));
        assert!(dot.contains("->"));
    }

    #[test]
    fn a_self_dependency_is_a_cycle() {
        let error = TaskGraph::build(vec![task("a", &["a"])], &path()).unwrap_err();
        assert!(matches!(error, RunnerError::Cycle { .. }));
    }

    #[test]
    fn a_longer_cycle_is_reported_by_name() {
        let error = TaskGraph::build(
            vec![task("a", &["c"]), task("b", &["a"]), task("c", &["b"])],
            &path(),
        )
        .unwrap_err();
        match error {
            RunnerError::Cycle { cycle } => {
                for name in ["a", "b", "c"] {
                    assert!(
                        cycle.contains(&name.to_owned()),
                        "{cycle:?} should name {name}"
                    );
                }
            }
            other => panic!("expected a cycle error, got {other}"),
        }
    }
}
