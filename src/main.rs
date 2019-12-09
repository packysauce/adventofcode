use failure::{format_err, Fallible, ResultExt};
use lazy_static::lazy_static;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::io::{self, BufRead, BufReader, Cursor, Write};
use permutohedron::Heap;
use std::collections::HashMap;

lazy_static! {
    static ref DEBUG: bool = { std::env::var("DEBUG").is_ok() };
}

trait Output {
    fn output(&mut self, what: i32) -> Fallible<()>;
    fn results(&mut self) -> Option<Vec<i32>>;
}

trait Input {
    fn input(&mut self) -> Fallible<i32>;
}

trait Instruction {
    fn execute(self, cpu: &mut IntcodeMachine) -> Fallible<()>;
}

struct MockInput {
    current_input: usize,
    inputs: Vec<i32>,
}

impl Input for Vec<i32> {
    fn input(&mut self) -> Fallible<i32> {
        self.pop().ok_or(MachineError::EOF.into())
    }
}

impl Output for Vec<i32> {
    fn output(&mut self, what: i32) -> Fallible<()> {
        Ok(self.push(what))
    }

    fn results(&mut self) -> Option<Vec<i32>> {
        Some(self.clone())
    }
}

impl Input for MockInput {
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

impl Input for io::Stdin {
    fn input(&mut self) -> Fallible<i32> {
        let mut buf = BufReader::new(self);
        let mut s = String::new();
        buf.read_line(&mut s)?;
        Ok(s.trim().parse()?)
    }
}

impl Output for io::Stdout {
    fn output(&mut self, what: i32) -> Fallible<()> {
        Ok(writeln!(self, "{}", what)?)
    }

    fn results(&mut self) -> Option<Vec<i32>> {
        None
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
    input: Box<dyn Input>,
    output: Option<Box<dyn Output>>,
}

impl<'a> IntcodeMachine {
    fn new(data: &[i32], input: Box<dyn Input>, output: Box<dyn Output>) -> IntcodeMachine {
        IntcodeMachine {
            ip: 0,
            halted: false,
            data: data.to_vec(),
            input,
            output: Some(output),
        }
    }

    fn take_output(mut self) -> Option<Box<dyn Output>> {
        self.output.take()
    }

    fn set_cell(&mut self, pos: usize, val: i32) -> Fallible<()> {
        if *DEBUG {
            println!("self.data[{}] <- {} (was: {})", pos, val, self.data[pos]);
        }
        if let Some(x) = self.data.get_mut(pos) {
            *x = val;
            Ok(())
        } else {
            Err(MachineError::OutOfBounds(pos, self.data.len()).into())
        }
    }

    fn value_at(&self, pos: &Parameter) -> i32 {
        match *pos {
            Parameter::Indirect(x) => {
                if *DEBUG {
                    println!("self.data[{}] = {}", x, self.data[x]);
                }
                self.data[x]
            }
            Parameter::Immediate(x) => {
                if *DEBUG {
                    println!("immediate: {}", x);
                }
                x
            }
        }
    }

    fn input(&mut self) -> Fallible<i32> {
        self.input.input()
    }

    fn output(&mut self, what: i32) -> Fallible<()> {
        if let Some(output) = self.output.as_mut() {
            output.output(what)?
        }
        Ok(())
    }

    fn as_ref(&'a self) -> &'a [i32] {
        self.data.as_ref()
    }

    fn set_ip(&mut self, pos: usize) -> Fallible<()> {
        if pos > self.data.len() {
            Err(MachineError::OutOfBounds(pos, self.data.len()).into())
        } else {
            if *DEBUG {
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
        if *DEBUG {
            println!("ip: {}, opcode: {}, flags: {}", self.ip, opcode, flags);
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
            }
            2 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Mul {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            }
            3 => {
                let ip = self.ip;
                self.ip += 2;
                Ok(Opcode::Input {
                    x: self.data[ip + 1] as usize,
                })
            }
            4 => {
                let ip = self.ip;
                self.ip += 2;
                Ok(Opcode::Output {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                })
            }
            5 => {
                let ip = self.ip;
                self.ip += 3;
                Ok(Opcode::JumpIfTrue {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    dest: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                })
            }
            6 => {
                let ip = self.ip;
                self.ip += 3;
                Ok(Opcode::JumpIfFalse {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    dest: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                })
            }
            7 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::LessThan {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            }
            8 => {
                let ip = self.ip;
                self.ip += 4;
                Ok(Opcode::Equal {
                    x: Parameter::of_kind_and_value(flags % 10, self.data[ip + 1])?,
                    y: Parameter::of_kind_and_value(flags / 10, self.data[ip + 2])?,
                    dest: self.data[ip + 3] as usize,
                })
            }
            99 => Ok(Opcode::Halt),
            _ => Err(MachineError::InvalidOpcode(opcode).into()),
        };

        result
    }

    fn run(&mut self) -> Fallible<()> {
        loop {
            let op = self.unpack_op()?;
            if *DEBUG {
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
        dest: Parameter,
    },
    JumpIfFalse {
        x: Parameter,
        dest: Parameter,
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
    Noop,
}

impl Display for Opcode {
    fn fmt(&self, w: &mut Formatter) -> FmtResult {
        match self {
            Opcode::Add { x, y, dest } => write!(w, "add {} + {} => {}", x, y, dest),
            Opcode::Mul { x, y, dest } => write!(w, "mul {} * {} => {}", x, y, dest),
            Opcode::Input { x } => write!(w, "input -> {}", x),
            Opcode::Output { x } => write!(w, "{} -> output", x),
            Opcode::JumpIfFalse { x, dest } => write!(w, "jmp-false {} -> {}", x, dest),
            Opcode::JumpIfTrue { x, dest } => write!(w, "jmp-true {} -> {}", x, dest),
            Opcode::LessThan { x, y, dest } => write!(w, "lessthan {} < {} => {}", x, y, dest),
            Opcode::Equal { x, y, dest } => write!(w, "equal {} == {} => {}", x, y, dest),
            Opcode::Halt => write!(w, "halt"),
            Opcode::Noop => write!(w, "noop"),
        }
    }
}

impl Instruction for Opcode {
    fn execute(self, cpu: &mut IntcodeMachine) -> Fallible<()> {
        match self {
            Opcode::Add { x, y, dest } => {
                cpu.set_cell(dest, cpu.value_at(&x) + cpu.value_at(&y))?
            }
            Opcode::Mul { x, y, dest } => {
                cpu.set_cell(dest, cpu.value_at(&x) * cpu.value_at(&y))?
            }
            Opcode::Input { x } => {
                let value = cpu.input()?;
                cpu.set_cell(x, value)?;
            }
            Opcode::Output { x } => {
                cpu.output(cpu.value_at(&x))?;
            }
            Opcode::JumpIfTrue { x, dest } => {
                if cpu.value_at(&x) != 0 {
                    cpu.set_ip(cpu.value_at(&dest) as usize)?;
                }
            }
            Opcode::JumpIfFalse { x, dest } => {
                if cpu.value_at(&x) == 0 {
                    cpu.set_ip(cpu.value_at(&dest) as usize)?;
                }
            }
            Opcode::LessThan { x, y, dest } => {
                if cpu.value_at(&x) < cpu.value_at(&y) {
                    cpu.set_cell(dest, 1)?;
                } else {
                    cpu.set_cell(dest, 0)?;
                }
            }
            Opcode::Equal { x, y, dest } => {
                if cpu.value_at(&x) == cpu.value_at(&y) {
                    cpu.set_cell(dest, 1)?;
                } else {
                    cpu.set_cell(dest, 0)?;
                }
            }
            Opcode::Halt => cpu.halt(),
            Opcode::Noop => (),
        };
        Ok(())
    }
}

fn main() -> Fallible<()> {
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

    let mut inputs: Vec<_> = (0..=4).collect();
    let heap = Heap::new(&mut inputs);
    //let mut prev_output = 0;
    let results: HashMap<_, _> = heap.into_iter().flat_map(|phase_order: Vec<i32>| {
        let mut prev_output = 0;
        for phase in phase_order.iter() {
            let mut cpu = IntcodeMachine::new(&data, Box::new(vec![prev_output, *phase]), Box::new(Vec::new()));
            if cpu.run().is_err() { return None };
            let results = cpu.take_output().unwrap().results().unwrap();
            assert_eq!(results.len(), 1);
            prev_output = *results.get(0).unwrap();
        }
        Some((phase_order, prev_output))
    }).collect();

    let x = results.iter().max_by(|(_, out1), (_, out2)| {
        out1.cmp(out2)
    });
    println!("{:?}", x);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_single_add() {
        let data = &[1, 0, 0, 0, 99];
        let mut machine = IntcodeMachine::new(data, Box::new(Vec::new()), Box::new(Vec::new()));
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
        let mut machine = IntcodeMachine::new(data, Box::new(Vec::new()), Box::new(Vec::new()));
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
        let mut machine = IntcodeMachine::new(data, Box::new(Vec::new()), Box::new(Vec::new()));
        machine.run().unwrap();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 2);
    }

    #[test]
    fn test_single_mul() {
        let data = &[2, 0, 0, 3, 99];
        let mut machine = IntcodeMachine::new(data, Box::new(Vec::new()), Box::new(Vec::new()));
        machine.run().unwrap();
        assert_eq!(machine.value_at(&Parameter::Indirect(3)), 4);
    }

    #[test]
    fn test_io() {
        let ins: Box<Vec<i32>> = Box::new(vec![99]);
        let outs: Box<Vec<i32>> = Box::new(Vec::new());
        let data = vec![3, 3, 104, 0, 99];
        let mut machine = IntcodeMachine::new(&data, ins, outs);
        machine.run().unwrap();
        println!("{:?}", &machine.data.clone());
        let mut output = machine.take_output().unwrap();
        assert_eq!(output.results(), Some(vec![99]));
    }

    #[test]
    fn test_jmp_if_false() {
        let ins: Box<Vec<i32>> = Box::new(Vec::new());
        let outs: Box<Vec<i32>> = Box::new(Vec::new());
        let data = vec![
            106, 0, 6, // jump to 6 if 0 is true
            104, 69, 99, // trap! 69 is bad number
            104, 420, 99, // print imm(420)
        ];
        let mut machine = IntcodeMachine::new(&data, ins, outs);
        machine.run().unwrap();
        println!("{:?}", &machine.data.clone());
        let mut output = machine.take_output().unwrap();
        assert_eq!(output.results(), Some(vec![420]));
    }

    #[test]
    fn test_jmp_if_true() {
        let ins: Box<Vec<i32>> = Box::new(Vec::new());
        let outs: Box<Vec<i32>> = Box::new(Vec::new());
        let data = vec![
            105, 0, 6, // jump to 6 if 0 is true
            104, 69, 99, // trap! 69 is bad number
            104, 420, 99, // print imm(420)
        ];
        let mut machine = IntcodeMachine::new(&data, ins, outs);
        machine.run().unwrap();
        let mut output = machine.take_output().unwrap();
        assert_eq!(output.results(), Some(vec![69]));
    }

    #[test]
    fn test_equal() {
        for (i, expected) in vec![(7, 0), (8, 1), (9, 0)] {
            let ins: Box<Vec<i32>> = Box::new(vec![i]);
            let outs: Box<Vec<i32>> = Box::new(Vec::new());
            let data = vec![3, 9, 8, 9, 10, 9, 4, 9, 99, -1, 8];
            let mut machine = IntcodeMachine::new(&data, ins, outs);
            machine.run().unwrap();
            let mut output = machine.take_output().unwrap();
            assert_eq!(output.results(), Some(vec![expected]));
        }
    }

    #[test]
    fn test_equal_imm() {
        for (i, expected) in vec![(7, 0), (8, 1), (9, 0)] {
            let ins: Box<Vec<i32>> = Box::new(vec![i]);
            let outs: Box<Vec<i32>> = Box::new(Vec::new());
            let data = vec![3, 3, 1108, -1, 8, 3, 4, 3, 99];
            let mut machine = IntcodeMachine::new(&data, ins, outs);
            machine.run().unwrap();
            let mut output = machine.take_output().unwrap();
            assert_eq!(output.results(), Some(vec![expected]));
        }
    }

    #[test]
    fn test_less() {
        for (i, expected) in vec![(7, 1), (8, 0), (9, 0)] {
            let ins: Box<Vec<i32>> = Box::new(vec![i]);
            let outs: Box<Vec<i32>> = Box::new(Vec::new());
            let data = vec![3, 9, 7, 9, 10, 9, 4, 9, 99, -1, 8];
            let mut machine = IntcodeMachine::new(&data, ins, outs);
            machine.run().unwrap();
            let mut output = machine.take_output().unwrap();
            assert_eq!(output.results(), Some(vec![expected]));
        }
    }

    #[test]
    fn test_less_imm() {
        for (i, expected) in vec![(7, 1), (8, 0), (9, 0)] {
            let ins: Box<Vec<i32>> = Box::new(vec![i]);
            let outs: Box<Vec<i32>> = Box::new(Vec::new());
            let data = vec![3, 3, 1107, -1, 8, 3, 4, 3, 99];
            let mut machine = IntcodeMachine::new(&data, ins, outs);
            machine.run().unwrap();
            let mut output = machine.take_output().unwrap();
            assert_eq!(output.results(), Some(vec![expected]));
        }
    }

    #[test]
    fn test_large_program() {
        for (i, expected) in vec![(7, 999), (8, 1000), (9, 1001)] {
            let ins: Box<Vec<i32>> = Box::new(vec![i]);
            let outs: Box<Vec<i32>> = Box::new(Vec::new());
            let data = vec![
                3, 21, 1008, 21, 8, 20, 1005, 20, 22, 107, 8, 21, 20, 1006, 20, 31, 1106, 0, 36,
                98, 0, 0, 1002, 21, 125, 20, 4, 20, 1105, 1, 46, 104, 999, 1105, 1, 46, 1101, 1000,
                1, 20, 4, 20, 1105, 1, 46, 98, 99,
            ];
            let mut machine = IntcodeMachine::new(&data, ins, outs);
            machine.run().unwrap();
            let mut output = machine.take_output().unwrap();
            assert_eq!(output.results(), Some(vec![expected]));
        }
    }
}
