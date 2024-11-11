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

use std::{collections::{HashMap, HashSet}, rc::Rc};

use petgraph::graph::{NodeIndex, UnGraph};

use crate::configuration::Participant;

fn construct_flow_network(
    participants: HashSet<Rc<Participant>>,
    cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
    cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
) -> UnGraph<String, u8> {
    // maps a person to the index of their sending and receiving node
    let mut node_owners: HashMap<Rc<Participant>, (NodeIndex, NodeIndex)> = HashMap::new();
    let mut flow_graph = UnGraph::<String, u8>::new_undirected();

    let source = flow_graph.add_node("flow_source".to_string());
    let sink = flow_graph.add_node("flow_sink".to_string());

    for p in &participants {
        let p_s = flow_graph.add_node(format!("{}_send", p.name));
        let p_r = flow_graph.add_node(format!("{}_receive", p.name));
        flow_graph.add_edge(source, p_s, 1);
        flow_graph.add_edge(p_r, sink, 1);
        node_owners.insert(p.clone(), (p_s, p_r));
    }
    
    for sender in &participants {
        for receiver in &participants {
            if sender == receiver { continue; }
            if cannot_send_to[receiver].contains(sender) { continue; }
            if cannot_receive_from[sender].contains(receiver) { continue; }
            flow_graph.add_edge(node_owners[sender].0, node_owners[receiver].1, 1);
        }
    }

    flow_graph
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_construct_flow_network() {
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

        let flow_network = construct_flow_network(participants, cannot_send_to, cannot_receive_from);

        // Each participant gets 1 sender node and 1 receiver node
        // +1 source node and +1 sink node makes 3*2 + 2 = 8 nodes
        assert_eq!(flow_network.node_count(), 8);

        let edges = HashSet::<(usize, usize, u8)>::from_iter(flow_network.raw_edges().iter().map(|edge| {
            (edge.source().index(), edge.target().index(), 1)
        }));

        // Included within this test is some implementation detail knowledge about the names of the nodes in the flow network.
        // This feels a little bad, so if there's a way to change this nicely, look into that.
        let source_node_index = flow_network.node_indices().find(|&node| flow_network[node] == "flow_source").unwrap().index();
        let sink_node_index = flow_network.node_indices().find(|&node| flow_network[node] == "flow_sink").unwrap().index();
        let p1_send_index = flow_network.node_indices().find(|&node| flow_network[node] == "Alice_send").unwrap().index();
        let p1_receive_index = flow_network.node_indices().find(|&node| flow_network[node] == "Alice_receive").unwrap().index();
        let p2_send_index = flow_network.node_indices().find(|&node| flow_network[node] == "Bob_send").unwrap().index();
        let p2_receive_index = flow_network.node_indices().find(|&node| flow_network[node] == "Bob_receive").unwrap().index();
        let p3_send_index = flow_network.node_indices().find(|&node| flow_network[node] == "Charlie_send").unwrap().index();
        let p3_receive_index = flow_network.node_indices().find(|&node| flow_network[node] == "Charlie_receive").unwrap().index();
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

}