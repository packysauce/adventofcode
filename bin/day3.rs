use geo::{Coordinate, Line, intersects::Intersects};
use std::collections::{HashMap, hash_map::Entry};
use std::fs::read_to_string;
use std::str::FromStr;
use itertools::{Itertools, iproduct};
use rayon::prelude::*;

// R123,U22,L22,D10,R10

#[derive(Clone,Copy)]
enum Direction {
    X(i32),
    Y(i32),
}
enum FuckYouError {
    BullshitNumber(String),
    RetardedDirection(char),
    EmptyShit,
}

impl FromStr for Direction {
    type Err = FuckYouError;
    fn from_str(s: &str) -> Result<Direction, Self::Err> {
        let number_part: i32 = (&s[1..]).parse::<i32>()
            .map_err(|_e| FuckYouError::BullshitNumber(s.to_string()))?;
        match s.chars().nth(0) {
            Some('R') => Ok(Direction::X(number_part)),
            Some('L') => Ok(Direction::X(-number_part)),
            Some('U') => Ok(Direction::Y(number_part)),
            Some('D') => Ok(Direction::Y(-number_part)),
            Some(x) => Err(FuckYouError::RetardedDirection(x)),
            None => Err(FuckYouError::EmptyShit),
        }
    }
}

impl std::ops::Add<Direction> for geo::Point<i32> {
    type Output = Line<i32>;

    fn add(self, rhs: Direction) -> Self::Output {
        let start = self.clone();
        let end = match rhs {
            Direction::X(x) => (self.x() + x, self.y()),
            Direction::Y(y) => (self.y(), self.y() + y),
        };
        Line {
            start: start.into(),
            end: end.into(),
        }
    }
}

#[derive(Debug)]
enum Either {
    LineA,
    LineB,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fname: String = std::env::args().skip(1).take(1).collect();
    let data = read_to_string(fname)?;
    let mut lines = data.lines();
    let wire1 = lines.next().unwrap();
    let wire2 = lines.next().unwrap();
    let lines1: Vec<Line<i32>> = wire1
        .split(",")
        .flat_map(|s| s.parse::<Direction>().ok())
        .scan(geo::point!(x: 0, y: 0), |origin, dir| {
            let line = *origin + dir;
            *origin = line.end.into();
            Some(line)
        }).collect();
    let lines2: Vec<Line<i32>> = wire2
        .split(",")
        .flat_map(|s| s.parse::<Direction>().ok())
        .scan(geo::point!(x: 0, y: 0), |origin, dir| {
            let line = *origin + dir;
            *origin = line.end.into();
            Some(line)
        }).collect();
    //let mut segment1 = Vec::new();
    for line in lines1 {
        println!("{:?}", line);
    }
    println!("-----------");
    for line in lines2 {
        println!("{:?}", line);
    }
    return Ok(());
/*
    let mut start1: Coordinate<i32> = (0, 0).into();

    let mut limit = 10;
    let mut intersections: Vec<(i32, i32)> = Vec::new();
    let mut path_map: HashMap<(i32, i32), Either> = HashMap::new();
    for segment in lines1 {
        let (x, y) = match segment {
            Direction::X(x) => {
                let mut coord: (i32, i32) = (0, 0);
                for i in 0..=x {
                    coord = (start1.x + i, start1.y);
                    path_map.entry(coord).or_insert(Either::LineA);
                }
                coord
            },
            Direction::Y(y) => {
                let mut coord: (i32, i32) = (0, 0);
                for i in 0..=y {
                    coord = (start1.x, start1.y + i);
                    path_map.entry(coord).or_insert(Either::LineA);
                }
                coord
            },
        };
        // update start to next place
        start1.x = x;
        start1.y = y;
    }
    let mut start2: Coordinate<i32> = (0, 0).into();
    for segment in lines2 {
        let (x, y) = match segment {
            Direction::X(x) => {
                let mut coord: (i32, i32) = (0, 0);
                for i in 0..=x {
                    coord = (start2.x + i, start2.y);
                    if let Entry::Occupied(entry) = path_map.entry(coord) {
                        intersections.push(*entry.key());
                    }
                }
                coord
            },
            Direction::Y(y) => {
                let mut coord: (i32, i32) = (0, 0);
                for i in 0..=y {
                    coord = (start2.x, start2.y + i);
                    if let Entry::Occupied(entry) = path_map.entry(coord) {
                        intersections.push(*entry.key());
                    }
                    path_map.entry(coord).or_insert(Either::LineA);
                }
                coord
            },
        };
        // update start to next place
        start2.x = x;
        start2.y = y;
    }

    /*
    for ((x, y), value) in path_map.iter() {
        println!("(x:{}, y:{}) -> {:?}", x, y, value);
    }
    std::thread::sleep(std::time::Duration::from_secs(600));
    */
    for (x, y) in intersections {
        println!("({}, {}) {}", x, y, x+y);
    }
    Ok(())
    */
}