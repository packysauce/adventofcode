use std::env::args;
use std::fs;
use std::io::{BufRead, BufReader, Cursor};

fn fuel_per_mass(x: i64) -> i64 {
    (x / 3) - 2
}

fn total_fuel_per_mass(x: i64) -> i64 {
    let mut total = 0;
    let mut mass = fuel_per_mass(x);
    while mass > 0 {
        total += mass;
        mass = fuel_per_mass(mass);
    }
    total
}

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() != 2 {
        eprintln!("I expect exactly one argument - a file");
        std::process::exit(1);
    }
    let total: i64 = fs::read(&args[1])
        .map(Cursor::new)
        .map(BufReader::new)
        .expect("Unable to read the file")
        .lines()
        .filter_map(std::result::Result::ok)
        .filter_map(|line| line.parse::<i64>().ok())
        .map(total_fuel_per_mass)
        .sum();

    println!("{}", total);
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_samples() {
        assert_eq!(fuel_per_mass(14), 2);
    }

    #[test]
    fn test_totals_samples() {
        assert_eq!(total_fuel_per_mass(14), 2);
        assert_eq!(total_fuel_per_mass(1969), 966);
    }
}
