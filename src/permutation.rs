use std::collections::HashSet;
use std::hash::Hash;
use std::fmt::Debug;

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct Assignment<T> {
    pub sender: T,
    pub recipient: T,
}

pub struct Permutation<T> {
    pub assignments: HashSet<Assignment<T>>,
}

impl<T> Permutation<T>
where
    T: Eq + Hash + Clone
{
    pub fn try_new(
        assignments: HashSet<Assignment<T>>,
        participants: &HashSet<T>,
    ) -> Result<Permutation<T>, String> {
        // Smart constructor to check it is actually a permutation

        // Make sure we have 1 assignment per participant
        if assignments.len() != participants.len() {
            return Err(format!("Invalid permutation: number of assignments ({}) does not match number of participants ({})", assignments.len(), participants.len()));
        }

        let all_senders: HashSet<_> = assignments
            .iter()
            .map(|assignment| assignment.sender.clone())
            .collect();
        let all_recipients: HashSet<_> = assignments
            .iter()
            .map(|assignment| assignment.recipient.clone())
            .collect();

        // Make sure every participant appears as a sender once and as a recipient once
        if all_senders.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique sender IDs ({}) does not match number of participants ({})", all_senders.len(), participants.len()));
        }
        if all_recipients.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique recipient IDs ({}) does not match number of participants ({})", all_recipients.len(), participants.len()));
        }

        Ok(Permutation { assignments })
    }

    pub fn ensure_is_derangement(&self) -> Result<(), T> {
        // Test the permutation to see if it is a derangement of the participants
        // A derangement is a permutation of elements in a set in which no element appears in it's original position

        // i.e., make sure no sender has themselves as a recipient
        for assignment in self.assignments.iter() {
            if assignment.sender == assignment.recipient {
                return Err(assignment.sender.clone());
            }
        }

        Ok(())
    }
}
