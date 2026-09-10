use std::collections::HashMap;

use crate::instructions::*;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: u8,
    pub program_counter: u16,
    pub stack_pointer: u8,
    memory: [u8; 0xFFFF],
}

impl CPU {
    pub fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            //add the value of the x register
            AddressingMode::ZeroPageX => {
                let address = self.mem_read(self.program_counter);
                address.wrapping_add(self.register_x) as u16
            }
            AddressingMode::ZeroPageY => {
                let address = self.mem_read(self.program_counter);
                address.wrapping_add(self.register_y) as u16
            }
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::AbsoluteX => {
                let address = self.mem_read_u16(self.program_counter);
                address.wrapping_add(self.register_x as u16)
            }
            AddressingMode::AbsoluteY => {
                let address = self.mem_read_u16(self.program_counter);
                address.wrapping_add(self.register_y as u16)
            }
            AddressingMode::IndirectX => {
                let base = self.mem_read(self.program_counter);

                let address = base.wrapping_add(self.register_x);
                let lo = self.mem_read(address as u16);
                let hi = self.mem_read(address.wrapping_add(1) as u16);

                let deref_adr = ((hi as u16) << 8) | (lo as u16);
                deref_adr.wrapping_add(self.register_y as u16)
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read(self.program_counter);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u16).wrapping_add(1));
                ((hi as u16) << 8) | (lo as u16)
            }
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::NoneAddressing => panic!("Cannot fetch data for non adressing"),
        }
    }
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: 0,
            program_counter: 0,
            stack_pointer: 0,
            memory: [0; 0xFFFF],
        }
    }
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.status = 0;
        self.program_counter = self.mem_read_u16(0xFFFC);
    }
    pub fn mem_read(&self, addr: u16) -> u8 {
        self.memory[addr as usize]
    }
    pub fn mem_read_u16(&self, pos: u16) -> u16 {
        let low_b = self.mem_read(pos) as u16;
        let high_b = self.mem_read(pos + 1) as u16;
        (high_b << 8) | low_b
    }
    pub fn mem_write(&mut self, addr: u16, val: u8) {
        self.memory[addr as usize] = val;
    }
    pub fn mem_write_u16(&mut self, pos: u16, data: u16) {
        let high_b = (data >> 8) as u8;
        let low_b = (data & 0xff) as u8;
        self.mem_write(pos, low_b);
        self.mem_write(pos + 1, high_b);
    }
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }
    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
    }
    pub fn run(&mut self) {
        let ref opcodes: HashMap<u8, &'static OpCode> = *OPCODES_MAP;
        loop {
            let code = self.memory[self.program_counter as usize];
            self.program_counter += 1;

            let opcode = opcodes
                .get(&code)
                .expect(&format!("invalid instruction: {:x}", code));

            match code {
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.adressing_mode);
                }

                0xA2 | 0xA6 | 0xb6 | 0xae | 0xbe => {
                    self.ldx(&opcode.adressing_mode);
                }
                0xAA => self.tax(),
                0xE8 => self.inx(),
                0x00 => return,
                _ => todo!(),
            }
            self.program_counter += (opcode.bytes - 1) as u16
        }
    }
    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.register_a = self.memory[address as usize];
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn ldx(&mut self, mode: &AddressingMode) {
        let adress = self.get_operand_address(mode);
        self.register_x = self.memory[adress as usize];
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn update_zero_and_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status |= 0b0000_0010;
        } else {
            self.status &= 0b1111_1101;
        }
        if result & 0b1000_0000 != 0 {
            self.status |= 0b1000_0000;
        } else {
            self.status &= 0b0111_1111;
        }
    }
}
