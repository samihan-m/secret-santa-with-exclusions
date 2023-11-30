struct Participant {
    name: String,
    discord_handle: String,
    address: String,
    interests: String,
    sender_exclusion_list: Vec<String>,
    recipient_exclusion_list: Vec<String>
}

struct Assignment {
    sender: Participant,
    recipient: Participant
}

fn read_csv(file_path: &str) -> Vec<Participant> {
    panic!("Not yet implemented!");
}

fn generate_configuration(participant_list: Vec<Participant>) -> Vec<Assignment> {
    panic!("Not yet implemented!");
}

fn main() {
    println!("Hello, world!");
}
