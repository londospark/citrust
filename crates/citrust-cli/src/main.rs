use clap::Parser;
use std::path::PathBuf;
use std::process;

#[derive(Parser)]
#[command(name = "citrust", about = "3DS ROM decryption tool")]
struct Cli {
    /// Path to the .3ds ROM file
    rom: PathBuf,
}

fn main() {
    let cli = Cli::parse();

    println!("{}", cli.rom.display());

    if let Err(e) = citrust_core::decrypt::decrypt_rom(&cli.rom, |msg| {
        println!("{msg}");
    }) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
