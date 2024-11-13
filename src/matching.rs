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

use std::{collections::{HashMap, HashSet}, rc::Rc, iter::zip};

use petgraph::graph::{DiGraph, NodeIndex};

use crate::{configuration::Participant, permutation::Assignment};

struct FlowNetwork<NodeDataType, EdgeDataType> {
    graph: DiGraph<NodeDataType, EdgeDataType>,
    source: NodeIndex,
    sink: NodeIndex,
}

fn construct_flow_network(
    participants: &HashSet<Rc<Participant>>,
    cannot_send_to: &HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
    cannot_receive_from: &HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
) -> FlowNetwork<String, usize> {
    // maps a person to the index of their sending and receiving node
    let mut node_owners: HashMap<Rc<Participant>, (NodeIndex, NodeIndex)> = HashMap::new();
    let mut flow_graph = DiGraph::<String, usize>::new();

    let source = flow_graph.add_node("flow_source".to_string());
    let sink = flow_graph.add_node("flow_sink".to_string());

    for p in participants {
        let p_s = flow_graph.add_node(format!("{}_send", p.name));
        let p_r = flow_graph.add_node(format!("{}_receive", p.name));
        flow_graph.add_edge(source, p_s, 1);
        flow_graph.add_edge(p_r, sink, 1);
        node_owners.insert(p.clone(), (p_s, p_r));
    }
    
    for sender in participants {
        for receiver in participants {
            if sender == receiver { continue; }
            if cannot_send_to[receiver].contains(sender) { continue; }
            if cannot_receive_from[sender].contains(receiver) { continue; }
            flow_graph.add_edge(node_owners[sender].0, node_owners[receiver].1, 1);
        }
    }

    FlowNetwork {
        graph: flow_graph,
        source,
        sink,
    }
}

fn get_matchings(participants: &HashSet<Rc<Participant>>, flow_network: FlowNetwork<String, usize>) -> Option<HashSet<Assignment<Participant>>> {
    let (flow, edge_capacities) = petgraph::algo::ford_fulkerson(&flow_network.graph, flow_network.source, flow_network.sink);

    // If the flow is not equal to the number of participants, then that means
    // there is at least one participant who is not receiving a gift (a matching is impossible)
    if flow != participants.len() {
        return None;
    }

    let mut assignments = HashSet::new();

    for (edge_capacity, edge) in zip(edge_capacities.iter(), flow_network.graph.raw_edges().iter()) {
        if *edge_capacity == 0 { continue; }
        let sender_name = flow_network.graph[edge.source()].clone().split_once("_").unwrap().0.to_string();
        let receiver_name = flow_network.graph[edge.target()].clone().split_once("_").unwrap().0.to_string();
        if sender_name.contains("flow") || receiver_name.contains("flow") { continue; }
        // TODO: see if there's a universe where we can switch from using names to using Rc<Participant> directly
        // so we don't have to do this lookup
        let sender = participants.iter().find(|p| p.name == sender_name).unwrap();
        let receiver = participants.iter().find(|p| p.name == receiver_name).unwrap();
        assignments.insert(Assignment {
            sender: sender.clone(),
            recipient: receiver.clone(),
        });
    }

    Some(assignments)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_participants() -> (Rc<Participant>, Rc<Participant>, Rc<Participant>) {
        let p1 = Rc::new(Participant {
            id: 1,
            name: "Alice".to_string(),
            discord_handle: "alice#1234".to_string(),
            mailing_info: "1234 Alice Lane".to_string(),
            interests: "Programming, cats".to_string(),
        });
        let p2 = Rc::new(Participant {
            id: 2,
            name: "Bob".to_string(),
            discord_handle: "bob#5678".to_string(),
            mailing_info: "5678 Bob Lane".to_string(),
            interests: "Programming, dogs".to_string(),
        });
        let p3 = Rc::new(Participant {
            id: 3,
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

        let participants = HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);

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

        let flow_network = construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let graph = flow_network.graph;

        // Each participant gets 1 sender node and 1 receiver node
        // +1 source node and +1 sink node makes 3*2 + 2 = 8 nodes
        assert_eq!(graph.node_count(), 8);

        let edges = HashSet::<(usize, usize, u8)>::from_iter(graph.raw_edges().iter().map(|edge| {
            (edge.source().index(), edge.target().index(), 1)
        }));

        // Included within this test is some implementation detail knowledge about the names of the nodes in the flow network.
        // This feels a little bad, so if there's a way to change this nicely, look into that.
        let source_node_index = graph.node_indices().find(|&node| graph[node] == "flow_source").unwrap().index();
        let sink_node_index = graph.node_indices().find(|&node| graph[node] == "flow_sink").unwrap().index();
        let p1_send_index = graph.node_indices().find(|&node| graph[node] == "Alice_send").unwrap().index();
        let p1_receive_index = graph.node_indices().find(|&node| graph[node] == "Alice_receive").unwrap().index();
        let p2_send_index = graph.node_indices().find(|&node| graph[node] == "Bob_send").unwrap().index();
        let p2_receive_index = graph.node_indices().find(|&node| graph[node] == "Bob_receive").unwrap().index();
        let p3_send_index = graph.node_indices().find(|&node| graph[node] == "Charlie_send").unwrap().index();
        let p3_receive_index = graph.node_indices().find(|&node| graph[node] == "Charlie_receive").unwrap().index();
        assert_eq!(edges, HashSet::from_iter(vec![
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
        ]));
    }

    #[test]
    fn test_get_matchings() {
        let (p1, p2, p3) = get_test_participants();

        let mut participants = HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);
        let p4 = Rc::new(Participant {
            id: 4,
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

        let flow_network = construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let assignments = get_matchings(&participants, flow_network).unwrap();

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

        let participants = HashSet::<Rc<Participant>>::from_iter(vec![p1.clone(), p2.clone(), p3.clone()]);

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

        let flow_network = construct_flow_network(&participants, &cannot_send_to, &cannot_receive_from);
        let assignments = get_matchings(&participants, flow_network);

        assert!(assignments.is_none());
    }
}