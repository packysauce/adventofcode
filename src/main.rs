use std::fs;
use std::env::args;
use std::io::{BufRead, BufReader, Cursor};

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() != 2 {
        eprintln!("I expect exactly one argument - a file");
        std::process::exit(1);
    }
    let total: u64 = fs::read(&args[1])
        .map(Cursor::new)
        .map(BufReader::new)
        .expect("Unable to read the file")
        .lines()
        .filter_map(|l| l.ok())
        .filter_map(|line| line.parse::<u64>().ok())
        .map(|mass| (mass / 3) - 2)
        .sum();
    
    println!("{}", total);
}
