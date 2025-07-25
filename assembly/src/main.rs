use crate::assembly::load_assembly;
use std::{thread::sleep, time::Duration};

mod assembly;
mod compile;
mod datatypes;
mod memory;
mod simulator;

use clap::Parser;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    /// Path to the assembly file
    #[arg(short, long)]
    assembly: String,

    /// Path to output the memory file
    #[arg(short, long)]
    output: Option<String>,

    /// Don't print the memory layout
    #[arg(short, long)]
    quiet: bool,

    /// Simulate running the program
    #[arg(short, long)]
    simulate: bool,
}

fn main() {
    let args = Args::parse();
    // std::env::set_var("RUST_BACKTRACE", "1");

    let source = std::fs::read_to_string(args.assembly).unwrap();
    let assembly = load_assembly(&source);
    let memory = assembly.compile();

    if !args.quiet {
        memory.pprint();
    }

    if let Some(output) = args.output {
        let file = std::fs::File::create(output).unwrap();
        serde_json::to_writer(file, &memory.to_json()).unwrap();
    }

    if args.simulate {
        let mut sim = memory.simulator();
        sim.subscribe_to_output(Box::new(|addr, value| {
            println!("{addr:?} {value:?}");
        }));
        let input = sim.input();
        std::thread::spawn(move || {
            sleep(Duration::from_millis(100));
            input.lock().unwrap().push(8);
            sleep(Duration::from_millis(100));
            input.lock().unwrap().push(5);
        });

        println!("===Execute===");
        println!("{:?}", sim.run(true, true));
    }
}
