use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use crate::permutation::Permutation;

#[derive(Debug)]
pub struct Participant {
    pub name: String,
    pub discord_handle: String,
    pub mailing_info: String,
    pub interests: String,
}

impl Display for Participant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.discord_handle)
    }
}

impl PartialEq for Participant {
    fn eq(&self, other: &Participant) -> bool {
        self.name == other.name
    }
}

impl Eq for Participant {}

impl Hash for Participant {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

#[derive(Debug)]
pub struct Configuration {
    pub participants: HashSet<Rc<Participant>>,
    pub cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
    pub cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
}

impl Configuration {
    pub fn ensure_exclusions_satisfied(
        &self,
        permutation: &Permutation<Rc<Participant>>,
    ) -> Result<(), String> {
        for assignment in permutation.assignments.iter() {
            // Make sure nobody is sending a present to somebody they excluded
            if self.cannot_send_to[&assignment.recipient].contains(&assignment.sender) {
                return Err(format!(
                    "Invalid permutation: {:?} cannot send to {:?}",
                    assignment.sender.name, assignment.recipient.name
                ));
            }
            // Make sure nobody is getting a present from somebody they excluded
            if self.cannot_receive_from[&assignment.sender].contains(&assignment.recipient) {
                return Err(format!(
                    "Invalid permutation: {:?} cannot receive from {:?}",
                    assignment.sender.name, assignment.recipient.name
                ));
            }
        }

        Ok(())
    }

    pub fn ensure_valid_permutation(
        &self,
        permutation: &Permutation<Rc<Participant>>,
    ) -> Result<(), String> {
        permutation
            .ensure_is_derangement()
            .map_err(|bad_sender| format!("Participant {} maps to themselves", bad_sender))?;
        self.ensure_exclusions_satisfied(permutation)?;
        Ok(())
    }
}
