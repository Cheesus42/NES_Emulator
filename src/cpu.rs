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
                //adc
                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.adressing_mode)
                }
                //and
                0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 => {
                    self.and(&opcode.adressing_mode)
                }
                //asl
                0x0A | 0x06 | 0x16 | 0x0E | 0x1E => {
                    self.asl(&opcode.adressing_mode);
                }
                //bcc
                0x90 => self.bcc(),
                //bcs
                0xB0 => self.bcs(),
                //beq
                0xF0 => self.beq(),
                //lda
                0xA9 | 0xA5 | 0xB5 | 0xAD | 0xBD | 0xB9 | 0xA1 | 0xB1 => {
                    self.lda(&opcode.adressing_mode)
                }
                //ldx
                0xA2 | 0xA6 | 0xb6 | 0xae | 0xbe => self.ldx(&opcode.adressing_mode),
                //ldy
                0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc => self.ldy(&opcode.adressing_mode),
                0x38 => self.sec(),
                0xAA => self.tax(),
                0xE8 => self.inx(),
                0x00 => return,
                _ => todo!(),
            }
            self.program_counter += (opcode.bytes - 1) as u16
        }
    }
    fn adc(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        let adds = self.memory[address as usize];
        let carry_in = if self.status & 0b0000_0001 != 0 { 1 } else { 0 };

        let original_a = self.register_a;

        let (sum1, carry1) = self.register_a.overflowing_add(adds);
        let (sum2, carry2) = sum1.overflowing_add(carry_in);
        let carry_out = carry1 || carry2;
        let overflow = (original_a ^ sum2) & (adds ^ sum2) & 0x80 != 0;

        self.register_a = sum2;
        self.update_zero_and_negative_flags(self.register_a);
        if overflow {
            self.status |= 0b0100_0000;
        } else {
            self.status &= 0b1011_1111;
        }
        if carry_out {
            self.status |= 0b0000_0001;
        } else {
            self.status &= 0b1111_1110;
        }
    }
    fn and(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.register_a &= self.memory[address as usize];
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn asl(&mut self, mode: &AddressingMode) {
        let value = match mode {
            AddressingMode::NoneAddressing => self.register_a,
            _ => {
                let address = self.get_operand_address(mode);
                self.mem_read(address)
            }
        };

        let carry = value & 0b1000_0000;
        let result = value << 1;

        match mode {
            AddressingMode::NoneAddressing => self.register_a = result,
            _ => {
                let address = self.get_operand_address(mode);
                self.mem_write(address, result);
            }
        }
        if carry != 0 {
            self.status |= 0b0000_0001;
        } else {
            self.status &= 0b1111_1110;
        }
        self.update_zero_and_negative_flags(result);
    }
    fn bcc(&mut self) {
        if self.mem_read(self.program_counter) == 0 {
            return;
        }
        if self.status & 0b0000_0001 == 0 {
            self.program_counter += (self.mem_read(self.program_counter) - 1) as u16;
        }
    }
    fn bcs(&mut self) {
        if self.mem_read(self.program_counter) == 0 {
            return;
        }
        if self.status & 0b0000_0001 != 0 {
            self.program_counter += (self.mem_read(self.program_counter) - 1) as u16;
        }
    }
    fn beq(&mut self) {
        if self.mem_read(self.program_counter) == 0 {
            return;
        }
        if self.status & 0b0000_0010 != 0 {
            self.program_counter += (self.mem_read(self.program_counter) - 1) as u16;
        }
    }
    fn lda(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.register_a = self.mem_read(address);
        self.update_zero_and_negative_flags(self.register_a);
    }
    fn ldx(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.register_x = self.mem_read(address);
        self.update_zero_and_negative_flags(self.register_x);
    }
    fn ldy(&mut self, mode: &AddressingMode) {
        let address = self.get_operand_address(mode);
        self.register_y = self.mem_read(address);
        self.update_zero_and_negative_flags(self.register_y);
    }
    fn sec(&mut self) {
        self.status |= 0b0000_0001;
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
