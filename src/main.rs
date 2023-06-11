use std::{
    collections::HashMap,
    fs::File,
    io::Read,
    num::NonZeroUsize,
    sync::{Arc, Mutex},
};

use clap::Parser;
use cli_table::{print_stdout, Cell, Table};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use snake_runtime::{SnakeRuntime, Winner};

#[derive(Parser, Debug)]
#[command(author, about, long_about = None)]
struct Args {
    /// WASM file for RED (team 0) player
    #[arg(short, long)]
    red: String,

    /// WASM file for BLUE (team 1) player
    #[arg(short, long)]
    blue: String,

    /// The starting seed (seed is incremented by 1 for each game)
    #[arg(short, long, default_value = "0")]
    seed: u32,

    /// Number of games to simulate
    #[arg(short, long, default_value = "100")]
    games: u32,

    /// Number of threads to use. Defaults to the number of cores.
    #[arg(short, long)]
    threads: Option<usize>,

    /// Print JSON statistics to stdout instead of the default human readable output
    #[arg(long)]
    json: bool,
}

struct State {
    seed: u32,
    wins: HashMap<Winner, u32>,
    lose_reasons: HashMap<Winner, HashMap<String, (u32, Vec<u32>)>>,
}

#[derive(Serialize, Deserialize)]
struct JsonOutput {
    red: u32,
    tie: u32,
    blue: u32,
}

pub fn main() {
    let args = Args::parse();

    let red = get_wasm_file_bytes(&args.red);
    let blue = get_wasm_file_bytes(&args.blue);

    let seed_mutex = Arc::new(Mutex::new(State {
        seed: args.seed,
        wins: HashMap::new(),
        lose_reasons: HashMap::new(),
    }));

    let num_threads = args.threads.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(NonZeroUsize::get)
            .unwrap_or(1)
    });

    if u32::checked_add(args.games, args.seed).is_none() {
        println!("Seed is too high for the number of games selected!");
    }

    if !args.json {
        println!("Running {} games with {} threads", args.games, num_threads);
    }

    let mut threads = vec![];

    for _ in 0..num_threads {
        let red = red.clone();
        let blue = blue.clone();
        let seed_mutex = seed_mutex.clone();

        threads.push(std::thread::spawn(move || {
            let mut runtime = SnakeRuntime::new(&red, &blue);

            loop {
                let seed = {
                    let mut state = seed_mutex.lock().unwrap();
                    let seed = state.seed;
                    if seed + 1 > (args.games + args.seed) {
                        return;
                    }
                    state.seed += 1;
                    seed
                };
                let result = runtime.run_game(seed);

                {
                    let mut state = seed_mutex.lock().unwrap();
                    *state.wins.entry(result.winner).or_insert(0) += 1;

                    let examples = state
                        .lose_reasons
                        .entry(result.winner)
                        .or_default()
                        .entry(result.lose_reason.clone())
                        .or_default();

                    if examples.1.len() < 5 {
                        examples.1.push(seed);
                    }
                    examples.0 += 1;
                }

                let winner_str = match result.winner {
                    Winner::Red => "RED".red(),
                    Winner::Blue => "BLUE".blue(),
                    Winner::Tie => "TIE".white(),
                };
                if !args.json {
                    println!(
                        "{:05} = {} ({}:{:05}) {}",
                        seed, winner_str, result.tick, result.cycle, result.lose_reason
                    );
                }
            }
        }));
    }

    for thread_handle in threads {
        thread_handle.join().unwrap();
    }

    let state = seed_mutex.lock().unwrap();

    let red_wins = state.wins.get(&Winner::Red).cloned().unwrap_or(0);
    let ties = state.wins.get(&Winner::Tie).cloned().unwrap_or(0);
    let blue_wins = state.wins.get(&Winner::Blue).cloned().unwrap_or(0);

    assert_eq!(red_wins + ties + blue_wins, args.games);

    if args.json {
        let json_output = JsonOutput {
            red: red_wins,
            tie: ties,
            blue: blue_wins,
        };
        println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
    } else {
        println!("\n===== RESULTS =====");
        println!("GAMES SIMULATED: {}", args.games);
        println!(
            "{} WINS: {} ({:.1}%)",
            "RED".red(),
            red_wins,
            (red_wins as f64 * 100.0) / (args.games as f64)
        );
        println!(
            "TIES: {} ({:.1}%)",
            ties,
            (ties as f64 * 100.0) / (args.games as f64)
        );
        println!(
            "{} WINS: {} ({:.1}%)",
            "BLUE".blue(),
            blue_wins,
            (blue_wins as f64 * 100.0) / (args.games as f64)
        );
        println!("\n");
        println!("{} lose reasons (why the last snake died)", "RED".red());
        print_lose_reasons_table(Winner::Blue, &state.lose_reasons);

        println!("\n");
        println!("{} lose reasons (why the last snake died)", "BLUE".blue());
        print_lose_reasons_table(Winner::Red, &state.lose_reasons);
    }
}

fn print_lose_reasons_table(
    winner: Winner,
    lose_reasons: &HashMap<Winner, HashMap<String, (u32, Vec<u32>)>>,
) {
    let mut lose_reasons: Vec<(String, (u32, Vec<u32>))> = lose_reasons
        .get(&winner)
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|(reason, examples)| (reason.clone(), examples.clone()))
        .collect();

    lose_reasons.sort_by_cached_key(|x| x.1 .0);
    lose_reasons.reverse();

    let mut table = vec![];

    for (reason, (count, mut examples)) in lose_reasons {
        examples.sort();
        let examples_string = examples
            .iter()
            .map(|num| num.to_string())
            .collect::<Vec<String>>()
            .join(", ");

        table.push(vec![reason.cell(), count.cell(), examples_string.cell()]);
    }

    print_stdout(table.table().title(vec![
        "Reason".cell(),
        "Count".cell(),
        "Seed Examples".cell(),
    ]))
    .unwrap();
}

fn get_wasm_file_bytes(filename: &str) -> Vec<u8> {
    let mut file = File::open(filename).unwrap();
    let mut output = vec![];
    file.read_to_end(&mut output).unwrap();
    output
}
