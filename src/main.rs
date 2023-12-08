use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::hash::{Hash, Hasher};
use std::iter::zip;
use std::rc::Rc;

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
    #[arg(short = 'v', long = "verbose", default_value = "false")]
    do_be_verbose: bool,
}

#[derive(Debug)]
struct Participant {
    // Assuming first name is unique because each person has a unique option in the Google Form
    // Will use this value like an ID for the participant
    name: String,
    discord_handle: String,
    mailing_info: String,
    interests: String,
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
                return Err(format!(
                    "Invalid permutation: {:?} cannot send to {:?}",
                    assignment.sender.name, assignment.recipient.name
                ));
            }
            // Make sure nobody is getting a present from somebody they excluded (i.e. the sender_id is not in the excluded_sender_id of the participant with ID recipient_id)
            if self.cannot_receive_from[&assignment.sender].contains(&assignment.recipient) {
                return Err(format!(
                    "Invalid permutation: {:?} cannot receive from {:?}",
                    assignment.sender.name, assignment.recipient.name
                ));
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
    fn try_new(
        assignments: HashSet<Assignment>,
        participants: &HashSet<Rc<Participant>>,
    ) -> Result<Permutation, String> {
        // Smart constructor to check it is actually a permutation

        // Make sure we have 1 assignment per participant
        if assignments.len() != participants.len() {
            return Err(format!("Invalid permutation: number of assignments ({}) does not match number of participants ({})", assignments.len(), participants.len()));
        }

        let all_senders: HashSet<_> = assignments
            .iter()
            .map(|assignment| Rc::clone(&assignment.sender))
            .collect();
        let all_recipients: HashSet<_> = assignments
            .iter()
            .map(|assignment| Rc::clone(&assignment.recipient))
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

    fn ensure_is_derangement(&self) -> Result<(), String> {
        // Test the permutation to see if it is a derangement of the participants
        // A derangement is a permutation of elements in a set in which no element appears in it's original position

        // i.e., make sure no sender has themselves as a recipient
        for assignment in self.assignments.iter() {
            if assignment.sender == assignment.recipient {
                return Err(format!(
                    "Invalid permutation: {:?} is sending to themselves",
                    assignment.sender.name
                ));
            }
        }

        Ok(())
    }
}

#[derive(Debug, serde::Deserialize)]
struct FormSubmission {
    #[serde(rename = "Timestamp")]
    timestamp: String,
    #[serde(rename = "Who are you?")]
    name: String,
    #[serde(rename = "Your Discord Handle")]
    discord_handle: String,
    #[serde(
        rename = "Sender Exclusions",
        deserialize_with = "deserialize_vec_string"
    )]
    cannot_send_to_submitter: Vec<String>,
    #[serde(
        rename = "Recipient Exclusions",
        deserialize_with = "deserialize_vec_string"
    )]
    cannot_receive_from_submitter: Vec<String>,
    #[serde(rename = "Your Mailing Info")]
    mailing_info: String,
    #[serde(rename = "Interests")]
    interests: String,
    #[serde(rename = "Anything Else?")]
    anything_else: String,
}

fn deserialize_vec_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let buf: String = String::deserialize(deserializer)?;
    Ok(buf.split(", ").map(|s| s.to_string()).collect())
}

fn participant_from_submission(submission: &FormSubmission) -> Participant {
    Participant {
        name: submission.name.clone(),
        discord_handle: submission.discord_handle.clone(),
        mailing_info: submission.mailing_info.clone(),
        interests: submission.interests.clone(),
    }
}

fn read_configuration_from_csv(file_path: &str) -> Configuration {
    // Read the CSV file at the given path and return the Configuration (participants and exclusion constraints)

    fn read_submissions(file_path: &str) -> Result<Vec<FormSubmission>, csv::Error> {
        let mut csv_reader = csv::Reader::from_path(file_path)?;
        let submissions = csv_reader
            .deserialize()
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        Ok(submissions)
    }
    let submissions = read_submissions(file_path).unwrap();

    type ParticipantName = String;

    let participant_map: HashMap<ParticipantName, Rc<Participant>> = submissions
        .iter()
        .map(|submission| {
            (
                submission.name.clone(),
                Rc::new(participant_from_submission(submission)),
            )
        })
        .collect();
    let cannot_send_to: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = submissions
        .iter()
        .map(|submission| {
            (
                participant_map[&submission.name].clone(),
                submission
                    .cannot_send_to_submitter
                    .iter()
                    .filter_map(|name| participant_map.get(name))
                    .map(|p| p.clone())
                    .collect(),
            )
        })
        .collect();
        
    let cannot_receive_from: HashMap<Rc<Participant>, HashSet<Rc<Participant>>> = submissions
        .iter()
        .map(|submission| {
            (
                participant_map[&submission.name].clone(),
                submission
                    .cannot_receive_from_submitter
                    .iter()
                    .filter_map(|name| participant_map.get(name))
                    .map(|p| p.clone())
                    .collect(),
            )
        })
        .collect();

    let participants: HashSet<Rc<Participant>> = participant_map
        .values()
        .map(|participant| Rc::clone(participant))
        .collect();

    Configuration {
        participants: participants,
        cannot_send_to: cannot_send_to,
        cannot_receive_from: cannot_receive_from,
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
            let mut participants: Vec<&Rc<Participant>> =
                Vec::from_iter(configuration.participants.iter());
            participants.shuffle(rng);
            participants
        };
        let random_assignments = zip(configuration.participants.iter(), participants_randomized)
            .map(|(p1, p2)| Assignment {
                sender: Rc::clone(p1),
                recipient: Rc::clone(p2),
            })
            .collect();

        let permutation = Permutation::try_new(random_assignments, &configuration.participants)?;
        configuration.ensure_valid_permutation(&permutation)?;
        Ok(permutation)
    }

    let mut loop_count: u128 = 0;

    loop {
        loop_count += 1;
        if do_be_verbose {
            eprintln!("Trying permutation #{}:", loop_count)
        };

        match gen_iter(&mut rng, &configuration) {
            Err(message) => {
                if do_be_verbose {
                    eprintln!("{}", message)
                }
            }
            Ok(permutation) => return permutation,
        }
    }
}

fn write_matching_files(permutation: Permutation, output_directory: &str) -> String {
    // Create matchings directory if necessary
    if let Err(_) = fs::create_dir(output_directory) {
        eprintln!(
            "Failed to create output directory {}, assuming it already exists.",
            output_directory
        );
    }

    // Create subfolder with timestamp
    let output_directory = format!(
        "{}/{}",
        output_directory,
        chrono::Local::now().format("%Y-%m-%d_%H-%M-%S")
    );
    if let Err(_) = fs::create_dir(output_directory.clone()) {
        eprintln!(
            "Failed to create output directory {}, assuming it already exists.",
            output_directory
        );
    }

    for assignment in permutation.assignments.iter() {
        let sender = &assignment.sender;
        let recipient = &assignment.recipient;

        let padding_disclaimer =
            "SCROLL DOWN TO SEE WHO YOU GOT\nTHIS IS TO HIDE IT FROM THE DISCORD EMBED\n"
                .to_string();
        let vertical_padding = &"|\n".repeat(25);
        let information = &format!(
            "You are the Secret Santa for {}! ({})\n\nAddress:\n{}\n\nTheir interests are:\n{}",
            recipient.name, recipient.discord_handle, recipient.mailing_info, recipient.interests
        );
        let closing = &"\n\n\n\nRemember to check the Google Form for information about suggested price range and gift 'due date'! Happy gifting!".to_string();

        fs::write(
            format!("{}/{}.txt", output_directory, sender.name),
            format!(
                "{}",
                padding_disclaimer + vertical_padding + information + closing
            ),
        )
        .unwrap();
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
