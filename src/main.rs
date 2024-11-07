use rand::rngs::ThreadRng;
use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::iter::zip;
use std::rc::Rc;

use clap::Parser;

mod configuration;
mod permutation;
mod matching;

use crate::configuration::{Configuration, Participant};
use crate::permutation::{Permutation, Assignment};

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

fn participant_from_submission(submission: &FormSubmission, id: usize) -> Participant {
    Participant {
        id,
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
        .enumerate()
        .map(|(id, submission)| {
            (
                submission.name.clone(),
                Rc::new(participant_from_submission(submission, id)),
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

fn generate_valid_permutation(
    configuration: Configuration,
    do_be_verbose: bool,
) -> Permutation<Participant> {
    // Repeatedly try different derangements until we find one that satisfies the exclusion constraints

    // We have an n x n matrix (where n is the number of participants)
    // A possible permutation is a matrix that has exactly one 1 in each row and each column
    // A derangement is a permutation where there are no 1s on the diagonal

    // Generate random permutation matrices and test them until we find one that is 1. a derangement and 2. satisfies exclusion constraints

    let mut rng = thread_rng();

    fn gen_iter(
        rng: &mut ThreadRng,
        configuration: &Configuration,
    ) -> Result<Permutation<Participant>, String> {
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

fn write_matching_files(permutation: Permutation<Participant>, output_directory: &str) -> String {
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
