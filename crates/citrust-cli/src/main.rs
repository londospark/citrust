use clap::Parser;
use std::path::PathBuf;
use std::process;

use citrust_core::keydb::KeyDatabase;

#[derive(Parser)]
#[command(name = "citrust", about = "3DS ROM decryption tool")]
struct Cli {
    /// Path to the .3ds ROM file
    rom: PathBuf,

    /// Path to aes_keys.txt key file
    #[arg(long = "keys", value_name = "PATH")]
    keys: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();

    println!("{}", cli.rom.display());

    let keydb = if let Some(ref keys_path) = cli.keys {
        match KeyDatabase::from_file(keys_path) {
            Ok(db) => {
                println!(
                    "Loaded key file: {} ({} keys)",
                    keys_path.display(),
                    db.len()
                );
                db
            }
            Err(e) => {
                eprintln!("Error loading key file: {e}");
                process::exit(1);
            }
        }
    } else if let Some(found_path) = KeyDatabase::search_default_locations() {
        match KeyDatabase::from_file(&found_path) {
            Ok(db) => {
                println!(
                    "Found key file: {} ({} keys)",
                    found_path.display(),
                    db.len()
                );
                db
            }
            Err(e) => {
                eprintln!(
                    "Warning: found key file at {} but failed to parse: {e}",
                    found_path.display()
                );
                process::exit(1);
            }
        }
    } else {
        eprintln!(
            "Error: No key file found. citrust requires an aes_keys.txt file for decryption."
        );
        eprintln!();
        eprintln!("Place your aes_keys.txt in one of these locations:");
        eprintln!("  - ./aes_keys.txt (next to your ROM)");
        eprintln!("  - ~/.config/citrust/aes_keys.txt (Linux)");
        eprintln!("  - %APPDATA%\\citrust\\aes_keys.txt (Windows)");
        eprintln!("  - Or specify with: citrust --keys /path/to/aes_keys.txt");
        eprintln!();
        eprintln!("See README.md for key file setup instructions.");
        process::exit(1);
    };

    if let Err(e) = citrust_core::decrypt::decrypt_rom(&cli.rom, &keydb, |msg| {
        println!("{msg}");
    }) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
