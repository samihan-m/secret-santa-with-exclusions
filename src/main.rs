use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::iter::zip;
use std::rc::Rc;

use clap::{Parser, ValueEnum};

mod configuration;
mod matching;
mod permutation;

use crate::configuration::{Configuration, Participant};
use crate::permutation::{Assignment, Permutation};

#[derive(Clone, Debug, ValueEnum)]
enum MatchingMethod {
    Permutation,
    FlowNetwork,
}

#[derive(Parser, Debug)]
struct Args {
    /// Input file path
    #[arg(short, long, default_value = "./input_data.csv")]
    input_file_path: String,

    /// Output directory path
    #[arg(short, long, default_value = "./matchings")]
    output_directory_path: String,

    /// Matching method. "flow-network" is recommended, as it will terminate if a valid assignment cannot be found, unlike "permutation".
    #[arg(short, long, value_enum, default_value_t = MatchingMethod::Permutation)]
    matching_method: MatchingMethod,

    /// Verbose flag. Has no effect when using the flow-network matching method.
    #[arg(short = 'v', long = "verbose", default_value = "false")]
    do_be_verbose: bool,
}

#[derive(Debug, serde::Deserialize)]
struct FormSubmission {
    #[serde(rename = "Timestamp")]
    _timestamp: String,
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
    _anything_else: String,
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
        let submissions = csv_reader.deserialize().collect::<Result<Vec<_>, _>>()?;
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
                    .cloned()
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
                    .cloned()
                    .collect(),
            )
        })
        .collect();

    let participants: HashSet<Rc<Participant>> = participant_map.values().map(Rc::clone).collect();

    Configuration {
        participants,
        cannot_send_to,
        cannot_receive_from,
    }
}

fn generate_valid_permutation(
    configuration: Configuration,
    do_be_verbose: bool,
) -> Permutation<Rc<Participant>> {
    // Repeatedly try different derangements until we find one that satisfies the exclusion constraints

    // We have an n x n matrix (where n is the number of participants)
    // A possible permutation is a matrix that has exactly one 1 in each row and each column
    // A derangement is a permutation where there are no 1s on the diagonal

    // Generate random permutation matrices and test them until we find one that is 1. a derangement and 2. satisfies exclusion constraints

    let mut rng = thread_rng();

    fn gen_iter(
        rng: &mut ThreadRng,
        configuration: &Configuration,
    ) -> Result<Permutation<Rc<Participant>>, String> {
        let participants_randomized = {
            let mut participants: Vec<&Rc<Participant>> =
                Vec::from_iter(configuration.participants.iter());
            participants.shuffle(rng);
            participants
        };
        let random_assignments = zip(configuration.participants.iter(), participants_randomized)
            .map(|(p1, p2)| Assignment {
                sender: p1.clone(),
                recipient: p2.clone(),
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

fn try_generate_assignments_via_flow_network(
    configuration: Configuration,
) -> Result<HashSet<Assignment<Rc<Participant>>>, String> {
    let flow_network = matching::construct_flow_network(
        &configuration.participants,
        &configuration.cannot_send_to,
        &configuration.cannot_receive_from,
    );

    matching::get_matchings(&configuration.participants, flow_network).map_err(
        |problematic_nodes| {
            format!(
                "Failed to find a valid assignment: {}",
                problematic_nodes
                    .into_iter()
                    .filter_map(|p| {
                        match p {
                            matching::NodeLabel::Sender(p) => {
                                Some(format!("{} is unable to send to anyone", p.name))
                            }
                            matching::NodeLabel::Receiver(p) => {
                                Some(format!("{} is unable to receive from anyone", p.name))
                            }
                            _ => None,
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    )
}

fn write_matching_files(
    assignments: HashSet<Assignment<Rc<Participant>>>,
    output_directory: &str,
) -> String {
    // Create matchings directory if necessary
    if fs::create_dir(output_directory).is_err() {
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
    if fs::create_dir(output_directory.clone()).is_err() {
        eprintln!(
            "Failed to create output directory {}, assuming it already exists.",
            output_directory
        );
    }

    for assignment in assignments {
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
            padding_disclaimer + vertical_padding + information + closing,
        )
        .unwrap();
    }

    output_directory
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

    let assignments = match arguments.matching_method {
        MatchingMethod::Permutation => {
            eprintln!("Generating valid permutation...");
            generate_valid_permutation(configuration, arguments.do_be_verbose).assignments
        }
        MatchingMethod::FlowNetwork => {
            eprintln!("Generating assignments via flow network...");
            match try_generate_assignments_via_flow_network(configuration) {
                Ok(assignments) => assignments,
                Err(message) => {
                    eprintln!("{}", message);
                    eprintln!("Exiting...");
                    std::process::exit(1);
                }
            }
        }
    };

    eprintln!("Writing matching files...");
    let output_directory = write_matching_files(assignments, &arguments.output_directory_path);
    eprintln!("Done! Wrote matchings to {}.", output_directory);

    let duration = start_time.elapsed();
    eprintln!("Time elapsed: {:?}", duration);
}
