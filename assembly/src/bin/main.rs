use assembly::{compile_assembly, layout_pages, load_assembly};
use clap::Parser;
use std::{thread::sleep, time::Duration};

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

    /// Optional comma-separated list of integers between -32768 and 65535
    #[arg(
        short,
        long,
        value_delimiter = ',',
        num_args = 1..,
        value_parser = clap::builder::ValueParser::new(parse_single_value)
    )]
    inputs: Option<Vec<u16>>,
}

fn parse_single_value(s: &str) -> Result<u16, String> {
    let value: i32 = s
        .parse()
        .map_err(|e| format!("Invalid integer '{s}': {e}"))?;
    if value < i16::MIN as i32 || value > u16::MAX as i32 {
        return Err(format!(
            "Value {value} is out of allowed range (-32768 to 65535)"
        ));
    }
    Ok((value as u32 & 0xFFFF) as u16)
}
fn main() {
    let args = Args::parse();
    // std::env::set_var("RUST_BACKTRACE", "1");

    let source = std::fs::read_to_string(args.assembly).unwrap();
    let assembly = load_assembly(&source).unwrap();
    let page_layout = layout_pages(&assembly).unwrap();
    let memory = compile_assembly(&page_layout).unwrap().memory().clone();

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
            if let Some(inputs) = args.inputs {
                for val in inputs {
                    sleep(Duration::from_millis(100));
                    input.lock().unwrap().push(val);
                }
            }
        });

        println!("{:?}", sim.run(true, true));
    } else if args.inputs.is_some() {
        panic!("Input sequence given for simulator but not running simulation");
    }
}
