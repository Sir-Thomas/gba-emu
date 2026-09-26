#![allow(dead_code)]

use crate::{bus::Bus, program_status_register::ProgramStatusRegister};

#[derive(Debug)]
pub enum ThumbInstruction {
    SoftwareInterrupt,
    UnconditionalBranch,
    ConditionalBranch,
    MultipleLoadstore,
    LongBranchWithLink,
    AddOffsetToStackPointer,
    PushPopRegisters,
    LoadStoreHalfword,
    SPRelativeLoadStore,
    LoadAddress,
    LoadStoreWithImmediateOffset,
    LoadStoreWithRegisterOffset,
    LoadStoreSignExtendedByteHalfword,
    PCRelativeLoad,
    HiRegisterOperationsBranchExchange,
    ALUOperations,
    MoveCompareAddSubtractImmediate,
    AddSubtract,
    MoveShiftedRegister,
    Unimplemented,
}

#[derive(Default)]
pub struct Cpu {
    r00: u32,
    r01: u32,
    r02: u32,
    r03: u32,
    r04: u32,
    r05: u32,
    r06: u32,
    r07: u32,
    r08: u32,
    r09: u32,
    r10: u32,
    r11: u32,
    r12: u32,
    stack_pointer: u32,
    link_register: u32,
    program_counter: u32,
    program_status_register: ProgramStatusRegister,
}

impl Cpu {
    //temp
    pub fn get_r00(&self) -> u32 {
        self.r00
    }
    
    //temp
    pub fn get_r01(&self) -> u32 {
        self.r01
    }

    //temp
    pub fn set_r00(&mut self, value: u32) {
        self.r00 = value;
    }

    pub fn get_program_counter(&self) -> u32 {
        self.program_counter
    }

    pub fn get_next_instruction(&self, bus: &Bus) -> (u16, ThumbInstruction) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_instruction(opcode);
        (opcode, instruction)
    }

    pub fn cpu_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_instruction(opcode);
        self.program_counter = self.program_counter.wrapping_add(2);
        self.run_instruction(instruction, opcode, bus);
    }

    fn run_instruction(&mut self, instruction: ThumbInstruction, opcode: u16, bus: &mut Bus) {
        match instruction {
            ThumbInstruction::SoftwareInterrupt => self.software_interrupt(opcode),
            ThumbInstruction::UnconditionalBranch => self.unconditional_branch(opcode),
            ThumbInstruction::ConditionalBranch => self.conditional_branch(opcode),
            ThumbInstruction::MultipleLoadstore => self.multiple_loadstore(opcode, bus),
            ThumbInstruction::LongBranchWithLink => self.long_branch_with_link(opcode),
            ThumbInstruction::AddOffsetToStackPointer => self.add_offset_to_stack_pointer(opcode),
            ThumbInstruction::PushPopRegisters => self.push_pop_registers(opcode, bus),
            ThumbInstruction::LoadStoreHalfword => self.load_store_halfword(opcode, bus),
            ThumbInstruction::SPRelativeLoadStore => self.sp_relative_load_store(opcode, bus),
            ThumbInstruction::LoadAddress => self.load_address(opcode, bus),
            ThumbInstruction::LoadStoreWithImmediateOffset => self.load_store_with_immediate_offset(opcode, bus),
            ThumbInstruction::LoadStoreWithRegisterOffset => self.load_store_with_register_offset(opcode, bus),
            ThumbInstruction::LoadStoreSignExtendedByteHalfword => self.load_store_sign_extended_byte_halfword(opcode, bus),
            ThumbInstruction::PCRelativeLoad => self.pc_relative_load(opcode, bus),
            ThumbInstruction::HiRegisterOperationsBranchExchange => self.hi_register_operations_branch_exchange(opcode),
            ThumbInstruction::ALUOperations => self.alu_operations(opcode),
            ThumbInstruction::MoveCompareAddSubtractImmediate => self.move_compare_add_subtract_immediate(opcode),
            ThumbInstruction::AddSubtract => self.add_subtract(opcode),
            ThumbInstruction::MoveShiftedRegister => self.move_shifted_register(opcode),
            ThumbInstruction::Unimplemented => unimplemented!(),
        }
    }

    fn software_interrupt(&mut self, opcode: u16) {
        let _value = opcode & 0x00FF;
        // todo!(); // This moves to ARM mode, I'll save it for later
    }

    fn unconditional_branch(&mut self, opcode: u16) {
        // Offset is a 12 bit value, but is stored as 11 bits (lsb is dropped) because it must be halfword aligned
        const MASK: u16 = 0x07FF;
        let offset = (opcode & MASK << 1) as i32;
        self.program_counter = self.program_counter.wrapping_add_signed(offset);
    }

    fn conditional_branch(&mut self, _opcode: u16) {
        todo!();
    }

    fn multiple_loadstore(&mut self, _opcode: u16, _bus: &mut Bus) {
        todo!();
    }

    fn long_branch_with_link(&mut self, _opcode: u16) {
        todo!();
    }

    fn add_offset_to_stack_pointer(&mut self, opcode: u16) {
        const SIGN_FLAG_MASK: u16 = 0x0080;
        const SIGN_FLAG_SHIFT: usize = 7;
        const WORD_MASK: u16 = 0x007F;
        const WORD_SHIFT: usize = 2;
        let sign = (opcode & SIGN_FLAG_MASK) >> SIGN_FLAG_SHIFT == 0x01;
        let word = (opcode & WORD_MASK) << WORD_SHIFT;
        if sign {
            self.stack_pointer = self.stack_pointer.wrapping_sub(u32::from(word));
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_add(u32::from(word));
        }
    }

    fn push_pop_registers(&mut self, opcode: u16, _bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const PC_LR_MASK: u16 = 0x0100;
        const PC_LR_SHIFT: usize = 8;
        const R_LIST_MASK: u16 = 0x00FF;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT == 0x01;
        let pc_lr = (opcode & PC_LR_MASK) >> PC_LR_SHIFT == 0x01;
        let r_list = opcode & R_LIST_MASK;
        if load {
            self.pop_registers(r_list, pc_lr);
        } else {
            self.push_registers(r_list, pc_lr);
        }
    }

    fn pop_registers(&mut self, r_list: u16, pc_lr: bool) {
        if pc_lr {
            self.pop_register(15);
        }
        if r_list & 0x01 == 0x01 {
            self.pop_register(0);
        }
        if r_list & 0x02 == 0x02 {
            self.pop_register(1);
        }
        if r_list & 0x04 == 0x04 {
            self.pop_register(2);
        }
        if r_list & 0x08 == 0x08 {
            self.pop_register(3);
        }
        if r_list & 0x10 == 0x10 {
            self.pop_register(4);
        }
        if r_list & 0x20 == 0x20 {
            self.pop_register(5);
        }
        if r_list & 0x40 == 0x40 {
            self.pop_register(6);
        }
        if r_list & 0x80 == 0x80 {
            self.pop_register(7);
        }
    }

    fn push_registers(&mut self, r_list: u16, pc_lr: bool) {
        if pc_lr {
            self.push_register(14);
        }
        if r_list & 0x01 == 0x01 {
            self.push_register(0);
        }
        if r_list & 0x02 == 0x02 {
            self.push_register(1);
        }
        if r_list & 0x04 == 0x04 {
            self.push_register(2);
        }
        if r_list & 0x08 == 0x08 {
            self.push_register(3);
        }
        if r_list & 0x10 == 0x10 {
            self.push_register(4);
        }
        if r_list & 0x20 == 0x20 {
            self.push_register(5);
        }
        if r_list & 0x40 == 0x40 {
            self.push_register(6);
        }
        if r_list & 0x80 == 0x80 {
            self.push_register(7);
        }
    }

    fn pop_register(&mut self, _register: u8) {
        todo!();
    }

    fn push_register(&mut self, _register: u8) {
        todo!();
    }

    fn load_store_halfword(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const OFFSET_IMMEDIATE_MASK: u16 = 0x07C0;
        const OFFSET_IMMEDIATE_SHIFT: usize = 5;
        const BASE_REGISTER_MASK: u16 = 0x0038;
        const BASE_REGISTER_SHIFT: usize = 3;
        const SOURCE_DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT == 0x01;
        // 6 bit immediate offset stored as 5 bits
        let offset = (opcode & OFFSET_IMMEDIATE_MASK) >> OFFSET_IMMEDIATE_SHIFT << 1;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self.get_register(base_register).wrapping_add(offset as u32) as usize;
        if load {
            self.store(u32::from(bus.read_16(address)), source_destination_register);
        } else {
            bus.write_16(address, self.get_register(source_destination_register).truncate());
        }
    }

    fn sp_relative_load_store(&mut self, _opcode: u16, _bus: &mut Bus) {
        todo!();
    }

    fn load_address(&mut self, opcode: u16, bus: &Bus) {
        const SOURCE_MASK: u16 = 0x0800;
        const SOURCE_SHIFT: usize = 11;
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const WORD_MASK: u16 = 0x00FF;
        const WORD_SHIFT: usize = 2;
        let source = (opcode & SOURCE_MASK) >> SOURCE_SHIFT == 0x01;
        let destination_register = (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let word = (opcode & WORD_MASK) << WORD_SHIFT;
        let address = if source {
            self.stack_pointer.wrapping_add(word as u32)
        } else {
            self.program_counter.wrapping_add(word as u32)
        };
        self.store(bus.read_32(address as usize), destination_register);
    }

    fn load_store_with_immediate_offset(&mut self, opcode: u16, bus: &mut Bus) {
        const BYTE_WORD_MASK: u16 = 0x1000;
        const BYTE_WORD_SHIFT: usize = 12;
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const OFFSET_IMMEDIATE_MASK: u16 = 0x07C0;
        const OFFSET_IMMEDIATE_SHIFT: usize = 6;
        const BASE_REGISTER_MASK: u16 = 0x0038;
        const BASE_REGISTER_SHIFT: usize = 3;
        const SOURCE_DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT == 0x01;
        let byte = (opcode & BYTE_WORD_MASK) >> BYTE_WORD_SHIFT == 0x01;
        let offset = (opcode & OFFSET_IMMEDIATE_MASK) >> OFFSET_IMMEDIATE_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let byte_address = self.get_register(base_register).wrapping_add(offset as u32) as usize;
        let word_address = self.get_register(base_register).wrapping_add((offset << 2) as u32) as usize;
        match (load, byte) {
            (true, true) => self.store(u32::from(bus.read_8(byte_address)), source_destination_register),
            (true, false) => self.store(bus.read_32(word_address), source_destination_register),
            (false, true) => bus.write_8(byte_address, self.get_register(source_destination_register).truncate()),
            (false, false) => bus.write_32(word_address, self.get_register(source_destination_register)),
        }
    }

    fn load_store_with_register_offset(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const BYTE_WORD_MASK: u16 = 0x0400;
        const BYTE_WORD_SHIFT: usize = 10;
        const OFFSET_REGISTER_MASK: u16 = 0x01C0;
        const OFFSET_REGISTER_SHIFT: usize = 6;
        const BASE_REGISTER_MASK: u16 = 0x0038;
        const BASE_REGISTER_SHIFT: usize = 3;
        const SOURCE_DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT == 0x01;
        let byte = (opcode & BYTE_WORD_MASK) >> BYTE_WORD_SHIFT == 0x01;
        let offset_register = (opcode & OFFSET_REGISTER_MASK) >> OFFSET_REGISTER_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self.get_register(base_register).wrapping_add(self.get_register(offset_register)) as usize;
        match (load, byte) {
            (true, true) => self.store(u32::from(bus.read_8(address)), source_destination_register),
            (true, false) => self.store(bus.read_32(address), source_destination_register),
            (false, true) => bus.write_8(address, self.get_register(source_destination_register).truncate()),
            (false, false) => bus.write_32(address, self.get_register(source_destination_register)),
        }
    }

    fn load_store_sign_extended_byte_halfword(&mut self, opcode: u16, bus: &mut Bus) {
        const H_FLAG_MASK: u16 = 0x0800;
        const H_FLAG_SHIFT: usize = 11;
        const SIGN_EXTEND_FLAG_MASK: u16 = 0x0400;
        const SIGN_EXTEND_FLAG_SHIFT: usize = 10;
        const OFFSET_REGISTER_MASK: u16 = 0x01C0;
        const OFFSET_REGISTER_SHIFT: usize = 6;
        const BASE_REGISTER_MASK: u16 = 0x0038;
        const BASE_REGISTER_SHIFT: usize = 3;
        const SOURCE_DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let h_flag = (opcode & H_FLAG_MASK) >> H_FLAG_SHIFT == 0x01;
        let sign_extend_flag = (opcode & SIGN_EXTEND_FLAG_MASK) >> SIGN_EXTEND_FLAG_SHIFT == 0x01;
        let offset_register = (opcode & OFFSET_REGISTER_MASK) >> OFFSET_REGISTER_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self.get_register(base_register).wrapping_add(self.get_register(offset_register)) as usize;
        match (sign_extend_flag, h_flag) {
            (false, false) => bus.write_16(address, self.get_register(source_destination_register).truncate()),
            (false, true) => self.store(u32::from(bus.read_16(address)), source_destination_register),
            (true, false) => self.store(bus.read_8(address).sign_extend(), source_destination_register),
            (true, true) => self.store(bus.read_16(address).sign_extend(), source_destination_register),
        }
    }

    fn pc_relative_load(&mut self, opcode: u16, bus: &Bus) {
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const OFFSET_MASK: u16 = 0x00FF;
        const OFFSET_SHIFT: usize = 2;
        let destination_register = (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let offset = (opcode & OFFSET_MASK) << OFFSET_SHIFT; // offset is 10 bit word aligned, so
                                                               // it is stored as 8 bits
        self.store(bus.read_32(self.program_counter.wrapping_add(u32::from(offset)) as usize), destination_register);
    }
 
    fn hi_register_operations_branch_exchange(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x0300;
        const OPCODE_SHIFT: usize = 8;
        const H1_MASK: u16 = 0x0080;
        const H1_SHIFT: usize = 7;
        const H2_MASK: u16 = 0x0040;
        const H2_SHIFT: usize = 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let h1 = (opcode & H1_MASK) >> H1_SHIFT;
        let h2 = (opcode & H2_MASK) >> H2_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        match sub_opcode {
            0b00 => self.hi_register_operations_branch_exchange_add(destination_register | h1 << 3, source_register | h2 << 3),
            0b01 => self.hi_register_operations_branch_exchange_cmp(destination_register | h1 << 3, source_register | h2 << 3),
            0b10 => self.hi_register_operations_branch_exchange_mov(destination_register | h1 << 3, source_register | h2 << 3),
            0b11 => self.hi_register_operations_branch_exchange_bx(source_register | h2 << 3),
            _ => unreachable!(),
        }
    }

    fn hi_register_operations_branch_exchange_add(&mut self, destination_register: u16, source_register: u16) {
        let value = self.get_register(destination_register).wrapping_add(self.get_register(source_register));
        self.store(value, destination_register);
    }

    fn hi_register_operations_branch_exchange_cmp(&mut self, destination_register: u16, source_register: u16) {
        self.test_sub(destination_register, source_register);
    }

    fn hi_register_operations_branch_exchange_mov(&mut self, destination_register: u16, source_register: u16) {
        self.store(self.get_register(source_register), destination_register);
    }

    fn hi_register_operations_branch_exchange_bx(&mut self, source_register: u16) {
        let value = self.get_register(source_register);
        self.program_counter = value & 0xFFFF_FFFE; // remove last bit for alignment
        if value & 0x01 == 0x00 { // last bit determines thumb vs arm mode
            todo!(); // switch to arm mode
        }
    }

    fn alu_operations(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x07C0;
        const OPCODE_SHIFT: usize = 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        let value = match sub_opcode {
            0x00 => self.get_register(destination_register) & self.get_register(source_register),
            0x01 => self.get_register(destination_register) | self.get_register(source_register),
            0x02 => self.get_register(destination_register) << self.get_register(source_register),
            0x03 => self.get_register(destination_register) >> self.get_register(source_register),
            0x04 => (self.get_register(destination_register) as i32 >> self.get_register(source_register)) as u32,
            0x05 => self.get_register(destination_register).wrapping_add(self.get_register(source_register)).wrapping_add(self.get_carry_flag()),
            0x06 => self.get_register(destination_register).wrapping_sub(self.get_register(source_register)).wrapping_sub(self.get_not_carry_flag()),
            0x07 => self.ror(source_register),
            0x08 => { self.test_and(destination_register, source_register); self.get_register(destination_register) },
            0x09 => 0u32.wrapping_sub(self.get_register(source_register)),
            0x0A => { self.test_sub(destination_register, source_register); self.get_register(destination_register) },
            0x0B => { self.test_add(destination_register, source_register); self.get_register(destination_register) },
            0x0C => self.get_register(destination_register) | self.get_register(source_register),
            0x0D => self.get_register(destination_register).wrapping_mul(self.get_register(source_register)),
            0x0E => self.get_register(destination_register) & !self.get_register(source_register),
            0x0F => !self.get_register(source_register),
            _ => unreachable!(),
        };
        self.store(value, destination_register);
    }

    fn move_compare_add_subtract_immediate(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x1800;
        const OPCODE_SHIFT: usize = 11;
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const IMMEDIATE_VALUE_MASK: u16 = 0x00FF;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let destination_register = (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let immediate_value = opcode & IMMEDIATE_VALUE_MASK;
        match sub_opcode {
            0b00 => self.store(u32::from(immediate_value), destination_register),
            0b01 => self.test_cmp(destination_register, u32::from(immediate_value)),
            0b10 => self.store(self.get_register(destination_register).wrapping_add(u32::from(immediate_value)), destination_register),
            0b11 => self.store(self.get_register(destination_register).wrapping_sub(u32::from(immediate_value)), destination_register),
            _ => unreachable!(),
        };
    }

    fn add_subtract(&mut self, opcode: u16) {
        const I_MASK: u16 = 0x0400;
        const I_SHIFT: usize = 10;
        const OPCODE_MASK: u16 = 0x0200;
        const OPCODE_SHIFT: usize = 9;
        const R3_MASK: u16 = 0x01C0;
        const R3_SHIFT: usize = 6;
        const IMMEDIATE_VALUE_MASK: u16 = 0x01C0;
        const IMMEDIATE_VALUE_SHIFT: usize = 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let i = (opcode & I_MASK) >> I_SHIFT == 1;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT == 1;
        let r3 = (opcode & R3_MASK) >> R3_SHIFT;
        let immediate_value = (opcode & IMMEDIATE_VALUE_MASK) >> IMMEDIATE_VALUE_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        let value = match (sub_opcode, i) {
            (false, false) => self.get_register(source_register).wrapping_add(self.get_register(r3)),
            (false, true) => self.get_register(source_register).wrapping_add(u32::from(immediate_value)),
            (true, false) => self.get_register(source_register).wrapping_sub(self.get_register(r3)),
            (true, true) => self.get_register(source_register).wrapping_sub(u32::from(immediate_value)),
        };
        self.store(value, destination_register);
    }

    fn move_shifted_register(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x1800;
        const OPCODE_SHIFT: usize = 11;
        const IMMEDIATE_VALUE_MASK: u16 = 0x07C0;
        const IMMEDIATE_VALUE_SHIFT: usize = 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let immediate_value = (opcode & IMMEDIATE_VALUE_MASK) >> IMMEDIATE_VALUE_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        let value = match sub_opcode {
            0b00 => self.get_register(source_register) << immediate_value,
            0b01 => self.get_register(source_register) >> immediate_value,
            // Arithmatic shift (signed right shift)
            0b10 => (self.get_register(source_register) as i32 >> immediate_value) as u32,
            _ => unreachable!(),
        };
        self.store(value, destination_register);
    }

    fn get_register(&self, register: u16) -> u32 {
        match register {
            0 => self.r00,
            1 => self.r01,
            2 => self.r02,
            3 => self.r03,
            4 => self.r04,
            5 => self.r05,
            6 => self.r06,
            7 => self.r07,
            8 => self.r08,
            9 => self.r09,
            10 => self.r10,
            11 => self.r11,
            12 => self.r12,
            13 => self.stack_pointer,
            14 => self.link_register,
            15 => self.program_counter,
            _ => unreachable!()
        }
    }

    fn store(&mut self, value: u32, register: u16) {
        match register {
            0 => self.r00 = value,
            1 => self.r01 = value,
            2 => self.r02 = value,
            3 => self.r03 = value,
            4 => self.r04 = value,
            5 => self.r05 = value,
            6 => self.r06 = value,
            7 => self.r07 = value,
            8 => self.r08 = value,
            9 => self.r09 = value,
            10 => self.r10 = value,
            11 => self.r11 = value,
            12 => self.r12 = value,
            13 => self.stack_pointer = value,
            14 => self.link_register = value,
            15 => self.program_counter = value,
            _ => unreachable!()
        }
    }

    fn get_carry_flag(&self) -> u32 {
        if self.program_status_register.get_carry() {
            1
        } else {
            0
        }
    }

    fn get_not_carry_flag(&self) -> u32 {
        if self.program_status_register.get_carry() {
            0
        } else {
            1
        }
    }

    fn ror(&mut self, source_register: u16) -> u32 {
        let temp = self.get_carry_flag();
        self.program_status_register.set_carry(self.get_register(source_register) & 0x01 == 0x01);
        let rotated = (self.get_register(source_register) >> 1) | (temp << 31);
        rotated
    }

    fn test_and(&mut self, source_register: u16, destination_register: u16) {
        let temp = self.get_register(source_register) & self.get_register(destination_register);
        self.program_status_register.set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
    }

    fn test_add(&mut self, source_register: u16, destination_register: u16) {
        let (temp, carry) = self.get_register(source_register).overflowing_add(self.get_register(destination_register));
        self.program_status_register.set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(carry);
        self.program_status_register.set_overflow(carry);
    }

    fn test_sub(&mut self, source_register: u16, destination_register: u16) {
        let (temp, overflow) = self.get_register(source_register).overflowing_sub(self.get_register(destination_register));
        self.program_status_register.set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(self.get_register(source_register) >= self.get_register(destination_register));
        self.program_status_register.set_overflow(overflow);
    }

    fn test_cmp(&mut self, source_register: u16, immediate: u32) {
        let (temp, overflow) = self.get_register(source_register).overflowing_sub(immediate);
        self.program_status_register.set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(self.get_register(source_register) >= immediate);
        self.program_status_register.set_overflow(overflow);
    }
}

fn decode_instruction(opcode: u16) -> ThumbInstruction {
    match () {
        _ if is_software_interrupt(opcode) => ThumbInstruction::SoftwareInterrupt,
        _ if is_unconditional_branch(opcode) => ThumbInstruction::UnconditionalBranch,
        _ if is_conditional_branch(opcode) => ThumbInstruction::ConditionalBranch,
        _ if is_multiple_loadstore(opcode) => ThumbInstruction::MultipleLoadstore,
        _ if is_long_branch_with_link(opcode) => ThumbInstruction::LongBranchWithLink,
        _ if is_add_offset_to_stack_pointer(opcode) => ThumbInstruction::AddOffsetToStackPointer,
        _ if is_push_pop_registers(opcode) => ThumbInstruction::PushPopRegisters,
        _ if is_load_store_halfword(opcode) => ThumbInstruction::LoadStoreHalfword,
        _ if is_sp_relative_load_store(opcode) => ThumbInstruction::SPRelativeLoadStore,
        _ if is_load_address(opcode) => ThumbInstruction::LoadAddress,
        _ if is_load_store_with_immediate_offset(opcode) => ThumbInstruction::LoadStoreWithImmediateOffset,
        _ if is_load_store_with_register_offset(opcode) => ThumbInstruction::LoadStoreWithRegisterOffset,
        _ if is_load_store_sign_extended_byte_halfword(opcode) => ThumbInstruction::LoadStoreSignExtendedByteHalfword,
        _ if is_pc_relative_load(opcode) => ThumbInstruction::PCRelativeLoad,
        _ if is_hi_register_operations_branch_exchange(opcode) => ThumbInstruction::HiRegisterOperationsBranchExchange,
        _ if is_alu_operations(opcode) => ThumbInstruction::ALUOperations,
        _ if is_move_compare_add_subtract_immediate(opcode) => ThumbInstruction::MoveCompareAddSubtractImmediate,
        _ if is_add_subtract(opcode) => ThumbInstruction::AddSubtract,
        _ if is_move_shifted_register(opcode) => ThumbInstruction::MoveShiftedRegister,
        _ => ThumbInstruction::Unimplemented,
    }
}

fn is_software_interrupt(opcode: u16) -> bool {
    const MASK: u16 = 0xFF00;
    const SOFTWARE_INTERRUPT: u16 = 0xDF00;
    opcode & MASK == SOFTWARE_INTERRUPT
}

fn is_unconditional_branch(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const UNCONDITIONAL_BRANCH: u16 = 0xE000;
    opcode & MASK == UNCONDITIONAL_BRANCH
}

fn is_conditional_branch(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const CONDITIONAL_BRANCH: u16 = 0xD000;
    opcode & MASK == CONDITIONAL_BRANCH
}

fn is_multiple_loadstore(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const MULTIPLE_LOADSTORE: u16 = 0xC000;
    opcode & MASK == MULTIPLE_LOADSTORE
}

fn is_long_branch_with_link(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LONG_BRANCH_WITH_LINK: u16 = 0xF000;
    opcode & MASK == LONG_BRANCH_WITH_LINK
}

fn is_add_offset_to_stack_pointer(opcode: u16) -> bool {
    const MASK: u16 = 0xFF00;
    const ADD_OFFSET_TO_STACK_POINTER: u16 = 0xB000;
    opcode & MASK == ADD_OFFSET_TO_STACK_POINTER
}

fn is_push_pop_registers(opcode: u16) -> bool {
    const MASK: u16 = 0xF600;
    const PUSH_POP_REGISTERS: u16 = 0xB600;
    opcode & MASK == PUSH_POP_REGISTERS
}

fn is_load_store_halfword(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LOAD_STORE_HALFWORD: u16 = 0x8000;
    opcode & MASK == LOAD_STORE_HALFWORD
}

fn is_sp_relative_load_store(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const SP_RELATIVE_LOAD_STORE: u16 = 0x9000;
    opcode & MASK == SP_RELATIVE_LOAD_STORE
}

fn is_load_address(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LOAD_ADDRESS: u16 = 0xA000;
    opcode & MASK == LOAD_ADDRESS
}

fn is_load_store_with_immediate_offset(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const LOAD_STORE_WITH_IMMEDIATE_OFFSET: u16 = 0x6000;
    opcode & MASK == LOAD_STORE_WITH_IMMEDIATE_OFFSET
}

fn is_load_store_with_register_offset(opcode: u16) -> bool {
    const MASK: u16 = 0xF200;
    const LOAD_STORE_WITH_REGISTER_OFFSET: u16 = 0x5000;
    opcode & MASK == LOAD_STORE_WITH_REGISTER_OFFSET
}

fn is_load_store_sign_extended_byte_halfword(opcode: u16) -> bool {
    const MASK: u16 = 0xF200;
    const LOAD_STORE_SIGN_EXTENDED_BYTE_HALFWORD: u16 = 0x5200;
    opcode & MASK == LOAD_STORE_SIGN_EXTENDED_BYTE_HALFWORD
}

fn is_pc_relative_load(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const PC_RELATIVE_LOAD: u16 = 0x4800;
    opcode & MASK == PC_RELATIVE_LOAD
}

fn is_hi_register_operations_branch_exchange(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE: u16 = 0x4400;
    opcode & MASK == HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE
}

fn is_alu_operations(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const ALU_OPERATIONS: u16 = 0x4000;
    opcode & MASK == ALU_OPERATIONS
}

fn is_move_compare_add_subtract_immediate(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const MOVE_COMPARE_ADD_SUBTRACT_IMMEDIATE: u16 = 0x2000;
    opcode & MASK == MOVE_COMPARE_ADD_SUBTRACT_IMMEDIATE
}

fn is_add_subtract(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const ADD_SUBTRACT: u16 = 0x1800;
    opcode & MASK == ADD_SUBTRACT
}

fn is_move_shifted_register(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const MOVE_SHIFTED_REGISTER: u16 = 0x0000;
    opcode & MASK == MOVE_SHIFTED_REGISTER
}

trait Extendable {
    fn sign_extend(self) -> u32;
}

impl Extendable for u8 {
    fn sign_extend(self) -> u32 {
        if self & 0x80 == 0x10 {
            0xFFFF_FF00 + self as u32
        } else {
            self as u32
        }
    }
}

impl Extendable for u16 {
    fn sign_extend(self) -> u32 {
        if self & 0x8000 == 0x1000 {
            0xFFFF_0000 + self as u32
        } else {
            self as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_shifted_register_sets_register() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0xFFFF_FFFF;
        bus.write_16(cpu.get_program_counter() as usize, 0x0001);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0xFFFF_FFFF);
    }

    #[test]
    fn move_shifted_register_left_shift() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x0040);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0x0000_0002);
    }

    #[test]
    fn move_shifted_register_right_shift() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0002;
        bus.write_16(0, 0x0840);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0x0000_0001);
    }

    #[test]
    fn move_shifted_register_arithmetic_shift() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x8000_0002;
        bus.write_16(0, 0x1040);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0xC000_0001);
    }

    #[test]
    fn add_subtract_add_register() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        cpu.r01 = 0x0000_0001;
        bus.write_16(0, 0x1842);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r02, 0x0000_0002);
    }

    #[test]
    fn add_subtract_add_immediate() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x1C41);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0x0000_0002);
    }

    #[test]
    fn add_subtract_sub_register() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        cpu.r01 = 0x0000_0001;
        bus.write_16(0, 0x1A42);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r02, 0x0000_0000);
    }

    #[test]
    fn add_subtract_sub_immediate() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x1E41);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0x0000_0000);
    }
}

