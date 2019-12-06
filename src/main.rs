use failure::{format_err, Fallible, ResultExt};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{BufRead, BufReader, Cursor, self};

const DEBUG: bool = true;

trait Output {
    fn output(&mut self, what: i32) -> Fallible<()>;
}

trait Input {
    fn input(&mut self) -> Fallible<i32>;
}

trait Instruction {
    fn execute(self, cpu: &mut IntcodeMachine) -> Fallible<()>;
}

struct MockIO {
    current_input: usize,
    inputs: Vec<i32>,
    outputs: Vec<i32>,
}

impl Input for MockIO {
    fn input(&mut self) -> Fallible<i32> {
        if let Some(x) = self.inputs.get(self.current_input) {
            let result = Ok(*x);
            self.current_input += 1;
            result
        } else {
            Err(MachineError::EOF.into())
        }
    }
}

impl Output for MockIO {
    fn output(&mut self, what: i32) -> Fallible<()> {
        self.outputs.push(what);
        Ok(())
    }

}

impl Input for dyn std::io::Read {
    fn input(&mut self) -> Fallible<i32> {
        let mut buf = BufReader::new(self);
        let mut s = String::new();
        buf.read_line(&mut s)?;
        Ok(s.parse()?)
    }
}

impl Output for dyn std::io::Write {
    fn output(&mut self, what: i32) -> Fallible<()> {
        Ok(write!(self, "{}", what)?)
    }
}

#[derive(Debug)]
enum MachineError {
    Halted,
    OutOfBounds(usize, usize),
    InvalidOpcode(i32),
    EOF,
    InvalidIndirect,
    InvalidImmediate,
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

    fn set_cell(&mut self, pos: usize, val: i32) -> Fallible<()> {
        if DEBUG {
            println!("self.data[{}] <- {} (was: {})", pos, val, self.data[pos]);
        }
        if pos > self.data.len() {
            return Err(MachineError::OutOfBounds(pos, self.data.len()).into())
        }
        self.data[pos] = val;
        Ok(())
    }

    fn value_at(&self, pos: &Parameter) -> i32 {
        match *pos {
            Parameter::Indirect(x) => {
                if DEBUG {
                    println!("self.data[{}] = {}", x, self.data[x]);
                }
                self.data[x]
            },
            Parameter::Immediate(x) => {
                if DEBUG {
                    println!("immediate: {}", x);
                }
                x
            },
        }
    }

    fn as_ref(&'a self) -> &'a [i32] {
        self.data.as_ref()
    }

    fn set_ip(&mut self, pos: usize) -> Fallible<()> {
        if pos > self.data.len() {
            Err(MachineError::OutOfBounds(pos, self.data.len()).into())
        } else {
            if DEBUG {
                println!("cpu.ip <= {} (was {})", pos, self.ip);
            }
            self.ip = pos;
            Ok(())
        }
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
        if DEBUG {
            println!("opcode: {}, flags: {}", opcode, flags);
        }
        
        let result = match opcode {
            1 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Add {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            },
            2 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Mul {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            },
            3 => {
                let ip = self.ip;
                self.ip += 2;
                Ok(Opcode::Input {
                    x: self.data[ip + 1] as usize,
                })
            },
            4 => {
                let ip = self.ip;
                self.ip += 2;
                Ok(Opcode::Output {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                })
            },
            5 => {
                let ip = self.ip;
                self.ip += 3;
                Ok(Opcode::JumpIfTrue {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    dest: self.data[ip + 2] as usize,
                })
            },
            6 => {
                let ip = self.ip;
                self.ip += 3;
                Ok(Opcode::JumpIfFalse {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    dest: self.data[ip + 2] as usize,
                })
            },
            7 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::LessThan {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            },
            8 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Equal {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            },
            99 => Ok(Opcode::Halt),
            x => Err(MachineError::InvalidOpcode(x).into()),
        };

        result
    }

    fn run(&mut self) -> Fallible<()> {
        loop {
            let op = self.unpack_op()?;
            if DEBUG {
                println!("{:?}", op);
            }
            match op {
                Opcode::Halt => return Ok(()),
                x => x.execute(self)?,
            }
        }
    }
}

#[derive(Debug, PartialEq)]
enum Parameter {
    Immediate(i32),
    Indirect(usize),
}

impl Parameter {
    fn as_address(&self) -> Fallible<usize> {
        match *self {
            Parameter::Indirect(x) => Ok(x),
            _ => Err(format_err!("as_address called on immediate value")),
        }
    }

    fn of_kind_and_value(kind: i32, value: i32) -> Fallible<Parameter> {
        match kind {
            0 => Ok(Parameter::Indirect(value as usize)),
            1 => Ok(Parameter::Immediate(value)),
            _ => Err(MachineError::InvalidOpcode(kind).into()),
        }
    }

    fn from_cpu(&self, cpu: &IntcodeMachine) -> i32 {
        match *self {
            Parameter::Immediate(x) => x,
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
        dest: usize,
    },
    Mul {
        x: Parameter,
        y: Parameter,
        dest: usize,
    },
    Input {
        x: usize,
    },
    Output {
        x: Parameter,
    },
    JumpIfTrue {
        x: Parameter,
        dest: usize,
    },
    JumpIfFalse {
        x: Parameter,
        dest: usize,
    },
    LessThan {
        x: Parameter,
        y: Parameter,
        dest: usize,
    },
    Equal {
        x: Parameter,
        y: Parameter,
        dest: usize,
    },
    Halt,
}

impl Display for Opcode {
    fn fmt(&self, w: &mut Formatter) -> FmtResult {
        match self {
            Opcode::Add { x, y, dest } => write!(w, "add {} + {} => {}", x, y, dest),
            Opcode::Mul { x, y, dest } => write!(w, "mul {} * {} => {}", x, y, dest),
            Opcode::Input { x } => write!(w, "input -> {}", x),
            Opcode::Output { x } => write!(w, "{} -> output", x),
            Opcode::JumpIfFalse { x, dest} => write!(w, "jmp-false {} -> {}", x, dest),
            Opcode::JumpIfTrue { x, dest} => write!(w, "jmp-true {} -> {}", x, dest),
            Opcode::LessThan { x, y, dest} => write!(w, "lessthan {} < {} => {}", x, y, dest),
            Opcode::Equal { x, y, dest} => write!(w, "equal {} == {} => {}", x, y, dest),
            Opcode::Halt => write!(w, "halt"),
        }
    }
}

impl Instruction for Opcode {
    fn execute(self, cpu: &mut IntcodeMachine) -> Fallible<()> {
        match self {
            Opcode::Add { x, y, dest } => cpu.set_cell(
                dest,
                cpu.value_at(&x) + cpu.value_at(&y),
            )?,
            Opcode::Mul { x, y, dest } => cpu.set_cell(
                dest,
                cpu.value_at(&x) * cpu.value_at(&y),
            )?,
            Opcode::Input { x} => {
                let mut s = String::new();
                io::stdin().read_line(&mut s)?;
                let value: i32 = s.trim().parse::<i32>().context("parsing input")?;
                cpu.set_cell(x, value)?;
            },
            Opcode::Output {x} => {
                println!("{}", cpu.value_at(&x));
            },
            Opcode::JumpIfTrue {x, dest} => {
                if cpu.value_at(&x) != 0 {
                    cpu.set_ip(dest)?;
                }
            },
            Opcode::JumpIfFalse {x, dest} => {
                if cpu.value_at(&x) == 0 {
                    cpu.set_ip(dest)?;
                }
            },
            Opcode::LessThan {x, y, dest} => {
                if cpu.value_at(&x) < cpu.value_at(&y) {
                    cpu.set_cell(dest, 1)?;
                } else {
                    cpu.set_cell(dest, 0)?;
                }
            },
            Opcode::Equal {x, y, dest} => {
                if cpu.value_at(&x) == cpu.value_at(&y) {
                    cpu.set_cell(dest, 1)?;
                } else {
                    cpu.set_cell(dest, 0)?;
                }
            },
            Opcode::Halt => cpu.halt(),
        };
        Ok(())
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
    if let Err(e) = machine.run() {
        eprintln!("{:?}", &e);
        for i in e.iter_causes() {
            eprintln!("{:?}", i);
        }
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
            dest: 0,
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
            dest: 0,
        };
        assert_eq!(result, expected);
        assert_eq!(machine.unpack_op().unwrap(), Opcode::Halt);
    }

    #[test]
    fn test_single_add() {
        let data = &[1, 5, 2, 3, 99, 0];
        let mut machine = IntcodeMachine::new(data);
        machine.run().unwrap();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 2);
    }

    #[test]
    fn test_single_mul() {
        let data = &[2, 0, 0, 3, 99];
        let mut machine = IntcodeMachine::new(data);
        machine.run().unwrap();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 4);
    }
}
