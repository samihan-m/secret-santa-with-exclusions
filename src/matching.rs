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

use petgraph::graph::DiGraph;

use crate::configuration::Participant;

fn create_complete_digraph(participants: HashSet<Rc<Participant>>) -> DiGraph<usize, ()> {
    let participant_count = participants.len();
    let mut digraph = DiGraph::<usize, ()>::with_capacity(participant_count, participant_count * (participant_count - 1));

    let node_indices = participants.iter().map(|participant| digraph.add_node(participant.id)).collect::<Vec<_>>();

    node_indices.iter().for_each(|sender| {
        node_indices.iter().for_each(|recipient| {
            if sender != recipient {
                digraph.add_edge(*sender, *recipient, ());
            }
        });
    });

    digraph
}

fn remove_exclusion_edges(
    // A complete digraph
    mut digraph: DiGraph<usize, ()>,
    cannot_send_to: HashMap<usize, HashSet<usize>>,
    cannot_receive_from: HashMap<usize, HashSet<usize>>,
) -> DiGraph<usize, ()> {
    digraph.node_indices().for_each(|sender_node_index| {
        let sender_id = digraph[sender_node_index];
        digraph.node_indices().for_each(|recipient_node_index| {
            let recipient_id = digraph[recipient_node_index];
            if cannot_send_to[&sender_id].contains(&recipient_id) {
                digraph.remove_edge(digraph.find_edge(sender_node_index, recipient_node_index).unwrap());
            }
            if cannot_receive_from[&recipient_id].contains(&sender_id) {
                digraph.remove_edge(digraph.find_edge(sender_node_index, recipient_node_index).unwrap());
            }
        });
    });

    digraph
}

fn create_bipartite_graph(digraph: DiGraph<usize, ()>) -> DiGraph<usize, ()> {
    let mut bipartite_graph = DiGraph::<usize, ()>::with_capacity(digraph.node_count() * 2, digraph.edge_count());

    (0..=1).for_each(|_| {
        digraph.node_indices()
            .map(|node_index| digraph[node_index])
            .for_each(|participant_id| {
                bipartite_graph.add_node(participant_id);
            });
    });

    let node_count = digraph.node_count();
    let new_edges = digraph.raw_edges().iter().map(|edge| {
        let source = edge.source().index();
        let target = edge.target().index() + node_count;
        (source as u32, target as u32)
    });

    bipartite_graph.extend_with_edges(new_edges);

    bipartite_graph
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_participants() -> HashSet<Rc<Participant>> {
        let mut participants = HashSet::<Rc<Participant>>::new();
        
        participants.insert(Rc::new(Participant {
            id: 1,
            name: "Alice".to_string(),
            discord_handle: "alice#1234".to_string(),
            mailing_info: "1234 Alice Lane".to_string(),
            interests: "Programming, cats".to_string(),
        }));
        participants.insert(Rc::new(Participant {
            id: 2,
            name: "Bob".to_string(),
            discord_handle: "bob#5678".to_string(),
            mailing_info: "5678 Bob Lane".to_string(),
            interests: "Programming, dogs".to_string(),
        }));
        participants.insert(Rc::new(Participant {
            id: 3,
            name: "Charlie".to_string(),
            discord_handle: "charlie#9101".to_string(),
            mailing_info: "9101 Charlie Lane".to_string(),
            interests: "Programming, birds".to_string(),
        }));

        participants
    }

    #[test]
    fn test_create_complete_digraph() {
        let digraph = create_complete_digraph(get_test_participants());

        assert_eq!(digraph.node_count(), 3);
        assert_eq!(digraph.edge_count(), 6);
    }

    #[test]
    fn test_remove_no_exclusion_edges() {
        let mut cannot_send_to = HashMap::<usize, HashSet<usize>>::new();
        cannot_send_to.insert(1, HashSet::new());
        cannot_send_to.insert(2, HashSet::new());
        cannot_send_to.insert(3, HashSet::new());

        let mut cannot_receive_from = HashMap::<usize, HashSet<usize>>::new();
        cannot_receive_from.insert(1, HashSet::new());
        cannot_receive_from.insert(2, HashSet::new());
        cannot_receive_from.insert(3, HashSet::new());

        let mut digraph = create_complete_digraph(get_test_participants());
        digraph = remove_exclusion_edges(digraph, cannot_send_to, cannot_receive_from);

        assert_eq!(digraph.edge_count(), 6);
    }

    #[test]
    fn test_remove_exclusion_edges() {
        let mut cannot_send_to = HashMap::<usize, HashSet<usize>>::new();
        cannot_send_to.insert(1, {
            let mut set = HashSet::new();
            set.insert(2);
            set
        });
        cannot_send_to.insert(2, HashSet::new());
        cannot_send_to.insert(3, HashSet::new());

        let mut cannot_receive_from = HashMap::<usize, HashSet<usize>>::new();
        cannot_receive_from.insert(1, HashSet::new());
        cannot_receive_from.insert(2, HashSet::new());
        cannot_receive_from.insert(3, {
            let mut set = HashSet::new();
            set.insert(2);
            set
        });

        let mut digraph = create_complete_digraph(get_test_participants());
        digraph = remove_exclusion_edges(digraph, cannot_send_to, cannot_receive_from);

        assert_eq!(digraph.edge_count(), 4);
    }

    #[test]
    fn test_create_bipartite_graph() {
        let mut digraph = create_complete_digraph(get_test_participants());
        digraph = create_bipartite_graph(digraph);

        assert_eq!(digraph.node_count(), 6);
        assert_eq!(digraph.edge_count(), 6);

        let edges = HashSet::<(usize, usize)>::from_iter(digraph.raw_edges().iter().map(|edge| {
            (edge.source().index(), edge.target().index())
        }));
        
        assert_eq!(edges, HashSet::from_iter(vec![
            (0, 4), // L1 should point to R2,
            (0, 5), // L1 should point to R3
            (1, 3), // L2 should point to R1
            (1, 5), // L2 should point to R3
            (2, 3), // L3 should point to R1
            (2, 4), // L3 should point to R2
            // No other edges should exist
        ]));
    }
}