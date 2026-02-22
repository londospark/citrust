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
                Some(db)
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
                Some(db)
            }
            Err(e) => {
                eprintln!(
                    "Warning: found key file at {} but failed to parse: {e}",
                    found_path.display()
                );
                None
            }
        }
    } else {
        None
    };

    if let Err(e) = citrust_core::decrypt::decrypt_rom(&cli.rom, keydb.as_ref(), |msg| {
        println!("{msg}");
    }) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
