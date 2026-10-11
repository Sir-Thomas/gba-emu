#![allow(dead_code)]

const SOFTWARE_INTERRUPT_ADDRESS: u32 = 0x0000_0008;

pub use crate::cpu::decode::Instruction;
use crate::{
    bus::Bus,
    cpu::alu::AluResult,
    cpu::decode::{ArmInstruction, ThumbInstruction},
    cpu::registers::{CpuMode, InstructionSet, Registers},
};

mod alu;
mod decode;
mod registers;

enum DataProcessingOperation {
    And,                      // 0000
    Xor,                      // 0001
    Subtract,                 // 0010
    ReverseSubtract,          // 0011
    Add,                      // 0100
    AddWithCarry,             // 0101
    SubtractWithCarry,        // 0110
    ReverseSubtractWithCarry, // 0111
    TestAnd,                  // 1000
    TestXor,                  // 1001
    TestSubtract,             // 1010
    TestAdd,                  // 1011
    Or,                       // 1100
    Move,                     // 1101
    BitClear,                 // 1110
    MoveNot,                  // 1111
}

impl From<u32> for DataProcessingOperation {
    fn from(value: u32) -> DataProcessingOperation {
        match value & 0x0F {
            0x0 => DataProcessingOperation::And,
            0x1 => DataProcessingOperation::Xor,
            0x2 => DataProcessingOperation::Subtract,
            0x3 => DataProcessingOperation::ReverseSubtract,
            0x4 => DataProcessingOperation::Add,
            0x5 => DataProcessingOperation::AddWithCarry,
            0x6 => DataProcessingOperation::SubtractWithCarry,
            0x7 => DataProcessingOperation::ReverseSubtractWithCarry,
            0x8 => DataProcessingOperation::TestAnd,
            0x9 => DataProcessingOperation::TestXor,
            0xA => DataProcessingOperation::TestSubtract,
            0xB => DataProcessingOperation::TestAdd,
            0xC => DataProcessingOperation::Or,
            0xD => DataProcessingOperation::Move,
            0xE => DataProcessingOperation::BitClear,
            0xF => DataProcessingOperation::MoveNot,
            _ => unreachable!("Data Processing: Invalid Operation {value:#03X}"),
        }
    }
}

#[derive(Default)]
pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self::default();
        cpu.registers.set_program_counter(0x0800_0000);
        cpu.registers.set_stack_pointer(0x03007F00);
        cpu
    }

    pub fn thumb() -> Self {
        let mut cpu = Self::default();
        cpu.registers.set_state(InstructionSet::Thumb);
        cpu
    }

    pub const fn negative(&self) -> bool {
        self.registers.negative()
    }

    pub const fn zero(&self) -> bool {
        self.registers.zero()
    }

    pub const fn carry(&self) -> bool {
        self.registers.carry()
    }

    pub const fn overflow(&self) -> bool {
        self.registers.overflow()
    }

    pub const fn program_counter(&self) -> u32 {
        self.registers.program_counter
    }

    pub const fn link_register(&self) -> u32 {
        self.registers.link_register
    }

    pub const fn stack_pointer(&self) -> u32 {
        self.registers.stack_pointer
    }

    pub fn next_instruction(&self, bus: &Bus) -> (u32, Instruction) {
        if self.registers.state() == InstructionSet::Arm {
            let opcode = bus.read_32(self.registers.program_counter as usize);
            let instruction = decode::arm_instruction(opcode);
            (opcode, Instruction::Arm(instruction))
        } else {
            let opcode = bus.read_16(self.registers.program_counter as usize);
            let instruction = decode::thumb_instruction(opcode);
            (u32::from(opcode), Instruction::Thumb(instruction))
        }
    }

    pub fn cpu_cycle(&mut self, bus: &mut Bus) {
        match self.registers.state() {
            InstructionSet::Arm => self.arm_cycle(bus),
            InstructionSet::Thumb => self.thumb_cycle(bus),
        }
    }

    pub fn arm_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_32(self.registers.program_counter as usize);
        self.registers.program_counter = self.registers.program_counter.wrapping_add(4);
        if self.registers.check_conditions(conditions(opcode)) {
            let instruction = decode::arm_instruction(opcode);
            self.run_arm_instruction(instruction, opcode, bus);
        }
    }

    fn run_arm_instruction(&mut self, instruction: ArmInstruction, opcode: u32, bus: &mut Bus) {
        match instruction {
            ArmInstruction::DataProcessing => self.data_processing(opcode),
            ArmInstruction::PsrTransferMRS => self.psr_transfer_mrs(opcode),
            ArmInstruction::PsrTransferMSR => self.psr_transfer_msr(opcode),
            ArmInstruction::Multiply => self.multiply(opcode),
            ArmInstruction::MultiplyLong => self.multiply_long(opcode),
            ArmInstruction::SingleDataSwap => self.single_data_swap(opcode),
            ArmInstruction::BranchAndExchange => self.branch_and_exchange(opcode),
            ArmInstruction::HalfwordDataTransferRegisterOffset => {
                self.halfword_data_transfer_register_offset(opcode, bus)
            }
            ArmInstruction::HalfwordDataTransferImmediateOffset => {
                self.halfword_data_transfer_immediate_offset(opcode, bus)
            }
            ArmInstruction::SingleDataTransfer => self.single_data_transfer(opcode, bus),
            ArmInstruction::Undefined => unreachable!(),
            ArmInstruction::BlockDataTransfer => self.block_data_transfer(opcode, bus),
            ArmInstruction::Branch => self.branch(opcode),
            ArmInstruction::CoprocessorDataTransfer => self.coprocessor_data_transfer(opcode),
            ArmInstruction::CoprocessorDataOperation => self.coprocessor_data_operation(opcode),
            ArmInstruction::CoprocessorRegisterTransfer => {
                self.coprocessor_register_transfer(opcode)
            }
            ArmInstruction::SoftwareInterrupt => self.arm_software_interrupt(),
        }
    }

    fn shift(&mut self, operand: u32) -> AluResult {
        const IMMEDIATE_VALUE_MASK: u32 = 0x0000_0F80;
        const IMMEDIATE_VALUE_SHIFT: usize = 7;
        const SHIFT_REGISTER_MASK: u32 = 0x0000_0F00;
        const SHIFT_REGISTER_SHIFT: usize = 8;
        const SHIFT_MASK: u32 = 0x0000_0060;
        const SHIFT_SHIFT: usize = 5;
        const IMMEDIATE_VALUE_BIT: u32 = 1 << 4;
        const SOURCE_REGISTER_MASK: u32 = 0x0000_000F;
        let shift_type = (operand & SHIFT_MASK) >> SHIFT_SHIFT;
        let immediate = operand & IMMEDIATE_VALUE_BIT == 0;
        let shift = if immediate {
            (operand & IMMEDIATE_VALUE_MASK) >> IMMEDIATE_VALUE_SHIFT
        } else {
            let register = (operand & SHIFT_REGISTER_MASK) >> SHIFT_REGISTER_SHIFT;
            self.registers.get(register.truncate()) & 0x0000_00FF
        };
        let source_register = (operand & SOURCE_REGISTER_MASK).truncate();
        match (shift_type, immediate && shift == 0) {
            (0b00, _) => alu::shift_left(self.registers.get(source_register), shift),
            (0b01, true) => alu::shift_right(self.registers.get(source_register), 32),
            (0b01, false) => alu::shift_right(self.registers.get(source_register), shift),
            (0b10, true) => alu::arithmetic_shift_right(self.registers.get(source_register), 32),
            (0b10, false) => {
                alu::arithmetic_shift_right(self.registers.get(source_register), shift)
            }
            (0b11, true) => {
                alu::rotate_right_carry(self.registers.get(source_register), self.registers.carry())
            }
            (0b11, false) => alu::rotate_right(self.registers.get(source_register), shift),
            _ => unreachable!("Invalid shift type: {shift_type:#06X}"),
        }
    }

    fn parse_second_operand(&mut self, immediate: bool, operand: u32) -> AluResult {
        const ROTATE_MASK: u32 = 0x0000_0F00;
        const ROTATE_SHIFT: usize = 8;
        const IMMEDIATE_MASK: u32 = 0x0000_00FF;
        if immediate {
            let rotate = ((operand & ROTATE_MASK) >> ROTATE_SHIFT).wrapping_mul(2);
            let value = operand & IMMEDIATE_MASK;
            alu::rotate_right(value, rotate)
        } else {
            self.shift(operand)
        }
    }

    fn data_processing(&mut self, opcode: u32) {
        const IMMEDIATE_VALUE_BIT: u32 = 1 << 25;
        const OPCODE_MASK: u32 = 0x01E0_0000;
        const OPCODE_SHIFT: usize = 21;
        const SET_CONDITIONS_BIT: u32 = 1 << 20;
        const FIRST_OPERAND_REGISTER_MASK: u32 = 0x000F_0000;
        const FIRST_OPERAND_REGISTER_SHIFT: usize = 16;
        const DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
        const DESTINATION_REGISTER_SHIFT: usize = 12;
        const SECOND_OPERAND_MASK: u32 = 0x0000_0FFF;
        let immediate = opcode & IMMEDIATE_VALUE_BIT > 0;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let set_conditions = opcode & SET_CONDITIONS_BIT > 0;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let first_operand_register =
            (opcode & FIRST_OPERAND_REGISTER_MASK) >> FIRST_OPERAND_REGISTER_SHIFT;
        let first_operand = self.registers.get(first_operand_register.truncate());
        let alu_result = self.parse_second_operand(immediate, opcode & SECOND_OPERAND_MASK);
        let second_operand = alu_result.result;
        if set_conditions {
            self.registers.status_registers.set_conditions(alu_result);
        }
        let operation = DataProcessingOperation::from(sub_opcode);
        let carry_flag = u32::from(self.registers.carry());
        let result = match operation {
            DataProcessingOperation::And => alu::and(first_operand, second_operand),
            DataProcessingOperation::Xor => alu::xor(first_operand, second_operand),
            DataProcessingOperation::Subtract => alu::subtract(first_operand, second_operand),
            DataProcessingOperation::ReverseSubtract => {
                alu::subtract(second_operand, first_operand)
            }
            DataProcessingOperation::Add => alu::add(first_operand, second_operand),
            DataProcessingOperation::AddWithCarry => {
                alu::add_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::SubtractWithCarry => {
                alu::sub_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::ReverseSubtractWithCarry => {
                alu::sub_carry(second_operand, first_operand, carry_flag)
            }
            DataProcessingOperation::TestAnd => alu::and(first_operand, second_operand),
            DataProcessingOperation::TestXor => alu::xor(first_operand, second_operand),
            DataProcessingOperation::TestSubtract => alu::subtract(first_operand, second_operand),
            DataProcessingOperation::TestAdd => alu::add(first_operand, second_operand),
            DataProcessingOperation::Or => alu::or(first_operand, second_operand),
            DataProcessingOperation::Move => alu::nop(second_operand),
            DataProcessingOperation::BitClear => alu::and(first_operand, !second_operand),
            DataProcessingOperation::MoveNot => alu::nop(!second_operand),
        };
        if set_conditions {
            self.registers.status_registers.set_conditions(result);
        }
        if !matches!(
            operation,
            DataProcessingOperation::TestAnd
                | DataProcessingOperation::TestXor
                | DataProcessingOperation::TestSubtract
                | DataProcessingOperation::TestAdd
        ) {
            self.registers
                .set(destination_register.truncate(), result.result);
        }
    }

    fn psr_transfer_mrs(&mut self, opcode: u32) {
        const SOURCE_PSR_BIT: u32 = 1 << 22;
        const DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
        const DESTINATION_REGISTER_SHIFT: usize = 12;
        let saved = opcode & SOURCE_PSR_BIT > 0;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        if !saved {
            self.registers
                .set(destination_register.truncate(), self.registers.current());
            return;
        }
        let value = self.registers.status_registers.get();
        self.registers.set(destination_register.truncate(), value);
    }

    fn psr_transfer_msr(&mut self, opcode: u32) {
        const IMMEDIATE_VALUE_BIT: u32 = 1 << 25;
        const SOURCE_PSR_BIT: u32 = 1 << 22;
        const MODE_INCLUDED_BIT: u32 = 1 << 16;
        const ROTATE_MASK: u32 = 0x0000_0F00;
        const ROTATE_SHIFT: usize = 8;
        const IMMEDIATE_VALUE_MASK: u32 = 0x0000_00FF;
        const REGISTER_MASK: u32 = 0x0000_000F;
        let immediate = opcode & IMMEDIATE_VALUE_BIT > 0;
        let mode_included = opcode & MODE_INCLUDED_BIT > 0;
        let saved = opcode & SOURCE_PSR_BIT > 0;
        let mut value = if immediate {
            let rotate = (opcode & ROTATE_MASK) >> ROTATE_SHIFT;
            (opcode & IMMEDIATE_VALUE_MASK).rotate_right(rotate.wrapping_mul(2))
        } else {
            let register = opcode & REGISTER_MASK;
            self.registers.get(register.truncate())
        };
        if !mode_included {
            value = (value & !0b11111) | (self.registers.current() & 0b11111);
        }
        if !saved {
            self.registers.status_registers.set_current(value);
            return;
        }
        self.registers.status_registers.save(value);
    }

    fn multiply(&mut self, opcode: u32) {
        const ACCUMULATE: u32 = 1 << 21;
        const SET_CONDITION_CODE: u32 = 1 << 20;
        const DESTINATION_REGISTER_MASK: u32 = 0x000F_0000;
        const DESTINATION_REGISTER_SHIFT: usize = 16;
        const SOURCE_REGISTER_N_MASK: u32 = 0x0000_F000;
        const SOURCE_REGISTER_N_SHIFT: usize = 12;
        const SOURCE_REGISTER_S_MASK: u32 = 0x0000_0F00;
        const SOURCE_REGISTER_S_SHIFT: usize = 8;
        const SOURCE_REGISTER_M_MASK: u32 = 0x0000_000F;
        let accumulate = opcode & ACCUMULATE > 0;
        let set_condition_code = opcode & SET_CONDITION_CODE > 0;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let source_register_n =
            ((opcode & SOURCE_REGISTER_N_MASK) >> SOURCE_REGISTER_N_SHIFT).truncate();
        let source_register_s =
            ((opcode & SOURCE_REGISTER_S_MASK) >> SOURCE_REGISTER_S_SHIFT).truncate();
        let source_register_m = (opcode & SOURCE_REGISTER_M_MASK).truncate();
        let value = if accumulate {
            self.registers
                .get(source_register_m)
                .wrapping_mul(self.registers.get(source_register_s))
                .wrapping_add(self.registers.get(source_register_n))
        } else {
            self.registers
                .get(source_register_m)
                .wrapping_mul(self.registers.get(source_register_s))
        };
        self.registers.set(destination_register.truncate(), value);
        if set_condition_code {
            self.registers.set_zero(value == 0);
            self.registers.set_negative(value & 0x8000_0000 > 0);
        }
    }

    fn multiply_long(&self, opcode: u32) {
        todo!("MultiplyLong {opcode:#010X}");
    }

    fn single_data_swap(&self, opcode: u32) {
        todo!("SingleDataSwap {opcode:#010X}");
    }

    fn branch_and_exchange(&mut self, opcode: u32) {
        const REGISTER_MASK: u32 = 0x0000_000F;
        const THUMB_MODE_BIT: u32 = 1;
        let register = opcode & REGISTER_MASK;
        let address = self.registers.get(register.truncate());
        let thumb = address & THUMB_MODE_BIT > 0;
        if thumb {
            self.registers.set_state(InstructionSet::Thumb);
            self.registers.program_counter = address & !1;
        } else {
            self.registers.set_state(InstructionSet::Arm);
            self.registers.program_counter = address & !3;
        }
    }

    fn halfword_data_transfer_register_offset(&mut self, opcode: u32, bus: &mut Bus) {
        const PRE_POST_INDEXING_BIT: u32 = 1 << 24;
        const UP_DOWN_BIT: u32 = 1 << 23;
        const WRITE_BACK_BIT: u32 = 1 << 21;
        const LOAD_STORE_BIT: u32 = 1 << 20;
        const BASE_REGISTER_MASK: u32 = 0x000F_0000;
        const BASE_REGISTER_SHIFT: usize = 16;
        const SOURCE_DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
        const SOURCE_DESTINATION_REGISTER_SHIFT: usize = 12;
        const SIGNED_BIT: u32 = 1 << 6;
        const HALFWORD_BIT: u32 = 1 << 5;
        const OFFSET_REGISTER_MASK: u32 = 0x0000_000F;
        let pre_index = opcode & PRE_POST_INDEXING_BIT > 0;
        let up = opcode & UP_DOWN_BIT > 0;
        let write_back = opcode & WRITE_BACK_BIT > 0;
        let load = opcode & LOAD_STORE_BIT > 0;
        let base_register = ((opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT).truncate();
        let source_destination_register = ((opcode & SOURCE_DESTINATION_REGISTER_MASK)
            >> SOURCE_DESTINATION_REGISTER_SHIFT)
            .truncate();
        let signed = opcode & SIGNED_BIT > 0;
        let halfword = opcode & HALFWORD_BIT > 0;
        let offset_register = opcode & OFFSET_REGISTER_MASK;
        let offset = self.registers.get(offset_register.truncate());
        let address = match (pre_index, up) {
            (true, true) => self.registers.get(base_register).wrapping_add(offset),
            (true, false) => self.registers.get(base_register).wrapping_sub(offset),
            (false, _) => self.registers.get(base_register),
        };
        match (write_back, pre_index, up) {
            // docs say post-index always writes back
            (false, true, _) => {}
            (_, false, false) => self
                .registers
                .set(base_register, address.wrapping_sub(offset)),
            (_, false, true) => self
                .registers
                .set(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.registers.set(base_register, address),
        }
        match (halfword, load, signed) {
            (true, true, true) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_16(address as usize).into(), 16),
            ),
            (true, true, false) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_16(address as usize)),
            ),
            (true, false, false) => bus.write_16(
                address as usize,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, true, true) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_8(address as usize).into(), 8),
            ),
            (false, true, false) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (false, false, false) => bus.write_8(
                address as usize,
                self.registers.get(source_destination_register).truncate(),
            ),
            _ => unreachable!("Sign extend should not be used with store"),
        }
    }

    fn halfword_data_transfer_immediate_offset(&mut self, opcode: u32, bus: &mut Bus) {
        const PRE_POST_INDEXING_BIT: u32 = 1 << 24;
        const UP_DOWN_BIT: u32 = 1 << 23;
        const WRITE_BACK_BIT: u32 = 1 << 21;
        const LOAD_STORE_BIT: u32 = 1 << 20;
        const BASE_REGISTER_MASK: u32 = 0x000F_0000;
        const BASE_REGISTER_SHIFT: usize = 16;
        const SOURCE_DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
        const SOURCE_DESTINATION_REGISTER_SHIFT: usize = 12;
        const IMMEDIATE_OFFSET_HIGH_MASK: u32 = 0x0000_0F00;
        const IMMEDIATE_OFFSET_HIGH_SHIFT: usize = 4; // leave it in the high nibble of 8 bit value
        const SIGNED_BIT: u32 = 1 << 6;
        const HALFWORD_BIT: u32 = 1 << 5;
        const IMMEDIATE_OFFSET_LOW_MASK: u32 = 0x0000_000F;
        let pre_index = opcode & PRE_POST_INDEXING_BIT > 0;
        let up = opcode & UP_DOWN_BIT > 0;
        let write_back = opcode & WRITE_BACK_BIT > 0;
        let load = opcode & LOAD_STORE_BIT > 0;
        let base_register = ((opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT).truncate();
        let source_destination_register = ((opcode & SOURCE_DESTINATION_REGISTER_MASK)
            >> SOURCE_DESTINATION_REGISTER_SHIFT)
            .truncate();
        let signed = opcode & SIGNED_BIT > 0;
        let halfword = opcode & HALFWORD_BIT > 0;
        let offset = ((opcode & IMMEDIATE_OFFSET_HIGH_MASK) >> IMMEDIATE_OFFSET_HIGH_SHIFT)
            | (opcode & IMMEDIATE_OFFSET_LOW_MASK);
        let address = match (pre_index, up) {
            (true, true) => self.registers.get(base_register).wrapping_add(offset),
            (true, false) => self.registers.get(base_register).wrapping_sub(offset),
            (false, _) => self.registers.get(base_register),
        };
        match (write_back, pre_index, up) {
            (false, true, _) => {}
            (_, false, false) => self
                .registers
                .set(base_register, address.wrapping_sub(offset)),
            (_, false, true) => self
                .registers
                .set(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.registers.set(base_register, address),
        }
        match (halfword, load, signed) {
            (true, true, true) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_16(address as usize).into(), 16),
            ),
            (true, true, false) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_16(address as usize)),
            ),
            (true, false, false) => bus.write_16(
                address as usize,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, true, true) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_8(address as usize).into(), 8),
            ),
            (false, true, false) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (false, false, false) => bus.write_8(
                address as usize,
                self.registers.get(source_destination_register).truncate(),
            ),
            _ => unreachable!("Sign extend should not be used with store"),
        }
    }

    fn single_data_transfer(&mut self, opcode: u32, bus: &mut Bus) {
        const IMMEDIATE_OFFSET_BIT: u32 = 1 << 25;
        const PRE_POST_INDEXING_BIT: u32 = 1 << 24;
        const UP_DOWN_BIT: u32 = 1 << 23;
        const BYTE_WORD_BIT: u32 = 1 << 22;
        const WRITE_BACK_BIT: u32 = 1 << 21;
        const LOAD_STORE_BIT: u32 = 1 << 20;
        const BASE_REGISTER_MASK: u32 = 0x000F_0000;
        const BASE_REGISTER_SHIFT: usize = 16;
        const SOURCE_DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
        const SOURCE_DESTINATION_REGISTER_SHIFT: usize = 12;
        const IMMEDIATE_OFFSET_MASK: u32 = 0x0000_0FFF;
        const SHIFT_MASK: u32 = 0x0000_0FF0;
        const SHIFT_SHIFT: usize = 4;
        const REGISTER_MASK: u32 = 0x0000_000F;
        let immediate = opcode & IMMEDIATE_OFFSET_BIT == 0;
        let pre_index = opcode & PRE_POST_INDEXING_BIT > 0;
        let up = opcode & UP_DOWN_BIT > 0;
        let byte = opcode & BYTE_WORD_BIT > 0;
        let write_back = opcode & WRITE_BACK_BIT > 0;
        let load = opcode & LOAD_STORE_BIT > 0;
        let base_register = ((opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT).truncate();
        let source_destination_register = ((opcode & SOURCE_DESTINATION_REGISTER_MASK)
            >> SOURCE_DESTINATION_REGISTER_SHIFT)
            .truncate();
        let offset = if immediate {
            opcode & IMMEDIATE_OFFSET_MASK
        } else {
            self.shift(opcode & IMMEDIATE_OFFSET_MASK).result
        };
        let address = match (pre_index, up) {
            (true, true) => self.registers.get(base_register).wrapping_add(offset),
            (true, false) => self.registers.get(base_register).wrapping_sub(offset),
            (false, _) => self.registers.get(base_register),
        };
        match (write_back, pre_index, up) {
            (false, true, _) => {}
            (_, false, false) => self
                .registers
                .set(base_register, address.wrapping_sub(offset)),
            (_, false, true) => self
                .registers
                .set(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.registers.set(base_register, address),
        }
        match (byte, load) {
            (true, true) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (true, false) => bus.write_8(
                address as usize,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, true) => self
                .registers
                .set(source_destination_register, bus.read_32(address as usize)),
            (false, false) => bus.write_32(
                address as usize,
                self.registers.get(source_destination_register),
            ),
        }
    }

    fn block_data_transfer(&mut self, opcode: u32, bus: &mut Bus) {
        const PRE_POST_INDEXING_BIT: u32 = 1 << 24;
        const UP_DOWN_BIT: u32 = 1 << 23;
        const PSR_FORCE_USER_BIT: u32 = 1 << 22;
        const WRITE_BACK_BIT: u32 = 1 << 21;
        const LOAD_STORE_BIT: u32 = 1 << 20;
        const BASE_REGISTER_MASK: u32 = 0x000F_0000;
        const BASE_REGISTER_SHIFT: usize = 16;
        const REGISTER_LIST_MASK: u32 = 0x0000_FFFF;
        let pre_index = opcode & PRE_POST_INDEXING_BIT > 0;
        let up = opcode & UP_DOWN_BIT > 0;
        let psr = opcode & PSR_FORCE_USER_BIT > 0;
        if psr {
            unimplemented!("ARM: Block Data Transfer - PSR & force user bit set");
        }
        let write_back = opcode & WRITE_BACK_BIT > 0;
        let load = opcode & LOAD_STORE_BIT > 0;
        let base_register = ((opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT).truncate();
        let register_list = opcode & REGISTER_LIST_MASK;
        let base_value = self.registers.get(base_register);
        let byte_count = register_list.count_ones().wrapping_mul(4);
        let mut address = match (up, pre_index) {
            (true, true) => base_value.wrapping_add(4),
            (true, false) => base_value,
            (false, true) => base_value.wrapping_sub(byte_count),
            (false, false) => base_value.wrapping_sub(byte_count).wrapping_add(4),
        };
        for register in (0..=15).filter(|r| register_list & (1 << r) > 0) {
            if load {
                self.registers.set(register, bus.read_32(address as usize));
            } else {
                bus.write_32(address as usize, self.registers.get(register));
            }
            address = address.wrapping_add(4);
        }
        if write_back {
            let final_address = if up {
                base_value.wrapping_add(byte_count)
            } else {
                base_value.wrapping_sub(byte_count)
            };
            self.registers.set(base_register, final_address);
        }
    }

    fn branch(&mut self, opcode: u32) {
        const LINK_MASK: u32 = 0x0100_0000;
        const OFFSET_MASK: u32 = 0x00FF_FFFF;
        let link = opcode & LINK_MASK > 0;
        if link {
            self.registers.link_register = self.registers.program_counter;
        }
        let offset = sign_extend((opcode & OFFSET_MASK) << 2, 26);
        // + 4 for prefetch
        self.registers.program_counter = self
            .registers
            .get(15)
            .wrapping_add_signed(offset.cast_signed());
    }

    fn coprocessor_data_transfer(&self, opcode: u32) {
        todo!("CoprocessorDataTransfer {opcode:#010X}");
    }

    fn coprocessor_data_operation(&self, opcode: u32) {
        todo!("CoprocessorDataOperation {opcode:#010X}");
    }

    fn coprocessor_register_transfer(&self, opcode: u32) {
        todo!("CoprocessorRegisterTransfer {opcode:#010X}");
    }

    fn arm_software_interrupt(&mut self) {
        self.registers.link_register = self.registers.program_counter;
        self.registers
            .status_registers
            .set_supervisor(self.registers.current());
        self.registers.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.registers.set_mode(CpuMode::Supervisor);
    }

    pub fn thumb_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_16(self.registers.program_counter as usize);
        let instruction = decode::thumb_instruction(opcode);
        self.registers.program_counter = self.registers.program_counter.wrapping_add(2);
        self.run_thumb_instruction(instruction, opcode, bus);
    }

    fn run_thumb_instruction(&mut self, instruction: ThumbInstruction, opcode: u16, bus: &mut Bus) {
        match instruction {
            ThumbInstruction::SoftwareInterrupt => self.thumb_software_interrupt(),
            ThumbInstruction::UnconditionalBranch => self.unconditional_branch(opcode),
            ThumbInstruction::ConditionalBranch => self.conditional_branch(opcode),
            ThumbInstruction::MultipleLoadstore => self.multiple_loadstore(opcode, bus),
            ThumbInstruction::LongBranchWithLink => self.long_branch_with_link(opcode),
            ThumbInstruction::AddOffsetToStackPointer => self.add_offset_to_stack_pointer(opcode),
            ThumbInstruction::PushPopRegisters => self.push_pop_registers(opcode, bus),
            ThumbInstruction::LoadStoreHalfword => self.load_store_halfword(opcode, bus),
            ThumbInstruction::SPRelativeLoadStore => self.sp_relative_load_store(opcode, bus),
            ThumbInstruction::LoadAddress => self.load_address(opcode),
            ThumbInstruction::LoadStoreWithImmediateOffset => {
                self.load_store_with_immediate_offset(opcode, bus);
            }
            ThumbInstruction::LoadStoreWithRegisterOffset => {
                self.load_store_with_register_offset(opcode, bus);
            }
            ThumbInstruction::LoadStoreSignExtendedByteHalfword => {
                self.load_store_sign_extended_byte_halfword(opcode, bus);
            }
            ThumbInstruction::PCRelativeLoad => self.pc_relative_load(opcode, bus),
            ThumbInstruction::HiRegisterOperationsBranchExchange => {
                self.hi_register_operations_branch_exchange(opcode);
            }
            ThumbInstruction::AluOperations => self.alu_operations(opcode),
            ThumbInstruction::MoveCompareAddSubtractImmediate => {
                self.move_compare_add_subtract_immediate(opcode);
            }
            ThumbInstruction::AddSubtract => self.add_subtract(opcode),
            ThumbInstruction::MoveShiftedRegister => self.move_shifted_register(opcode),
            ThumbInstruction::Unimplemented => {
                println!(
                    "Location: {:#010X}, Callback: {:#010X}",
                    self.registers.program_counter, self.registers.link_register
                );
                unreachable!("Unimplemented Thumb opcode: {opcode:#06X}")
            }
        }
    }

    fn thumb_software_interrupt(&mut self) {
        self.registers.link_register = self.registers.program_counter;
        self.registers
            .status_registers
            .set_supervisor(self.registers.current());
        self.registers.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.registers.set_state(InstructionSet::Arm);
        self.registers.set_mode(CpuMode::Supervisor);
    }

    fn unconditional_branch(&mut self, opcode: u16) {
        // Offset is a 12 bit value, but is stored as 11 bits (lsb is dropped) because it must be halfword aligned
        const MASK: u16 = 0x07FF;
        let offset = sign_extend(((opcode & MASK) << 1).into(), 12);
        self.registers.program_counter = self
            .registers
            .get(15)
            .wrapping_add_signed(offset.cast_signed());
    }

    fn conditional_branch(&mut self, opcode: u16) {
        const CONDITIONS_MASK: u16 = 0x0F00;
        const CONDITIONS_SHIFT: usize = 8;
        const SIGNED_OFFSET_MASK: u16 = 0x00FF;
        let conditions = (opcode & CONDITIONS_MASK) >> CONDITIONS_SHIFT;
        let offset = ((opcode & SIGNED_OFFSET_MASK) as i8) << 1;
        let branch = self.registers.check_conditions(conditions.truncate());
        if branch {
            self.registers.program_counter = self
                .registers
                .get(15)
                .wrapping_add_signed(i32::from(offset));
        }
    }

    fn multiple_loadstore(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const BASE_REGISTER_MASK: u16 = 0x0700;
        const BASE_REGISTER_SHIFT: usize = 8;
        const REGISTER_LIST_MASK: u16 = 0x00FF;
        let load = opcode & LOAD_STORE_MASK > 0;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let register_list = opcode & REGISTER_LIST_MASK;
        if load {
            self.multiple_load(base_register, register_list, bus);
        } else {
            self.multiple_store(base_register, register_list, bus);
        }
    }

    fn multiple_load(&mut self, base_register: u16, register_list: u16, bus: &Bus) {
        let mut address = self.registers.get(base_register) as usize;
        for register in (0..=7).filter(|r| register_list & (1 << r) > 0) {
            self.registers.set(register, bus.read_32(address));
            address = address.wrapping_add(4);
        }
        self.registers.set(base_register, address as u32);
    }

    fn multiple_store(&mut self, base_register: u16, register_list: u16, bus: &mut Bus) {
        let mut address = self.registers.get(base_register) as usize;
        for register in (0..=7).filter(|r| register_list & (1 << r) > 0) {
            bus.write_32(address, self.registers.get(register));
            address = address.wrapping_add(4);
        }
        self.registers.set(base_register, address as u32);
    }

    fn long_branch_with_link(&mut self, opcode: u16) {
        const H_MASK: u16 = 0x0800;
        const OFFSET_MASK: u16 = 0x07FF;
        let h = opcode & H_MASK > 0;
        let offset = opcode & OFFSET_MASK;
        if !h {
            self.registers.link_register = self
                .registers
                .get(15)
                .wrapping_add(sign_extend((offset << 1).into(), 12) << 11);
        } else {
            let return_address = self.registers.program_counter;
            self.registers.link_register = self
                .registers
                .link_register
                .wrapping_add(u32::from(offset << 1));
            self.registers.program_counter = self.registers.link_register;
            self.registers.link_register = return_address | 1;
        }
    }

    fn add_offset_to_stack_pointer(&mut self, opcode: u16) {
        const SIGN_FLAG_MASK: u16 = 0x0080;
        const SIGN_FLAG_SHIFT: usize = 7;
        const SIGNED_OFFSET_MASK: u16 = 0x007F;
        const SIGNED_OFFSET_SHIFT: usize = 2;
        let sign = (opcode & SIGN_FLAG_MASK) >> SIGN_FLAG_SHIFT > 0;
        let word = ((opcode & SIGNED_OFFSET_MASK) << SIGNED_OFFSET_SHIFT).cast_signed();
        if sign {
            self.registers.stack_pointer = self
                .registers
                .stack_pointer
                .wrapping_sub_signed(i32::from(word));
        } else {
            self.registers.stack_pointer = self
                .registers
                .stack_pointer
                .wrapping_add_signed(i32::from(word));
        }
    }

    fn push_pop_registers(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const PC_LR_MASK: u16 = 0x0100;
        const PC_LR_SHIFT: usize = 8;
        const REGISTER_LIST_MASK: u16 = 0x00FF;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT > 0;
        let pc_lr = (opcode & PC_LR_MASK) >> PC_LR_SHIFT > 0;
        let register_list = opcode & REGISTER_LIST_MASK;
        if load {
            self.pop_registers(register_list, pc_lr, bus);
        } else {
            self.push_registers(register_list, pc_lr, bus);
        }
    }

    fn pop_registers(&mut self, register_list: u16, pc_lr: bool, bus: &Bus) {
        for register in (0..=7).filter(|r| register_list & (1 << r) > 0) {
            let val = self.pop(bus);
            self.registers.set(register, val);
        }
        if pc_lr {
            self.registers.program_counter = self.pop(bus);
        }
    }

    fn push_registers(&mut self, register_list: u16, pc_lr: bool, bus: &mut Bus) {
        if pc_lr {
            self.push(self.registers.link_register, bus);
        }
        for register in (0..=7).rev().filter(|r| register_list & (1 << r) > 0) {
            self.push(self.registers.get(register), bus);
        }
    }

    fn pop(&mut self, bus: &Bus) -> u32 {
        let value = bus.read_32(self.registers.stack_pointer as usize);
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add(4);
        value
    }

    fn push(&mut self, value: u32, bus: &mut Bus) {
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub(4);
        bus.write_32(self.registers.stack_pointer as usize, value);
    }

    fn load_store_halfword(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        // 6 bit immediate offset stored as 5 bits
        const OFFSET_IMMEDIATE_MASK: u16 = 0x07C0;
        const OFFSET_IMMEDIATE_SHIFT: usize = 4;
        const BASE_REGISTER_MASK: u16 = 0x0038;
        const BASE_REGISTER_SHIFT: usize = 3;
        const SOURCE_DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT > 0;
        let offset = (opcode & OFFSET_IMMEDIATE_MASK) >> OFFSET_IMMEDIATE_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self
            .registers
            .get(base_register)
            .wrapping_add(u32::from(offset)) as usize;
        if load {
            self.registers
                .set(source_destination_register, u32::from(bus.read_16(address)));
        } else {
            bus.write_16(
                address,
                self.registers.get(source_destination_register).truncate(),
            );
        }
    }

    fn sp_relative_load_store(&mut self, opcode: u16, bus: &mut Bus) {
        const LOAD_STORE_MASK: u16 = 0x0800;
        const LOAD_STORE_SHIFT: usize = 11;
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const OFFSET_MASK: u16 = 0x00FF;
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT > 0;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let offset = u32::from((opcode & OFFSET_MASK) << 2);
        if load {
            self.registers.set(
                destination_register,
                bus.read_32(self.registers.stack_pointer.wrapping_add(offset) as usize),
            );
        } else {
            bus.write_32(
                self.registers.stack_pointer.wrapping_add(offset) as usize,
                self.registers.get(destination_register),
            );
        }
    }

    fn load_address(&mut self, opcode: u16) {
        const SOURCE_MASK: u16 = 0x0800;
        const SOURCE_SHIFT: usize = 11;
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const WORD_MASK: u16 = 0x00FF;
        const WORD_SHIFT: usize = 2;
        let source = (opcode & SOURCE_MASK) >> SOURCE_SHIFT > 0;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let word = u32::from((opcode & WORD_MASK) << WORD_SHIFT);
        let address = if source {
            self.registers.stack_pointer.wrapping_add(word)
        } else {
            self.registers.get(15).wrapping_add(word) & !3
        };
        self.registers.set(destination_register, address);
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
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT > 0;
        let byte = (opcode & BYTE_WORD_MASK) >> BYTE_WORD_SHIFT > 0;
        let offset = (opcode & OFFSET_IMMEDIATE_MASK) >> OFFSET_IMMEDIATE_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let byte_address = self
            .registers
            .get(base_register)
            .wrapping_add(u32::from(offset)) as usize;
        let word_address = self
            .registers
            .get(base_register)
            .wrapping_add(u32::from(offset << 2)) as usize;
        match (load, byte) {
            (true, true) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_8(byte_address)),
            ),
            (true, false) => self
                .registers
                .set(source_destination_register, bus.read_32(word_address)),
            (false, true) => bus.write_8(
                byte_address,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, false) => {
                bus.write_32(
                    word_address,
                    self.registers.get(source_destination_register),
                );
            }
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
        let load = (opcode & LOAD_STORE_MASK) >> LOAD_STORE_SHIFT > 0;
        let byte = (opcode & BYTE_WORD_MASK) >> BYTE_WORD_SHIFT > 0;
        let offset_register = (opcode & OFFSET_REGISTER_MASK) >> OFFSET_REGISTER_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self
            .registers
            .get(base_register)
            .wrapping_add(self.registers.get(offset_register)) as usize;
        match (load, byte) {
            (true, true) => self
                .registers
                .set(source_destination_register, u32::from(bus.read_8(address))),
            (true, false) => self
                .registers
                .set(source_destination_register, bus.read_32(address)),
            (false, true) => bus.write_8(
                address,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, false) => {
                bus.write_32(address, self.registers.get(source_destination_register))
            }
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
        let h_flag = (opcode & H_FLAG_MASK) >> H_FLAG_SHIFT > 0;
        let sign_extend_flag = (opcode & SIGN_EXTEND_FLAG_MASK) >> SIGN_EXTEND_FLAG_SHIFT > 0;
        let offset_register = (opcode & OFFSET_REGISTER_MASK) >> OFFSET_REGISTER_SHIFT;
        let base_register = (opcode & BASE_REGISTER_MASK) >> BASE_REGISTER_SHIFT;
        let source_destination_register = opcode & SOURCE_DESTINATION_REGISTER_MASK;
        let address = self
            .registers
            .get(base_register)
            .wrapping_add(self.registers.get(offset_register)) as usize;
        match (sign_extend_flag, h_flag) {
            (false, false) => bus.write_16(
                address,
                self.registers.get(source_destination_register).truncate(),
            ),
            (false, true) => {
                self.registers
                    .set(source_destination_register, u32::from(bus.read_16(address)));
            }
            (true, false) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_8(address).into(), 8),
            ),
            (true, true) => self.registers.set(
                source_destination_register,
                sign_extend(bus.read_16(address).into(), 16),
            ),
        }
    }

    fn pc_relative_load(&mut self, opcode: u16, bus: &Bus) {
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const OFFSET_MASK: u16 = 0x00FF;
        const OFFSET_SHIFT: usize = 2;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let offset = (opcode & OFFSET_MASK) << OFFSET_SHIFT; // offset is 10 bit word aligned, so
        // it is stored as 8 bits
        // we have to clear lowest bit of program counter
        let value =
            bus.read_32((self.registers.get(15) & !3).wrapping_add(u32::from(offset)) as usize);
        self.registers.set(destination_register, value);
    }

    fn hi_register_operations_branch_exchange(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x0300;
        const OPCODE_SHIFT: usize = 8;
        const H1_MASK: u16 = 1 << 7;
        const H2_MASK: u16 = 1 << 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let hi1 = opcode & H1_MASK > 0;
        let hi2 = opcode & H2_MASK > 0;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        match sub_opcode {
            0b00 => self.hi_register_add(hi1, hi2, source_register, destination_register),
            0b01 => self.hi_register_cmp(hi1, hi2, source_register, destination_register),
            0b10 => self.hi_register_mov(hi1, hi2, source_register, destination_register),
            0b11 => self.hi_register_bx(hi2, source_register),
            _ => unreachable!(),
        }
    }

    fn hi_register_add(
        &mut self,
        h1: bool,
        h2: bool,
        source_register: u16,
        destination_register: u16,
    ) {
        let dest = (u16::from(h1) << 3) | destination_register;
        let src = (u16::from(h2) << 3) | source_register;
        let value = self
            .registers
            .get(dest)
            .wrapping_add(self.registers.get(src));
        self.registers.set(dest, value);
    }

    fn hi_register_cmp(
        &mut self,
        h1: bool,
        h2: bool,
        source_register: u16,
        destination_register: u16,
    ) {
        let dest = (u16::from(h1) << 3) | destination_register;
        let src = (u16::from(h2) << 3) | source_register;
        let result = alu::subtract(self.registers.get(dest), self.registers.get(src));
        self.registers.status_registers.set_conditions(result);
    }

    fn hi_register_mov(
        &mut self,
        h1: bool,
        h2: bool,
        source_register: u16,
        destination_register: u16,
    ) {
        let dest = (u16::from(h1) << 3) | destination_register;
        let src = (u16::from(h2) << 3) | source_register;
        self.registers.set(dest, self.registers.get(src));
    }

    fn hi_register_bx(&mut self, h: bool, register: u16) {
        let r = (u16::from(h) << 3) | register;
        let mode = if self.registers.get(r) & 0x01 > 0 {
            InstructionSet::Thumb
        } else {
            InstructionSet::Arm
        };
        self.registers.set_state(mode);
        self.registers.program_counter = self.registers.get(r) & !1;
    }

    fn alu_operations(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x03C0;
        const OPCODE_SHIFT: usize = 6;
        const SOURCE_REGISTER_MASK: u16 = 0x0038;
        const SOURCE_REGISTER_SHIFT: usize = 3;
        const DESTINATION_REGISTER_MASK: u16 = 0x0007;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        let alu_result = match sub_opcode {
            0x00 => alu::and(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x01 => alu::xor(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x02 => alu::shift_left(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x03 => alu::shift_right(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x04 => alu::arithmetic_shift_right(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x05 => alu::add_carry(
                self.registers.get(source_register),
                self.registers.get(destination_register),
                self.registers.carry().into(),
            ),
            0x06 => alu::sub_carry(
                self.registers.get(source_register),
                self.registers.get(destination_register),
                self.registers.carry().into(),
            ),
            0x07 => alu::rotate_right(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x08 => alu::and(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x09 => alu::subtract(0, self.registers.get(source_register)),
            0x0A => alu::subtract(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x0B => alu::add(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x0C => alu::or(
                self.registers.get(destination_register),
                self.registers.get(source_register),
            ),
            0x0D => AluResult {
                result: self
                    .registers
                    .get(destination_register)
                    .wrapping_mul(self.registers.get(source_register)),
                carry: None,
                overflow: None,
            },
            0x0E => alu::and(
                self.registers.get(destination_register),
                !self.registers.get(source_register),
            ),
            0x0F => alu::nop(!self.registers.get(source_register)),
            _ => unreachable!(),
        };
        self.registers.status_registers.set_conditions(alu_result);
        if !matches!(sub_opcode, 0x08 | 0x0A | 0x0B) {
            self.registers.set(destination_register, alu_result.result);
        }
    }

    fn move_compare_add_subtract_immediate(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x1800;
        const OPCODE_SHIFT: usize = 11;
        const DESTINATION_REGISTER_MASK: u16 = 0x0700;
        const DESTINATION_REGISTER_SHIFT: usize = 8;
        const IMMEDIATE_VALUE_MASK: u16 = 0x00FF;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT;
        let destination_register =
            (opcode & DESTINATION_REGISTER_MASK) >> DESTINATION_REGISTER_SHIFT;
        let immediate_value = opcode & IMMEDIATE_VALUE_MASK;
        let result = match sub_opcode {
            0b00 => alu::nop(u32::from(immediate_value)),
            0b01 => alu::subtract(
                self.registers.get(destination_register),
                u32::from(immediate_value),
            ),
            0b10 => alu::add(
                self.registers.get(destination_register),
                u32::from(immediate_value),
            ),
            0b11 => alu::subtract(
                self.registers.get(destination_register),
                u32::from(immediate_value),
            ),
            _ => unreachable!(),
        };
        self.registers.status_registers.set_conditions(result);
        if sub_opcode == 0b01 {
            return;
        }
        self.registers.set(destination_register, result.result);
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
        let i = (opcode & I_MASK) >> I_SHIFT > 0;
        let sub_opcode = (opcode & OPCODE_MASK) >> OPCODE_SHIFT > 0;
        let r3 = (opcode & R3_MASK) >> R3_SHIFT;
        let immediate_value = (opcode & IMMEDIATE_VALUE_MASK) >> IMMEDIATE_VALUE_SHIFT;
        let source_register = (opcode & SOURCE_REGISTER_MASK) >> SOURCE_REGISTER_SHIFT;
        let destination_register = opcode & DESTINATION_REGISTER_MASK;
        let result = match (sub_opcode, i) {
            (false, false) => alu::add(self.registers.get(source_register), self.registers.get(r3)),
            (false, true) => alu::add(
                self.registers.get(source_register),
                u32::from(immediate_value),
            ),
            (true, false) => {
                alu::subtract(self.registers.get(source_register), self.registers.get(r3))
            }
            (true, true) => alu::subtract(
                self.registers.get(source_register),
                u32::from(immediate_value),
            ),
        };
        self.registers.status_registers.set_conditions(result);
        self.registers.set(destination_register, result.result);
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
        // Not sure about this, but ai says to do it. Need to find the correct documentation.
        //if immediate_value == 0 {
        //immediate_value = 32;
        //}
        let result = match sub_opcode {
            0b00 => alu::shift_left(self.registers.get(source_register), immediate_value.into()),
            0b01 => alu::shift_right(self.registers.get(source_register), immediate_value.into()),
            0b10 => alu::arithmetic_shift_right(
                self.registers.get(source_register),
                immediate_value.into(),
            ),
            _ => unreachable!(),
        };
        self.registers.status_registers.set_conditions(result);
        self.registers.set(destination_register, result.result);
    }
}

fn conditions(opcode: u32) -> u8 {
    const MASK: u32 = 0xF000_0000;
    const SHIFT: usize = 28;
    ((opcode & MASK) >> SHIFT).truncate()
}

fn sign_extend(value: u32, size: u32) -> u32 {
    ((value << (32 - size)).cast_signed() >> (32 - size)).cast_unsigned()
}

/*
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_shifted_register_sets_register() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.registers.set(0, 0xFFFF_FFFF);
        bus.write_16(cpu.program_counter() as usize, 0x0001);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0xFFFF_FFFF);
    }

    #[test]
    fn move_shifted_register_left_shift() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x0040);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0x0000_0002);
    }

    #[test]
    fn move_shifted_register_right_shift() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0002;
        bus.write_16(0, 0x0840);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0x0000_0001);
    }

    #[test]
    fn move_shifted_register_arithmetic_shift() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x8000_0002;
        bus.write_16(0, 0x1040);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r00, 0xC000_0001);
    }

    #[test]
    fn add_subtract_add_register() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        cpu.r01 = 0x0000_0001;
        bus.write_16(0, 0x1842);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r02, 0x0000_0002);
    }

    #[test]
    fn add_subtract_add_immediate() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x1C41);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0x0000_0002);
    }

    #[test]
    fn add_subtract_sub_register() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        cpu.r01 = 0x0000_0001;
        bus.write_16(0, 0x1A42);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r02, 0x0000_0000);
    }

    #[test]
    fn add_subtract_sub_immediate() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0x0000_0001;
        bus.write_16(0, 0x1E41);
        cpu.cpu_cycle(&mut bus);
        assert_eq!(cpu.r01, 0x0000_0000);
    }
}
*/
