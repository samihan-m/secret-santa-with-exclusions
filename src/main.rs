use std::collections::{HashSet, HashMap};
use std::iter::zip;
use std::hash::{Hash, Hasher};
use std::rc::Rc;
use rand::rngs::ThreadRng;
use rand::thread_rng;
use rand::seq::SliceRandom;
use std::fs;

use clap::Parser;

#[derive(Parser, Debug)]
struct Args {
    /// Input file path
    #[arg(short, long, default_value = "./input_data.csv")]
    input_file_path: String,

    /// Output directory path
    #[arg(short, long, default_value = "./matchings")]
    output_directory_path: String,

    /// Verbose flag
    #[arg(short = 'v', long, default_value = "false")]
    do_be_verbose: bool,
}

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

impl Configuration {

    fn ensure_exclusions_satisfied(&self, permutation: &Permutation) -> Result<(), String> {
        // Test the permutation to see if it satisfies the exclusion constraints of the participants

        for assignment in permutation.assignments.iter() {
            // Make sure nobody is sending a present to somebody they excluded (i.e. tt_id is not in the excluded_recipient_id of the participant with ID sender_id)
            if self.cannot_send_to[&assignment.recipient].contains(&assignment.sender) {
                return Err(format!("Invalid permutation: {:?} cannot send to {:?}", assignment.sender.name, assignment.recipient.name));
            }
            // Make sure nobody is getting a present from somebody they excluded (i.e. the sender_id is not in the excluded_sender_id of the participant with ID recipient_id)
            if self.cannot_receive_from[&assignment.sender].contains(&assignment.recipient) {
                return Err(format!("Invalid permutation: {:?} cannot receive from {:?}", assignment.sender.name, assignment.recipient.name));
            }
        }

        Ok(())
    }
    
    fn ensure_valid_permutation(&self, permutation: &Permutation) -> Result<(), String> {
        // Test the permutation to see if it is a valid permutation of the participants
        // i.e. it is a derangement and it satisfies the exclusion constraints
        permutation.ensure_is_derangement()?;
        self.ensure_exclusions_satisfied(permutation)?;
        Ok(())
    }
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

        let all_senders: HashSet<_> = assignments.iter().map(|assignment| Rc::clone(&assignment.sender)).collect();
        let all_recipients: HashSet<_> = assignments.iter().map(|assignment| Rc::clone(&assignment.recipient)).collect();

        // Make sure every participant appears as a sender once and as a recipient once
        if all_senders.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique sender IDs ({}) does not match number of participants ({})", all_senders.len(), participants.len()));
        }
        if all_recipients.len() != participants.len() {
            return Err(format!("Invalid permutation: number of unique recipient IDs ({}) does not match number of participants ({})", all_recipients.len(), participants.len()));
        }

        Ok(Permutation { assignments })
    }

    fn ensure_is_derangement(&self) -> Result<(), String> {
        // Test the permutation to see if it is a derangement of the participants
        // A derangement is a permutation of elements in a set in which no element appears in it's original position

        // i.e., make sure no sender has themselves as a recipient
        for assignment in self.assignments.iter() {
            if assignment.sender == assignment.recipient {
                return Err(format!("Invalid permutation: {:?} is sending to themselves", assignment.sender.name));
            }
        }
        
        Ok(())
    }

}

fn read_configuration_from_csv(file_path: &str) -> Configuration {
    // Read the CSV file at the given path and return the Configuration (participants and exclusion constraints)

    let mut participants_by_name: HashMap<String, Rc<Participant>> = HashMap::new();

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
    let mut cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = HashMap::new();
    let mut cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = HashMap::new();

    // Iterate over the records again, adding entries to the exclusion maps
    for result in csv_reader.records() {
        let record = result.unwrap();

        let sender_exclusion_entry = record[3].to_string();
        let recipient_exclusion_entry = record[4].to_string();

        let sender_exclusion_names = sender_exclusion_entry
            .split(",")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        let recipient_exclusion_names = recipient_exclusion_entry
            .split(",")
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        
        let mut sender_exclusion_reference_list: Vec<Rc<Participant>> = Vec::new();
        let mut recipient_exclusion_reference_list: Vec<Rc<Participant>> = Vec::new();

        for sender_exclusion_name in sender_exclusion_names {
            match participants_by_name.get(sender_exclusion_name) {
                Some(participant) => sender_exclusion_reference_list.push(participant.clone()),
                // There might not be a participant with this name (as not everybody might have signed up)
                None => eprintln!("Warning: participant with name {} not found", sender_exclusion_name)
            }
        }

        for recipient_exclusion_name in recipient_exclusion_names {
            match participants_by_name.get(recipient_exclusion_name) {
                Some(participant) => recipient_exclusion_reference_list.push(participant.clone()),
                // There might not be a participant with this name (as not everybody might have signed up)
                None => eprintln!("Warning: participant with name {} not found", recipient_exclusion_name)
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

fn generate_valid_permutation(configuration: Configuration, do_be_verbose: bool) -> Permutation {
    // Repeatedly try different derangements until we find one that satisfies the exclusion constraints

    // We have an n x n matrix (where n is the number of participants)
    // A possible permutation is a matrix that has exactly one 1 in each row and each column
    // A derangement is a permutation where there are no 1s on the diagonal

    // Generate random permutation matrices and test them until we find one that is 1. a derangement and 2. satisfies exclusion constraints

    let mut rng = thread_rng();
    
    fn gen_iter(rng: &mut ThreadRng, configuration: &Configuration) -> Result<Permutation, String> {
        let participants_randomized = {
            let mut participants: Vec<&Rc<Participant>> = Vec::from_iter(configuration.participants.iter());
            participants.shuffle(rng);
        
            participants
        };
        let random_assignments = zip(configuration.participants.iter(), participants_randomized).map(
            |(p1, p2)| Assignment {
                sender: Rc::clone(p1),
                recipient: Rc::clone(p2),
            }
        ).collect();

        let permutation = Permutation::try_new(random_assignments, &configuration.participants)?;
        configuration.ensure_valid_permutation(&permutation)?;
        Ok(permutation)
    }

    let mut loop_count: u128 = 0;

    loop {
        loop_count += 1;
        if do_be_verbose { eprintln!("Trying permutation #{}:", loop_count) };

        match gen_iter(&mut rng, &configuration) {
            Err(message) => {
                if do_be_verbose { eprintln!("{}", message) }
            },
            Ok(permutation) => return permutation,
        }
    }
}

fn write_matching_files(permutation: Permutation, output_directory: &str) -> String {

    // Create matchings directory if necessary
    if let Err(_) = fs::create_dir(output_directory) {
        eprintln!("Failed to create output directory {}, assuming it already exists.", output_directory);
    }

    // Create subfolder with timestamp
    let output_directory = format!("{}/{}", output_directory, chrono::Local::now().format("%Y-%m-%d_%H-%M-%S"));
    if let Err(_) = fs::create_dir(output_directory.clone()) {
        eprintln!("Failed to create output directory {}, assuming it already exists.", output_directory);
    }

    for assignment in permutation.assignments.iter() {
        
        let sender = &assignment.sender;
        let recipient = &assignment.recipient;

        let padding_disclaimer = "SCROLL DOWN TO SEE WHO YOU GOT\nTHIS IS TO HIDE IT FROM THE DISCORD EMBED\n".to_string();
        let vertical_padding = &"|\n".repeat(25);
        let information = &format!("You are the Secret Santa for {}! ({})\n\nAddress:\n{}\n\nTheir interests are:\n{}", recipient.name, recipient.discord_handle, recipient.full_name_and_address, recipient.interests);
        let closing = &"\n\n\n\nRemember to check the Google Form for information about suggested price range and gift 'due date'! Happy gifting!".to_string();

        fs::write(format!("{}/{}.txt", output_directory, sender.name), format!("{}", padding_disclaimer + vertical_padding + information + closing)).unwrap();
    }

    return output_directory;
}

fn main() {
    let arguments = Args::parse();

    let start_time = std::time::Instant::now();

    eprintln!("Loading configuration...");
    let configuration = read_configuration_from_csv(&arguments.input_file_path);

    eprintln!("Loaded participants:");
    for participant in configuration.participants.iter() {
        eprintln!("{:?}", participant.name);
    }

    eprintln!("Generating valid permutation...");
    let permutation = generate_valid_permutation(configuration, arguments.do_be_verbose);

    eprintln!("Writing matching files...");
    let output_directory = write_matching_files(permutation, &arguments.output_directory_path);
    eprintln!("Done! Wrote matchings to {}.", output_directory);

    let duration = start_time.elapsed();
    eprintln!("Time elapsed: {:?}", duration);
}
