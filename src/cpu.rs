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

    pub fn cpu_cycle(&mut self, bus: &Bus) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_instruction(opcode);
        self.program_counter = self.program_counter.wrapping_add(2);
        self.run_instruction(instruction, opcode);
    }

    fn run_instruction(&mut self, instruction: ThumbInstruction, opcode: u16) {
        match instruction {
            ThumbInstruction::SoftwareInterrupt => self.software_interrupt(opcode),
            ThumbInstruction::UnconditionalBranch => self.unconditional_branch(opcode),
            ThumbInstruction::ALUOperations => self.alu_operations(opcode),
            ThumbInstruction::MoveCompareAddSubtractImmediate => self.move_compare_add_subtract_immediate(opcode),
            ThumbInstruction::AddSubtract => self.add_subtract(opcode),
            ThumbInstruction::MoveShiftedRegister => self.move_shifted_register(opcode),
            _ => unimplemented!()
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
            0b01 => _ = self.get_register(destination_register).wrapping_sub(u32::from(immediate_value)), // TODO: set flags
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
            0x00 => self.get_register(source_register) << immediate_value,
            0x01 => self.get_register(source_register) >> immediate_value,
            // Arithmatic shift (signed right shift)
            0x10 => (self.get_register(source_register) as i32 >> immediate_value) as u32,
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
        let rotated = (self.get_register(source_register) >> 1) & (temp << 31);
        rotated
    }

    fn test_and(&mut self, source_register: u16, destination_register: u16) {
        let temp = self.get_register(source_register) & self.get_register(destination_register);
        self.program_status_register.set_negative(temp & 0x8000 == 0x8000);
        self.program_status_register.set_zero(temp == 0x0000);
    }

    fn test_add(&mut self, source_register: u16, destination_register: u16) {
        let (temp, carry) = self.get_register(source_register).overflowing_add(self.get_register(destination_register));
        self.program_status_register.set_negative(temp & 0x8000 == 0x8000);
        self.program_status_register.set_zero(temp == 0x0000);
        self.program_status_register.set_carry(carry);
        self.program_status_register.set_overflow(carry);
    }

    fn test_sub(&mut self, source_register: u16, destination_register: u16) {
        let (temp, overflow) = self.get_register(source_register).overflowing_sub(self.get_register(destination_register));
        self.program_status_register.set_negative(temp & 0x8000 == 0x8000);
        self.program_status_register.set_zero(temp == 0x0000);
        self.program_status_register.set_carry(self.get_register(source_register) >= self.get_register(destination_register));
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
    const PC_RELATIVE_LOAD: u16 = 0x6800;
    opcode & MASK == PC_RELATIVE_LOAD
}

fn is_hi_register_operations_branch_exchange(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE: u16 = 0x6600;
    opcode & MASK == HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE
}

fn is_alu_operations(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const ALU_OPERATIONS: u16 = 0x6000;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mov_shifted_register_sets_register() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0xFFFF;
        bus.write_16(cpu.get_program_counter() as usize, 0x0001);
        cpu.cpu_cycle(&bus);
        assert_eq!(cpu.r01, 0xFFFF);
    }
}

