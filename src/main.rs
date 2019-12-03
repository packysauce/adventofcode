use geo::{Coordinate, Line};
use std::fs::read_to_string;
use std::str::FromStr;

// R123,U22,L22,D10,R10

#[derive(Clone,Copy)]
enum Direction {
    Right(i32),
    Left(i32),
    Down(i32),
    Up(i32),
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
            Some('R') => Ok(Direction::Right(number_part)),
            Some('L') => Ok(Direction::Left(number_part)),
            Some('U') => Ok(Direction::Up(number_part)),
            Some('D') => Ok(Direction::Down(number_part)),
            Some(x) => Err(FuckYouError::RetardedDirection(x)),
            None => Err(FuckYouError::EmptyShit),
        }
    }
}

impl std::ops::Add<Direction> for Coordinate<i32> {
    type Output = Coordinate<i32>;

    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::Left(x) => Coordinate { x: self.x - x, y: self.y},
            Direction::Right(x) => Coordinate { x: self.x + x, y: self.y},
            Direction::Up(y) => Coordinate { x: self.x, y: self.y + y },
            Direction::Down(y) => Coordinate { x: self.x, y: self.y + y},
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fname: String = std::env::args().skip(1).take(1).collect();
    let data = read_to_string(fname)?;
    let mut lines = data.lines();
    let wire1 = lines.next().unwrap();
    let wire2 = lines.next().unwrap();
    let lines1: Vec<Direction> = wire1
        .split(",")
        .flat_map(|s| s.parse::<Direction>().ok()).collect();
    let lines2: Vec<Direction> = wire2
        .split(",")
        .flat_map(|s| s.parse::<Direction>().ok()).collect();
    let mut segment1 = Vec::new();
    let mut segment2 = Vec::new();
    let mut start1: Coordinate<i32> = (0, 0).into();
    for segment in lines1 {
        segment1.push(Line::new(
            start1,
            start1 + segment,
        ));
        start1 = start1 + segment;
    }
    let mut start2: Coordinate<i32> = (0, 0).into();
    for segment in lines2 {
        segment2.push(Line::new(
            start2,
            start2 + segment,
        ));
        start2 = start2 + segment;
    }

    for seg in segment1 {
        println!("{:?}", seg);
    }
    
    Ok(())
}