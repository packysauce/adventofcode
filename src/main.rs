use failure::{format_err, Fallible};
use rayon::prelude::*;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, BufReader, Cursor};

trait Instruction {
    fn execute(self, cpu: &mut IntcodeMachine);
}

#[derive(Debug)]
enum MachineError {
    Halted,
    OutOfBounds(usize, usize),
    InvalidOpcode(i32),
}

impl Display for MachineError {
    fn fmt(&self, w: &mut Formatter) -> FmtResult {
        write!(w, "{:?}", self)
    }
}

impl std::error::Error for MachineError {}

struct IntcodeMachine {
    ip: usize,
    halted: bool,
    data: Vec<i32>,
}

impl<'a> IntcodeMachine {
    fn new(data: &[i32]) -> IntcodeMachine {
        IntcodeMachine {
            ip: 0,
            halted: false,
            data: data.to_vec(),
        }
    }

    fn set_cell(&mut self, pos: usize, val: i32) {
        self.data[pos] = val;
    }

    fn value_at(&self, pos: &Parameter) -> i32 {
        match *pos {
            Parameter::Indirect(x) => self.data[x],
            Parameter::Immediate(x) => x,
        }
    }

    fn as_ref(&'a self) -> &'a [i32] {
        self.data.as_ref()
    }

    fn set_ip(&mut self, pos: usize) {
        self.ip = pos
    }

    fn halt(&mut self) {
        self.halted = true
    }

    fn unpack_op(&mut self) -> Fallible<Opcode> {
        if self.halted {
            return Err(MachineError::Halted.into());
        }
        if self.ip >= self.data.len() {
            return Err(MachineError::OutOfBounds(self.ip, self.data.len()).into());
        }
        let op: i32 = self.data[self.ip];
        let opcode = op % 100;
        let flags = op / 100;
        
        let result = match opcode {
            1 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Add {
                    x: Parameter::of_kind_and_value(flags / 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 100, self.data[ip + 2])?,
                    dest: Parameter::of_kind_and_value(flags / 1000, self.data[ip + 3])?,
                })
            },
            2 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Mul {
                    x: Parameter::of_kind_and_value(flags / 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 100, self.data[ip + 2])?,
                    dest: Parameter::of_kind_and_value(flags / 1000, self.data[ip + 3])?,
                })
            },
            99 => Ok(Opcode::Halt),
            _ => Err(MachineError::InvalidOpcode(99).into()),
        };

        result
    }

    fn run(&mut self) {
        while let Ok(opcode) = self.unpack_op() {
            opcode.execute(self);
        }
    }
}

#[derive(Debug, PartialEq)]
enum Parameter {
    Immediate(i32),
    Indirect(usize),
}

impl Parameter {
    fn of_kind_and_value(kind: i32, value: i32) -> Fallible<Parameter> {
        match kind {
            0 => Ok(Parameter::Indirect(value as usize)),
            1 => Ok(Parameter::Immediate(value)),
            _ => Err(MachineError::InvalidOpcode(kind).into()),
        }
    }

    fn from_cpu(&self, cpu: &IntcodeMachine) -> i32 {
        match *self {
            Parameter::Immediate(x) => x as i32,
            _ => cpu.value_at(&self),
        }
    }
}

impl Display for Parameter {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            Parameter::Immediate(x) => write!(f, "{}", x),
            Parameter::Indirect(x) => write!(f, "({})", x),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Opcode {
    Add {
        x: Parameter,
        y: Parameter,
        dest: Parameter,
    },
    Mul {
        x: Parameter,
        y: Parameter,
        dest: Parameter,
    },
    Halt,
}

impl Display for Opcode {
    fn fmt(&self, w: &mut Formatter) -> FmtResult {
        match self {
            Opcode::Add { x, y, dest } => write!(w, "add %{} + %{} => %{}", x, y, dest),
            Opcode::Mul { x, y, dest } => write!(w, "mul %{} * %{} => %{}", x, y, dest),
            Opcode::Halt => write!(w, "halt"),
        }
    }
}

impl Instruction for Opcode {
    fn execute(self, cpu: &mut IntcodeMachine) {
        match self {
            Opcode::Add { x, y, dest } => cpu.set_cell(
                dest.from_cpu(&cpu) as usize,
                cpu.value_at(&x) + cpu.value_at(&y),
            ),
            Opcode::Mul { x, y, dest } => cpu.set_cell(
                dest.from_cpu(&cpu) as usize,
                cpu.value_at(&x) * cpu.value_at(&y),
            ),
            Opcode::Halt => cpu.halt(),
        }
    }
}

fn main() {
    let fname: String = std::env::args().skip(1).take(1).collect();
    let input_data = std::fs::read(fname)
        .map(Cursor::new)
        .map(BufReader::new)
        .expect("couldnt read the file");

    let mut data: Vec<i32> = Vec::new();

    for line in input_data.lines() {
        for chunk in line.unwrap().split(',') {
            if let Ok(parsed) = chunk.parse() {
                data.push(parsed)
            } else {
                eprintln!("dropping chunk {}", chunk);
            }
        }
    }

    let mut pairs: Vec<(i32, i32)> = Vec::new();
    for i in 0..=99 {
        for j in 0..=99 {
            pairs.push((i, j));
        }
    }
    let result: Option<(i32, i32)> = pairs.par_iter().find_map_any(|(verb, noun)| {
        let mut machine = IntcodeMachine::new(&data);
        machine.set_cell(1, *verb);
        machine.set_cell(2, *noun);
        machine.run();
        if 19690720 == machine.value_at(&Parameter::Indirect(0)) {
            Some((*verb, *noun))
        } else {
            None
        }
    });

    if let Some((verb, noun)) = result {
        println!("verb {}, noun {}, code: {}", verb, noun, 100 * verb + noun);
    } else {
        println!("you gotta be fuckin kidding me");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_single_add() {
        let data = &[1, 0, 0, 0, 99];
        let mut machine = IntcodeMachine::new(data);
        let result = machine.unpack_op().unwrap();
        let expected = Opcode::Add {
            x: Parameter::Indirect(0),
            y: Parameter::Indirect(0),
            dest: Parameter::Indirect(0),
        };
        assert_eq!(result, expected);
        assert_eq!(machine.unpack_op().unwrap(), Opcode::Halt);
    }

    #[test]
    fn test_read_single_mul() {
        let data = &[2, 0, 0, 0, 99];
        let mut machine = IntcodeMachine::new(data);
        let result = machine.unpack_op().unwrap();
        let expected = Opcode::Mul {
            x: Parameter::Indirect(0),
            y: Parameter::Indirect(0),
            dest: Parameter::Indirect(0),
        };
        assert_eq!(result, expected);
        assert_eq!(machine.unpack_op().unwrap(), Opcode::Halt);
    }

    #[test]
    fn test_single_add() {
        let data = &[1, 5, 2, 3, 99, 0];
        let mut machine = IntcodeMachine::new(data);
        machine.run();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 2);
    }

    #[test]
    fn test_single_mul() {
        let data = &[2, 0, 0, 3, 99];
        let mut machine = IntcodeMachine::new(data);
        machine.run();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 4);
    }
}
