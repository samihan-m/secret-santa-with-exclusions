# Secret Santa Matching (with exclusion lists)

A program that takes in a `.csv` file of responses to a Secret Santa sign-up form (see template form here: [Template Secret Santa Sign-up Form](https://docs.google.com/forms/d/e/1FAIpQLSf2PSR-NRT5nQ0umhFVMbniDJZd-322R1rpxYmhYIu-PTs_Tw/viewform?usp=sf_link)) and outputs one text file per participant with the name, address (if applicable), and interests of the person for which they are the Secret Santa.

Big thanks to [Jason](https://github.com/chezbgone) for co-writing this with me.

## Usage

Create a sign-up form like the template linked above and export the responses as a `.csv` file. The program assumes this file is named `input_data.csv` but you can provide a different name with the `-i` flag:

Build: `cargo build --release`

Navigate to the `target/release` directory and run the executable:

Run: `secret_santa.exe [options]`

```
Options:
  -i, --input-file-path <INPUT_FILE_PATH>              Input file path [default: ./input_data.csv]
  -o, --output-directory-path <OUTPUT_DIRECTORY_PATH>  Output directory path [default: ./matchings]
  -m, --matching-method <MATCHING_METHOD>              Matching method. "flow-network" is recommended, as it will terminate if a valid assignment cannot be found, unlike "permutation" [default: permutation] [possible values: permutation, flow-network]
  -v, --verbose                                        Verbose flag. Has no effect when using the flow-network matching method
  -h, --help                                           Print help
```

Run `secret_santa.exe --help` to see the same information as above.

A directory named `<output-directory-path>` (by default `matchings`) will be created in the root directory, and within that will be a subfolder named with the immediate timestamp. Within that subfolder will be one text file per participant. Send each participant the `.txt` file with their name on it.

Happy gifting!
