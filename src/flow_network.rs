/*
The algorithm:
Input: List of participants (with the appropriate information)
1. Turn the list of participants in to a complete digraph G
2. Remove edges from G corresponding to every listed exclusion
3. Create a bipartite graph H from G via this process:
Given G = (V,E), form the graph H whose vertex set is two copies of V (call them V_L and V_R) so that each vertex v has two copies (called v_L and v_R as well).
For each arc (u,v) in E, add an arc (u_L, v_R) to H. Now find a perfect matching in H. Observe that every perfect matching in H corresponds to a cycle cover in G.
4. Transform H into a flow network H'
5. Find a perfect matching on H' via Ford-Fulkerson
6. If no perfect matching exists, then a valid Secret Santa matching is impossible.
Find the problematic vertex (participant that was excluded by everybody) by either looking at H' and seeing which vertices don't have any edges connected to them or maybe just doing that on G.
but otherwise, the perfect matching corresponds to a cycle cover in G.
7. Transform the cycle cover into the Secret Santa assignments
*/

use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    iter::zip,
    rc::Rc,
};

use petgraph::{
    dot::Dot,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{configuration::Participant, permutation::Assignment, random_ford_fulkerson};

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum NodeLabel {
    Source,
    Sink,
    Sender(Rc<Participant>),
    Receiver(Rc<Participant>),
}

impl Display for NodeLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeLabel::Source => write!(f, "Source"),
            NodeLabel::Sink => write!(f, "Sink"),
            NodeLabel::Sender(participant) => write!(f, "{}", participant.name),
            NodeLabel::Receiver(participant) => write!(f, "{}", participant.name),
        }
    }
}

pub struct FlowNetwork<NodeDataType, EdgeDataType> {
    graph: DiGraph<NodeDataType, EdgeDataType>,
    source: NodeIndex,
    sink: NodeIndex,
}

pub fn construct_flow_network(
    participants: &HashSet<Rc<Participant>>,
    cannot_send_to: &HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
    cannot_receive_from: &HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
) -> FlowNetwork<NodeLabel, usize> {
    // maps a person to the index of their sending and receiving node
    let mut node_owners: HashMap<Rc<Participant>, (NodeIndex, NodeIndex)> = HashMap::new();
    let mut flow_graph = DiGraph::<NodeLabel, usize>::new();

    let source = flow_graph.add_node(NodeLabel::Source);
    let sink = flow_graph.add_node(NodeLabel::Sink);

    for p in participants {
        let p_s = flow_graph.add_node(NodeLabel::Sender(p.clone()));
        let p_r = flow_graph.add_node(NodeLabel::Receiver(p.clone()));
        flow_graph.add_edge(source, p_s, 1);
        flow_graph.add_edge(p_r, sink, 1);
        node_owners.insert(p.clone(), (p_s, p_r));
    }

    for sender in participants {
        for receiver in participants {
            if sender == receiver {
                continue;
            }
            if cannot_send_to[receiver].contains(sender) {
                continue;
            }
            if cannot_receive_from[sender].contains(receiver) {
                continue;
            }
            flow_graph.add_edge(node_owners[sender].0, node_owners[receiver].1, 1);
        }
    }

    FlowNetwork {
        graph: flow_graph,
        source,
        sink,
    }
}

pub fn get_matchings(
    participants: &HashSet<Rc<Participant>>,
    flow_network: FlowNetwork<NodeLabel, usize>,
    be_verbose: bool,
) -> Result<HashSet<Assignment<Rc<Participant>>>, HashSet<NodeLabel>> {
    let (flow, edge_capacities) =
        random_ford_fulkerson::ford_fulkerson(&flow_network.graph, flow_network.source, flow_network.sink);

    // If the flow is not equal to the number of participants, then that means
    // there is at least one participant who is not receiving a gift (a matching is impossible)
    if flow != participants.len() {
        // Accumulate a list of participants who do not have an edge of weight 1 to another participant
        let mut problematic_nodes = HashSet::new();

        for edge in flow_network.graph.edges(flow_network.source) {
            if edge_capacities[edge.id().index()] == 0 {
                problematic_nodes.insert(flow_network.graph[edge.target()].clone());
            }
        }

        for edge in flow_network.graph.edges(flow_network.sink) {
            if edge_capacities[edge.id().index()] == 0 {
                problematic_nodes.insert(flow_network.graph[edge.source()].clone());
            }
        }

        return Err(problematic_nodes);
    }

    let mut assignments = HashSet::new();

    if be_verbose {
        let edges_with_flow = zip(
            edge_capacities.iter(),
            flow_network.graph.raw_edges().iter(),
        )
        .filter(|(capacity, _)| **capacity > 0)
        .map(|(capacity, edge)| (edge.source(), edge.target(), *capacity));
        let mut post_network = DiGraph::<NodeLabel, usize>::new();
        for node in flow_network.graph.node_indices() {
            post_network.add_node(flow_network.graph[node].clone());
        }
        for (source, target, weight) in edges_with_flow {
            post_network.add_edge(source, target, weight);
        }

        eprintln!("Here is a graphviz .dot format representation for the flow network after finding matchings.");
        eprintln!("Copy-paste it into something like https://viz-js.com/ to visualize it:");
        eprintln!("{}", Dot::new(&post_network));
    }

    for (edge_capacity, edge) in zip(
        edge_capacities.iter(),
        flow_network.graph.raw_edges().iter(),
    ) {
        if *edge_capacity == 0 {
            continue;
        }
        let source = &flow_network.graph[edge.source()];
        let target = &flow_network.graph[edge.target()];
        if let (NodeLabel::Sender(sender), NodeLabel::Receiver(receiver)) = (source, target) {
            assignments.insert(Assignment {
                sender: sender.clone(),
                recipient: receiver.clone(),
            });
        }
    }

    Ok(assignments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_participants() -> (Rc<Participant>, Rc<Participant>, Rc<Participant>) {
        let p1 = Rc::new(Participant {
            name: "Alice".to_string(),
            discord_handle: "alice#1234".to_string(),
            mailing_info: "1234 Alice Lane".to_string(),
            interests: "Programming, cats".to_string(),
        });
        let p2 = Rc::new(Participant {
            name: "Bob".to_string(),
            discord_handle: "bob#5678".to_string(),
            mailing_info: "5678 Bob Lane".to_string(),
            interests: "Programming, dogs".to_string(),
        });
        let p3 = Rc::new(Participant {
            name: "Charlie".to_string(),
            discord_handle: "charlie#9101".to_string(),
            mailing_info: "9101 Charlie Lane".to_string(),
            interests: "Programming, birds".to_string(),
        });

        (p1, p2, p3)
    }

    #[test]
    fn test_construct_flow_network() {
        let (p1, p2, p3) = get_test_participants();

        let participants =
            HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);

        let mut cannot_send_to = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_send_to.insert(p1.clone(), {
            let mut set = HashSet::new();
            set.insert(p2.clone());
            set
        });
        cannot_send_to.insert(p2.clone(), HashSet::new());
        cannot_send_to.insert(p3.clone(), HashSet::new());

        let mut cannot_receive_from = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_receive_from.insert(p1.clone(), HashSet::new());
        cannot_receive_from.insert(p2.clone(), HashSet::new());
        cannot_receive_from.insert(p3.clone(), {
            let mut set = HashSet::new();
            set.insert(p2.clone());
            set
        });

        let flow_network =
            construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let graph = flow_network.graph;

        // Each participant gets 1 sender node and 1 receiver node
        // +1 source node and +1 sink node makes 3*2 + 2 = 8 nodes
        assert_eq!(graph.node_count(), 8);

        let edges = HashSet::<(usize, usize, u8)>::from_iter(
            graph
                .raw_edges()
                .iter()
                .map(|edge| (edge.source().index(), edge.target().index(), 1)),
        );

        // Included within this test is some implementation detail knowledge about the names of the nodes in the flow network.
        // This feels a little bad, so if there's a way to change this nicely, look into that.
        let source_node_index = flow_network.source.index();
        let sink_node_index = flow_network.sink.index();
        let p1_send_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Sender(p1.clone()))
            .unwrap()
            .index();
        let p1_receive_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Receiver(p1.clone()))
            .unwrap()
            .index();
        let p2_send_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Sender(p2.clone()))
            .unwrap()
            .index();
        let p2_receive_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Receiver(p2.clone()))
            .unwrap()
            .index();
        let p3_send_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Sender(p3.clone()))
            .unwrap()
            .index();
        let p3_receive_index = graph
            .node_indices()
            .find(|&node| graph[node] == NodeLabel::Receiver(p3.clone()))
            .unwrap()
            .index();
        assert_eq!(
            edges,
            HashSet::from_iter(vec![
                (source_node_index, p1_send_index, 1),
                (source_node_index, p2_send_index, 1),
                (source_node_index, p3_send_index, 1),
                (p1_receive_index, sink_node_index, 1),
                (p2_receive_index, sink_node_index, 1),
                (p3_receive_index, sink_node_index, 1),
                (p1_send_index, p2_receive_index, 1),
                (p1_send_index, p3_receive_index, 1),
                (p2_send_index, p3_receive_index, 1),
                (p3_send_index, p1_receive_index, 1),
            ])
        );
    }

    #[test]
    fn test_get_matchings() {
        let (p1, p2, p3) = get_test_participants();

        let mut participants =
            HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);
        let p4 = Rc::new(Participant {
            name: "David".to_string(),
            discord_handle: "david#1213".to_string(),
            mailing_info: "1213 David Lane".to_string(),
            interests: "Programming, fish".to_string(),
        });
        participants.insert(p4.clone());

        let mut cannot_send_to = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_send_to.insert(p1.clone(), {
            let mut set = HashSet::new();
            set.insert(p2.clone());
            set
        });
        cannot_send_to.insert(p2.clone(), HashSet::new());
        cannot_send_to.insert(p3.clone(), HashSet::new());
        cannot_send_to.insert(p4.clone(), HashSet::new());

        let mut cannot_receive_from = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_receive_from.insert(p1.clone(), HashSet::new());
        cannot_receive_from.insert(p2.clone(), HashSet::new());
        cannot_receive_from.insert(p3.clone(), {
            let mut set = HashSet::new();
            set.insert(p2.clone());
            set
        });
        cannot_receive_from.insert(p4.clone(), HashSet::new());

        let flow_network =
            construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let assignments = get_matchings(&participants, flow_network, false).unwrap();

        assert_eq!(assignments.len(), participants.len());

        for assignment in &assignments {
            assert_ne!(assignment.sender, assignment.recipient);
            assert!(!cannot_send_to[&assignment.recipient].contains(&assignment.sender));
            assert!(!cannot_receive_from[&assignment.sender].contains(&assignment.recipient));
        }
    }

    #[test]
    fn test_get_matchings_when_impossible() {
        let (p1, p2, p3) = get_test_participants();

        let participants =
            HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);

        let mut cannot_send_to = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_send_to.insert(p1.clone(), {
            let mut set = HashSet::new();
            set.insert(p2.clone());
            set.insert(p3.clone());
            set
        });
        cannot_send_to.insert(p2.clone(), HashSet::new());
        cannot_send_to.insert(p3.clone(), HashSet::new());

        let mut cannot_receive_from = HashMap::<Rc<Participant>, HashSet<Rc<Participant>>>::new();
        cannot_receive_from.insert(p1.clone(), HashSet::new());
        cannot_receive_from.insert(p2.clone(), HashSet::new());
        cannot_receive_from.insert(p3.clone(), HashSet::new());

        let flow_network =
            construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let problematic_nodes = get_matchings(&participants, flow_network, false).unwrap_err();

        assert!(problematic_nodes.len() == 1);
    }
}
