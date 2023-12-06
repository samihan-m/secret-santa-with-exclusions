use std::collections::{HashSet, HashMap};
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use rand::{thread_rng, Rng};
use std::fs;

#[derive(Debug)]
struct Participant {
    // Assuming first name is unique because each person has a unique option in the Google Form
    // Will use this value like an ID for the participant
    name: String,
    discord_handle: String,
    full_name_and_address: String,
    interests: String
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
struct Configuration {
    participants: HashSet<Rc<Participant>>,
    cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
    cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>>,
}

#[derive(Hash, PartialEq, Eq)]
struct Assignment {
    sender: Rc<Participant>,
    recipient: Rc<Participant>,
}

struct Permutation {
    assignments: HashSet<Assignment>,
}

impl Permutation {
    fn try_new(assignments: HashSet<Assignment>, participants: &HashSet<Rc<Participant>>) -> Result<Permutation, String> {
        // Smart constructor to check it is actually a permutation

        // Make sure we have 1 assignment per participant
        if assignments.len() != participants.len() {
            return Err(format!("Invalid permutation: number of assignments ({}) does not match number of participants ({})", assignments.len(), participants.len()));
        }

        let mut all_senders: HashSet<Rc<Participant>> = HashSet::new();
        let mut all_recipients: HashSet<Rc<Participant>> = HashSet::new();

        for assignment in assignments.iter() {
            all_senders.insert(Rc::clone(&assignment.sender));
            all_recipients.insert(Rc::clone(&assignment.recipient));
        }

        // Make sure every participant appears as a sender once and as a recipient once
        if all_senders.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique sender IDs ({}) does not match number of participants ({})", all_senders.len(), participants.len()));
        }
        if all_recipients.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique recipient IDs ({}) does not match number of participants ({})", all_recipients.len(), participants.len()));
        }

        Ok(Permutation { assignments })
    }
    
    fn new(assignments: HashSet<Assignment>, participants: &HashSet<Rc<Participant>>) -> Permutation {
        Permutation::try_new(assignments, participants).unwrap()
    }

    fn is_derangement(&self) -> bool {
        // Test the permutation to see if it is a derangement of the participants
        // A derangement is a permutation of elements in a set in which no element appears in it's original position

        // Make sure no sender has themselves as a recipient
        self.assignments.iter().all(|assignment|
            assignment.sender != assignment.recipient
        )
    }

    fn satisfies_exclusion_constraints(&self, configuration: &Configuration) -> bool {
        // Test the permutation to see if it satisfies the exclusion constraints of the participants

        for assignment in self.assignments.iter() {
            // Make sure nobody is sending a present to somebody they excluded (i.e. the recipient_id is not in the excluded_recipient_id of the participant with ID sender_id)
            if configuration.cannot_send_to[&assignment.recipient].contains(&assignment.sender) {
                println!("Invalid configuration: {:?} cannot send to {:?}", assignment.sender.name, assignment.recipient.name);
                return false;
            }
            // Make sure nobody is getting a present from somebody they excluded (i.e. the sender_id is not in the excluded_sender_id of the participant with ID recipient_id)
            if configuration.cannot_receive_from[&assignment.sender].contains(&assignment.recipient) {
                println!("Invalid configuration: {:?} cannot receive from {:?}", assignment.sender.name, assignment.recipient.name);
                return false;
            }
        }

        println!("Valid configuration.");

        true

    }

}

fn load_configuration_from_csv(file_path: &str) -> Configuration {
    // Read the CSV file at the given path and return the Configuration (participants and exclusion constraints)

    let mut participants_by_name: HashMap<String, Rc<Participant>> = HashMap::new();
    let mut cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = HashMap::new();
    let mut cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = HashMap::new();

    let mut csv_reader = csv::Reader::from_path(file_path).unwrap();

    // Iterate over the records once, creating Participant objects
    for result in csv_reader.records() {
        let record = result.unwrap();

        // Structure of the CSV file:
        //Timestamp, First Name, Discord Handle, Sender Exclusions, Recipient Exclusions, Full Name + Address, Interests, Anything else
        let name = record[1].to_string();
        let discord_handle = record[2].to_string();
        let full_name_and_address = record[5].to_string();
        let interests = record[6].to_string();

        let participant = Participant {
            name: String::from(&name),
            discord_handle,
            full_name_and_address,
            interests,
        };

        participants_by_name.insert(name, Rc::new(participant));
    }

    let mut csv_reader = csv::Reader::from_path(file_path).unwrap();

    // Iterate over the records again, adding entries to the exclusion maps
    for result in csv_reader.records() {
        let record = result.unwrap();

        let sender_exclusion_entry = record[3].to_string();
        let recipient_exclusion_entry = record[4].to_string();

        let sender_exclusion_name_list: Vec<&str> = sender_exclusion_entry.split(",").map(|s| s.trim()).collect();
        let recipient_exclusion_name_list: Vec<&str> = recipient_exclusion_entry.split(",").map(|s| s.trim()).collect();

        let mut sender_exclusion_reference_list: Vec<Rc<Participant>> = Vec::new();
        let mut recipient_exclusion_reference_list: Vec<Rc<Participant>> = Vec::new();

        for sender_exclusion_name in sender_exclusion_name_list {
            if sender_exclusion_name == "" {
                continue;
            }

            match participants_by_name.get(sender_exclusion_name) {
                Some(participant) => sender_exclusion_reference_list.push(participant.clone()),
                // There might not be a participant with this name (as not everybody might have signed up)
                None => println!("Warning: participant with name {} not found", sender_exclusion_name)
            }
        }

        for recipient_exclusion_name in recipient_exclusion_name_list {
            if recipient_exclusion_name == "" {
                continue;
            }

            match participants_by_name.get(recipient_exclusion_name) {
                Some(participant) => recipient_exclusion_reference_list.push(participant.clone()),
                // There might not be a participant with this name (as not everybody might have signed up)
                None => println!("Warning: participant with name {} not found", recipient_exclusion_name)
            }
        }

        let participant_name = record[1].to_string();
        let participant = &participants_by_name[&participant_name];
        cannot_send_to.insert(Rc::clone(participant), HashSet::from_iter(sender_exclusion_reference_list));
        cannot_receive_from.insert(Rc::clone(participant), HashSet::from_iter(recipient_exclusion_reference_list));
    }

    let participants: HashSet<Rc<Participant>> = HashSet::from_iter(participants_by_name.values().map(|participant| Rc::clone(participant)));

    Configuration {
        participants,
        cannot_send_to,
        cannot_receive_from,
    }

}

fn generate_valid_permutation(configuration: Configuration) -> Permutation {
    // Repeatedly try different derangements until we find one that satisfies the exclusion constraints

    // We have an n x n matrix (where n is the number of participants)
    // A possible permutation is a matrix that has exactly one 1 in each row and each column
    // A derangement is a permutation where there are no 1s on the diagonal

    // Generate random permutation matrices and test them until we find one that is 1. a derangement and 2. satisfies exclusion constraints

    let mut rng = thread_rng();

    let mut possible_senders = Vec::from_iter(configuration.participants.iter());
    let mut possible_recipients = Vec::from_iter(configuration.participants.iter());

    let mut assignments: HashSet<Assignment> = HashSet::new();

    let valid_permutation: Permutation;
    let mut loop_count: u128 = 0;

    loop {
        loop_count += 1;
        println!("Trying permutation #{}:", loop_count);
        while possible_senders.len() > 0 {
            let sender_index = rng.gen_range(0..possible_senders.len());
            let sender = possible_senders.swap_remove(sender_index);
            
            let recipient_index = rng.gen_range(0..possible_recipients.len());
            let recipient = possible_recipients.swap_remove(recipient_index);
    
            let assignment = Assignment {
                sender: Rc::clone(sender),
                recipient: Rc::clone(recipient),
            };
            
            assignments.insert(assignment);
        }
    
        let permutation_attempt = Permutation::try_new(assignments, &configuration.participants);
        assignments = HashSet::new();

        match permutation_attempt {
            Ok(permutation) => {
                let is_derangement = permutation.is_derangement();
                let satisfies_exclusion_constraints = permutation.satisfies_exclusion_constraints(&configuration);

                if is_derangement && satisfies_exclusion_constraints {
                    valid_permutation = permutation;
                    break;
                }

                // Try again
                if !is_derangement {
                    println!("Invalid permutation: not a derangement");
                }
                if !satisfies_exclusion_constraints {
                    println!("Invalid permutation: does not satisfy exclusion constraints");
                }
                possible_senders = Vec::from_iter(configuration.participants.iter());
                possible_recipients = Vec::from_iter(configuration.participants.iter());
            },
            Err(_) => {
                // Try again
                possible_senders = Vec::from_iter(configuration.participants.iter());
                possible_recipients = Vec::from_iter(configuration.participants.iter());
            }
        }
    }

    valid_permutation
}

fn write_matching_files(permutation: Permutation, output_directory: &str) -> String {

    // Create matchings directory if necessary
    if let Err(_) = fs::create_dir(output_directory) {
        println!("Error creating output directory {}, assuming it already exists.", output_directory);
    }

    // Create subfolder with timestamp
    let output_directory = format!("{}/{}", output_directory, chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    if let Err(_) = fs::create_dir(output_directory.clone()) {
        println!("Error creating output directory {}, assuming it already exists.", output_directory);
    }

    for assignment in permutation.assignments.iter() {
        
        let sender = &assignment.sender;
        let recipient = &assignment.recipient;

        let information = format!("You are the Secret Santa for {}! ({})\n\nAddress:\n{}\n\nTheir interests are:\n{}", recipient.name, recipient.discord_handle, recipient.full_name_and_address, recipient.interests);

        fs::write(format!("{}/{}.txt", output_directory, sender.name), information).unwrap();
    }

    return output_directory;
}

fn main() {
    let start_time = std::time::Instant::now();

    println!("Loading configuration...");
    let configuration = load_configuration_from_csv("./input_data.csv");

    println!("Loaded participants:");
    for participant in configuration.participants.iter() {
        println!("{:?}", participant.name);
    }

    println!("Generating valid permutation...");
    let permutation = generate_valid_permutation(configuration);

    println!("Writing matching files...");
    let output_directory = write_matching_files(permutation, "./matchings");
    println!("Done! Wrote matchings to {}.", output_directory);

    let duration = start_time.elapsed();
    println!("Time elapsed: {:?}", duration);
}
