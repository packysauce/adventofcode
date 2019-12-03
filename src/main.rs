use failure::{format_err, Fallible};
use std::fmt::{Result as FmtResult, Formatter, Display};
use std::fs::read;
use std::io::{BufRead, BufReader, Cursor};

trait Instruction {
    fn execute(self, cpu: &mut IntcodeMachine);
}

#[derive(Debug)]
enum MachineError {
    Halted,
    OutOfBounds(usize, usize),
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

    fn value_at(&self, pos: usize) -> i32 {
        self.data[pos]
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

    fn read_op(&mut self) -> Fallible<Opcode> {
        if self.halted {
            return Err(MachineError::Halted.into())
        }
        if self.ip >= self.data.len() {
            return Err(MachineError::OutOfBounds(self.ip, self.data.len()).into())
        }
        let ip = self.ip;
        match self.data[self.ip] {
            1 => {
                self.ip += 4;
                Ok(Opcode::Add {
                    x: self.data[ip + 1] as usize,
                    y: self.data[ip + 2] as usize,
                    dest: self.data[ip + 3] as usize,
                })
            }
            2 => {
                self.ip += 4;
                Ok(Opcode::Mul {
                    x: self.data[ip + 1] as usize,
                    y: self.data[ip + 2] as usize,
                    dest: self.data[ip + 3] as usize,
                })
            }
            99 => Ok(Opcode::Halt),
            _ => Err(format_err!("Bad opcode {}", &self.data[self.ip])),
        }
    }

    fn run(&mut self) {
        while let Ok(opcode) = self.read_op() {
            opcode.execute(self);
        }
    }
}

#[derive(Debug, PartialEq)]
enum Opcode {
    Add { x: usize, y: usize, dest: usize },
    Mul { x: usize, y: usize, dest: usize },
    Halt,
}

impl Display for Opcode {
    fn fmt(&self, w: &mut Formatter) -> FmtResult {
        match self {
            Opcode::Add {x, y, dest} => write!(w, "add %{} + %{} => %{}", x, y, dest),
            Opcode::Mul {x, y, dest} => write!(w, "mul %{} * %{} => %{}", x, y, dest),
            Opcode::Halt => write!(w, "halt"),
        }
    }
}


impl Instruction for Opcode {
    fn execute(self, cpu: &mut IntcodeMachine) {
        match self {
            Opcode::Add { x, y, dest } => cpu.set_cell(dest, cpu.value_at(x) + cpu.value_at(y)),
            Opcode::Mul { x, y, dest } => cpu.set_cell(dest, cpu.value_at(x) * cpu.value_at(y)),
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
    let mut machine = IntcodeMachine::new(&data);
    machine.set_cell(1, 12);
    machine.set_cell(2, 2);
    machine.run();
    dbg!(machine.value_at(0));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_single_add() {
        let data = &[1, 0, 0, 0, 99];
        let mut machine = IntcodeMachine::new(data);
        let result = machine.read_op().unwrap();
        let expected = Opcode::Add {
            x: 0,
            y: 0,
            dest: 0,
        };
        assert_eq!(result, expected);
        assert_eq!(machine.read_op().unwrap(), Opcode::Halt);
    }

    #[test]
    fn test_read_single_mul() {
        let data = &[2, 0, 0, 0, 99];
        let mut machine = IntcodeMachine::new(data);
        let result = machine.read_op().unwrap();
        let expected = Opcode::Mul {
            x: 0,
            y: 0,
            dest: 0,
        };
        assert_eq!(result, expected);
        assert_eq!(machine.read_op().unwrap(), Opcode::Halt);
    }

    #[test]
    fn test_single_add() {
        let data = &[1, 5, 2, 3, 99, 0];
        let mut machine = IntcodeMachine::new(data);
        machine.run();
        assert_eq!(machine.value_at(3), 2);
    }

    #[test]
    fn test_single_mul() {
        let data = &[2, 0, 0, 3, 99];
        let mut machine = IntcodeMachine::new(data);
        machine.run();
        assert_eq!(machine.value_at(3), 4);
    }
}
