use rayon::prelude::*;
use std::collections::{hash_map::Entry, HashMap};

fn char_count(x: &str) -> HashMap<char, usize> {
    let chars: Vec<char> = x.chars().collect();
    // check for dupes
    let mut counts: HashMap<char, usize> = HashMap::new();
    for c in chars.iter() {
      if let Entry::Occupied(mut count) = counts.entry(*c) {
        *count.get_mut() += 1;
      } else {
        counts.insert(*c, 1);
      }
    }
    counts
}

fn has_dupes(x: &str) -> bool {
    let char_count = char_count(x);
    char_count.values().any(|x| *x == 2)
}

fn only_increases(x: &str) -> bool {
    let mut last: u32 = x.chars().take(1).collect::<String>().parse().unwrap();
    for i in x.chars().flat_map(|s| char::to_digit(s, 10)) {
        if i < last {
            return false;
        }
        last = i;
    }
    true
}

fn is_password(x: &str) -> Option<u32> {
    if has_dupes(x) && only_increases(x) && x.len() == 6 {
        x.parse().ok()
    } else {
        None
    }
}

fn main() {
    let mut args = std::env::args();
    args.next().expect("seriously?");
    let start: u32 = args
        .next()
        .expect("No range start provided")
        .parse()
        .expect("Start wasn't a number");
    let end: u32 = args
        .next()
        .expect("No range end provided")
        .parse()
        .expect("End wasn't a number");

    let passwords: Vec<u32> = (start..=end)
        .map(|s| s.to_string())
        .collect::<Vec<String>>()
        .par_iter()
        .filter_map(|s| is_password(&s))
        .collect();

    println!("# passwords: {}", passwords.len());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dupe_check() {
        assert!(has_dupes("113456"));
        assert!(has_dupes("122456"));
        assert!(has_dupes("123446"));
        assert!(has_dupes("123455"));
        assert!(!has_dupes("123456"));
    }

    #[test]
    fn test_only_increases() {
        assert!(only_increases("123456"));
        assert!(only_increases("123444"));
        assert!(only_increases("111111"));
        assert!(!only_increases("654321"));
    }

    #[test]
    fn test_char_count() {
      let result = char_count("111144");
      assert_eq!(*result.get(&'1').unwrap(), 4);
      assert_eq!(*result.get(&'4').unwrap(), 2);
    }
}
