#![allow(dead_code)]

const SOFTWARE_INTERRUPT_ADDRESS: u32 = 0x0000_0008;

use crate::{
    bus::Bus,
    program_status_register::{CpuMode, Mode, SavedProgramStatusRegisters},
};

#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    Arm(ArmInstruction),
    Thumb(ThumbInstruction),
}

#[derive(Clone, Copy, Debug)]
pub enum ArmInstruction {
    DataProcessing,
    PsrTransferMRS,
    PsrTransferMSR,
    Multiply,
    MultiplyLong,
    SingleDataSwap,
    BranchAndExchange,
    HalfwordDataTransferRegisterOffset,
    HalfwordDataTransferImmediateOffset,
    SingleDataTransfer,
    Undefined,
    BlockDataTransfer,
    Branch,
    CoprocessorDataTransfer,
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer,
    SoftwareInterrupt,
}

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

#[derive(Clone, Copy, Debug)]
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
    program_status_register: SavedProgramStatusRegisters,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self::default();
        cpu.program_counter = 0x0800_0000;
        cpu.stack_pointer = 0x03007F00;
        cpu
    }

    pub fn thumb() -> Self {
        let mut cpu = Self::default();
        cpu.program_status_register.set_state(CpuMode::Thumb);
        cpu
    }

    pub const fn negative(&self) -> bool {
        self.program_status_register.negative()
    }

    pub const fn zero(&self) -> bool {
        self.program_status_register.zero()
    }

    pub const fn carry(&self) -> bool {
        self.program_status_register.carry()
    }

    pub const fn overflow(&self) -> bool {
        self.program_status_register.overflow()
    }

    pub const fn program_counter(&self) -> u32 {
        self.program_counter
    }

    pub const fn link_register(&self) -> u32 {
        self.link_register
    }

    pub const fn stack_pointer(&self) -> u32 {
        self.stack_pointer
    }

    pub fn next_instruction(&self, bus: &Bus) -> (u32, Instruction) {
        if self.program_status_register.state() == CpuMode::Arm {
            let opcode = bus.read_32(self.program_counter as usize);
            let instruction = decode_arm_instruction(opcode);
            (opcode, Instruction::Arm(instruction))
        } else {
            let opcode = bus.read_16(self.program_counter as usize);
            let instruction = decode_thumb_instruction(opcode);
            (u32::from(opcode), Instruction::Thumb(instruction))
        }
    }

    pub fn cpu_cycle(&mut self, bus: &mut Bus) {
        match self.program_status_register.state() {
            CpuMode::Arm => self.arm_cycle(bus),
            CpuMode::Thumb => self.thumb_cycle(bus),
        }
    }

    pub fn arm_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_32(self.program_counter as usize);
        self.program_counter = self.program_counter.wrapping_add(4);
        if self
            .program_status_register
            .check_conditions(conditions(opcode))
        {
            let instruction = decode_arm_instruction(opcode);
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

    fn set_conditions(&mut self, result: u32, carry: Option<bool>, overflow: Option<bool>) {
        self.program_status_register
            .set_negative(result & 0x8000_0000 > 0);
        self.program_status_register.set_zero(result == 0);
        if let Some(c) = carry {
            self.program_status_register.set_carry(c);
        }
        if let Some(v) = overflow {
            self.program_status_register.set_overflow(v);
        }
    }

    fn shift(&mut self, operand: u32) -> u32 {
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
            self.register(register.truncate()) & 0x0000_00FF
        };
        let source_register = (operand & SOURCE_REGISTER_MASK).truncate();
        let (value, carry, overflow) = match shift_type {
            0b00 => shift_left(self.register(source_register), shift),
            0b01 => shift_right(self.register(source_register), shift),
            0b10 => arithmetic_shift_right(self.register(source_register), shift),
            0b11 => rotate_right(self.register(source_register), shift),
            _ => unreachable!("Invalid shift type: {shift_type:#06X}"),
        };
        self.set_conditions(value, carry, overflow);
        value
    }

    fn parse_second_operand(&mut self, immediate: bool, operand: u32) -> u32 {
        const ROTATE_MASK: u32 = 0x0000_0F00;
        const ROTATE_SHIFT: usize = 8;
        const IMMEDIATE_MASK: u32 = 0x0000_00FF;
        if immediate {
            let rotate = ((operand & ROTATE_MASK) >> ROTATE_SHIFT).wrapping_mul(2);
            (operand & IMMEDIATE_MASK).rotate_right(rotate)
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
        let first_operand = self.register(first_operand_register.truncate());
        let second_operand = self.parse_second_operand(immediate, opcode & SECOND_OPERAND_MASK);
        let operation = DataProcessingOperation::from(sub_opcode);
        let carry_flag = u32::from(self.program_status_register.carry());
        let (value, carry, overflow) = match operation {
            DataProcessingOperation::And => (first_operand & second_operand, None, None),
            DataProcessingOperation::Xor => (first_operand ^ second_operand, None, None),
            DataProcessingOperation::Subtract => subtract(first_operand, second_operand),
            DataProcessingOperation::ReverseSubtract => subtract(second_operand, first_operand),
            DataProcessingOperation::Add => add(first_operand, second_operand),
            DataProcessingOperation::AddWithCarry => {
                add_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::SubtractWithCarry => {
                sub_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::ReverseSubtractWithCarry => {
                sub_carry(second_operand, first_operand, carry_flag)
            }
            DataProcessingOperation::TestAnd => (first_operand & second_operand, None, None),
            DataProcessingOperation::TestXor => (first_operand ^ second_operand, None, None),
            DataProcessingOperation::TestSubtract => subtract(first_operand, second_operand),
            DataProcessingOperation::TestAdd => add(first_operand, second_operand),
            DataProcessingOperation::Or => (first_operand | second_operand, None, None),
            DataProcessingOperation::Move => (second_operand, None, None),
            DataProcessingOperation::BitClear => (first_operand & !second_operand, None, None),
            DataProcessingOperation::MoveNot => (!second_operand, None, None),
        };
        if set_conditions {
            self.set_conditions(value, carry, overflow);
        }
        if !matches!(
            operation,
            DataProcessingOperation::TestAnd
                | DataProcessingOperation::TestXor
                | DataProcessingOperation::TestSubtract
                | DataProcessingOperation::TestAdd
        ) {
            self.set_register(destination_register.truncate(), value);
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
            self.set_register(
                destination_register.truncate(),
                self.program_status_register.current(),
            );
            return;
        }
        let value = match self.program_status_register.mode() {
            Mode::Fiq => self.program_status_register.fiq(),
            Mode::Irq => self.program_status_register.irq(),
            Mode::Abort => self.program_status_register.abort(),
            Mode::Undefined => self.program_status_register.undefined(),
            Mode::Supervisor => self.program_status_register.supervisor(),
            _ => unreachable!(
                "Invalid CPU mode for MRS: {:?}",
                self.program_status_register.mode()
            ),
        };
        self.set_register(destination_register.truncate(), value);
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
            self.register(register.truncate())
        };
        if !mode_included {
            value = (value & !0b11111) | (self.program_status_register.current() & 0b11111);
        }
        if !saved {
            self.program_status_register.set(value);
            return;
        }
        match self.program_status_register.mode() {
            Mode::Fiq => self.program_status_register.set_fiq(value),
            Mode::Irq => self.program_status_register.set_irq(value),
            Mode::Abort => self.program_status_register.set_abort(value),
            Mode::Undefined => self.program_status_register.set_undefined(value),
            Mode::Supervisor => self.program_status_register.set_supervisor(value),
            _ => unreachable!(
                "Invalid CPU mode for MSR: {:?}",
                self.program_status_register.mode()
            ),
        }
    }

    fn multiply(&mut self, opcode: u32) {
        const ACCUMULATE: u32 = 1 << 21;
        const SET_CONDITION_CODE: u32 = 1 << 20;
        const DESTINATION_REGISTER_MASK: u32 = 0x000F_0000;
        const DESTINATION_REGISTER_SHIFT: usize = 16;
        const SOURCE_REGISTER_N_MASK: u32 = 0x0000_F000;
        const SOURCE_REGISTER_N_SHIFT: usize = 12;
        const SOURCE_REGISTER_S_MASK: u32 = 0x0000_0F00;
        const SOURCE_REGISTER_S_SHIFT: usize = 12;
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
            self.register(source_register_m)
                .wrapping_mul(self.register(source_register_s))
                .wrapping_add(self.register(source_register_n))
        } else {
            self.register(source_register_m)
                .wrapping_mul(self.register(source_register_s))
        };
        self.set_register(destination_register.truncate(), value);
        if set_condition_code {
            self.program_status_register.set_zero(value == 0);
            self.program_status_register
                .set_negative(value & 0x8000_0000 > 0);
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
        let address = self.register(register.truncate());
        let thumb = address & THUMB_MODE_BIT > 0;
        if thumb {
            self.program_status_register.set_state(CpuMode::Thumb);
            self.program_counter = address & !1;
        } else {
            self.program_status_register.set_state(CpuMode::Arm);
            self.program_counter = address & !3;
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
        let offset = self.register(offset_register.truncate());
        let address = match (pre_index, up) {
            (true, true) => self.register(base_register).wrapping_add(offset),
            (true, false) => self.register(base_register).wrapping_sub(offset),
            (false, _) => self.register(base_register),
        };
        match (write_back, pre_index, up) {
            // docs say write back should only be used with pre-index
            (true, false, false) => self.set_register(base_register, address.wrapping_sub(offset)),
            (true, false, true) => self.set_register(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.set_register(base_register, address),
            (false, _, _) => {}
        }
        match (halfword, load, signed) {
            (true, true, true) => self.set_register(
                source_destination_register,
                bus.read_16(address as usize).sign_extend().cast_unsigned(),
            ),
            (true, true, false) => self.set_register(
                source_destination_register,
                u32::from(bus.read_16(address as usize)),
            ),
            (true, false, false) => bus.write_16(
                address as usize,
                self.register(source_destination_register).truncate(),
            ),
            (false, true, true) => self.set_register(
                source_destination_register,
                bus.read_8(address as usize).sign_extend().cast_unsigned(),
            ),
            (false, true, false) => self.set_register(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (false, false, false) => bus.write_8(
                address as usize,
                self.register(source_destination_register).truncate(),
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
            (true, true) => self.register(base_register).wrapping_add(offset),
            (true, false) => self.register(base_register).wrapping_sub(offset),
            (false, _) => self.register(base_register),
        };
        match (write_back, pre_index, up) {
            // docs say write back should only be used with pre-index
            (true, false, false) => self.set_register(base_register, address.wrapping_sub(offset)),
            (true, false, true) => self.set_register(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.set_register(base_register, address),
            (false, _, _) => {}
        }
        match (halfword, load, signed) {
            (true, true, true) => self.set_register(
                source_destination_register,
                bus.read_16(address as usize).sign_extend().cast_unsigned(),
            ),
            (true, true, false) => self.set_register(
                source_destination_register,
                u32::from(bus.read_16(address as usize)),
            ),
            (true, false, false) => bus.write_16(
                address as usize,
                self.register(source_destination_register).truncate(),
            ),
            (false, true, true) => self.set_register(
                source_destination_register,
                bus.read_8(address as usize).sign_extend().cast_unsigned(),
            ),
            (false, true, false) => self.set_register(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (false, false, false) => bus.write_8(
                address as usize,
                self.register(source_destination_register).truncate(),
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
            self.shift(opcode & IMMEDIATE_OFFSET_MASK)
        };
        let address = match (pre_index, up) {
            (true, true) => self.register(base_register).wrapping_add(offset),
            (true, false) => self.register(base_register).wrapping_sub(offset),
            (false, _) => self.register(base_register),
        };
        match (write_back, pre_index, up) {
            // docs say write back should only be used with pre-index
            (true, false, false) => self.set_register(base_register, address.wrapping_sub(offset)),
            (true, false, true) => self.set_register(base_register, address.wrapping_add(offset)),
            (true, true, _) => self.set_register(base_register, address),
            (false, _, _) => {}
        }
        match (byte, load) {
            (true, true) => self.set_register(
                source_destination_register,
                u32::from(bus.read_8(address as usize)),
            ),
            (true, false) => bus.write_8(
                address as usize,
                self.register(source_destination_register).truncate(),
            ),
            (false, true) => {
                self.set_register(source_destination_register, bus.read_32(address as usize))
            }
            (false, false) => {
                bus.write_32(address as usize, self.register(source_destination_register))
            }
        }
    }

    fn block_data_transfer(&mut self, opcode: u32, bus: &mut Bus) {
        // 1110 1001 0010 1101 0100 0000 0000 0011
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
        let base_value = self.register(base_register);
        let byte_count = register_list.count_ones().wrapping_mul(4);
        let mut address = match (up, pre_index) {
            (true, true) => base_value.wrapping_add(4),
            (true, false) => base_value,
            (false, true) => base_value.wrapping_sub(byte_count),
            (false, false) => base_value.wrapping_sub(byte_count).wrapping_add(4),
        };
        for register in (0..=15).filter(|r| register_list & (1 << r) > 0) {
            if load {
                self.set_register(register, bus.read_32(address as usize));
            } else {
                bus.write_32(address as usize, self.register(register));
            }
            address = address.wrapping_add(4);
        }
        if write_back {
            let final_address = if up {
                base_value.wrapping_add(byte_count)
            } else {
                base_value.wrapping_sub(byte_count)
            };
            self.set_register(base_register, final_address);
        }
    }

    fn branch(&mut self, opcode: u32) {
        const LINK_MASK: u32 = 0x0100_0000;
        const OFFSET_MASK: u32 = 0x00FF_FFFF;
        let link = opcode & LINK_MASK > 0;
        if link {
            self.link_register = self.program_counter;
        }
        let offset = sign_extend_32((opcode & OFFSET_MASK) << 2, 26);
        // + 4 for prefetch
        self.program_counter = self.register(15).wrapping_add_signed(offset);
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
        self.link_register = self.program_counter;
        self.program_status_register
            .set_supervisor(self.program_status_register.current());
        self.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.program_status_register.set_mode(Mode::Supervisor);
    }

    pub fn thumb_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_thumb_instruction(opcode);
        self.program_counter = self.program_counter.wrapping_add(2);
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
            ThumbInstruction::ALUOperations => self.alu_operations(opcode),
            ThumbInstruction::MoveCompareAddSubtractImmediate => {
                self.move_compare_add_subtract_immediate(opcode);
            }
            ThumbInstruction::AddSubtract => self.add_subtract(opcode),
            ThumbInstruction::MoveShiftedRegister => self.move_shifted_register(opcode),
            ThumbInstruction::Unimplemented => {
                println!(
                    "Location: {:#010X}, Callback: {:#010X}",
                    self.program_counter, self.link_register
                );
                unreachable!("Unimplemented Thumb opcode: {opcode:#06X}")
            }
        }
    }

    fn thumb_software_interrupt(&mut self) {
        self.link_register = self.program_counter;
        self.program_status_register
            .set_supervisor(self.program_status_register.current());
        self.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.program_status_register.set_state(CpuMode::Arm);
        self.program_status_register.set_mode(Mode::Supervisor);
    }

    fn unconditional_branch(&mut self, opcode: u16) {
        // Offset is a 12 bit value, but is stored as 11 bits (lsb is dropped) because it must be halfword aligned
        const MASK: u16 = 0x07FF;
        let offset = ((opcode & MASK) << 1).sign_extend_12();
        self.program_counter = self.register(15).wrapping_add_signed(offset);
    }

    fn conditional_branch(&mut self, opcode: u16) {
        const CONDITIONS_MASK: u16 = 0x0F00;
        const CONDITIONS_SHIFT: usize = 8;
        const SIGNED_OFFSET_MASK: u16 = 0x00FF;
        let conditions = (opcode & CONDITIONS_MASK) >> CONDITIONS_SHIFT;
        let offset = ((opcode & SIGNED_OFFSET_MASK) as i8) << 1;
        let branch = self
            .program_status_register
            .check_conditions(conditions.truncate());
        if branch {
            self.program_counter = self.register(15).wrapping_add_signed(i32::from(offset));
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
        let mut address = self.register(base_register) as usize;
        for register in (0..=7).filter(|r| register_list & (1 << r) > 0) {
            self.set_register(register, bus.read_32(address));
            address = address.wrapping_add(4);
        }
        self.set_register(base_register, address as u32);
    }

    fn multiple_store(&mut self, base_register: u16, register_list: u16, bus: &mut Bus) {
        let mut address = self.register(base_register) as usize;
        for register in (0..=7).filter(|r| register_list & (1 << r) > 0) {
            bus.write_32(address, self.register(register));
            address = address.wrapping_add(4);
        }
        self.set_register(base_register, address as u32);
    }

    fn long_branch_with_link(&mut self, opcode: u16) {
        const H_MASK: u16 = 0x0800;
        const OFFSET_MASK: u16 = 0x07FF;
        let h = opcode & H_MASK > 0;
        let offset = opcode & OFFSET_MASK;
        if !h {
            self.link_register = self
                .register(15)
                .wrapping_add((offset << 1).sign_extend_12().cast_unsigned() << 11);
        } else {
            let return_address = self.program_counter;
            self.link_register = self.link_register.wrapping_add(u32::from(offset << 1));
            self.program_counter = self.link_register;
            self.link_register = return_address | 1;
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
            self.stack_pointer = self.stack_pointer.wrapping_sub_signed(i32::from(word));
        } else {
            self.stack_pointer = self.stack_pointer.wrapping_add_signed(i32::from(word));
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
            self.set_register(register, val);
        }
        if pc_lr {
            self.program_counter = self.pop(bus);
        }
    }

    fn push_registers(&mut self, register_list: u16, pc_lr: bool, bus: &mut Bus) {
        if pc_lr {
            self.push(self.link_register, bus);
        }
        for register in (0..=7).rev().filter(|r| register_list & (1 << r) > 0) {
            self.push(self.register(register), bus);
        }
    }

    fn pop(&mut self, bus: &Bus) -> u32 {
        let value = bus.read_32(self.stack_pointer as usize);
        self.stack_pointer = self.stack_pointer.wrapping_add(4);
        value
    }

    fn push(&mut self, value: u32, bus: &mut Bus) {
        self.stack_pointer = self.stack_pointer.wrapping_sub(4);
        bus.write_32(self.stack_pointer as usize, value);
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
        let address = self.register(base_register).wrapping_add(u32::from(offset)) as usize;
        if load {
            self.set_register(source_destination_register, u32::from(bus.read_16(address)));
        } else {
            bus.write_16(
                address,
                self.register(source_destination_register).truncate(),
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
            self.set_register(
                destination_register,
                bus.read_32(self.stack_pointer.wrapping_add(offset) as usize),
            );
        } else {
            bus.write_32(
                self.stack_pointer.wrapping_add(offset) as usize,
                self.register(destination_register),
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
            self.stack_pointer.wrapping_add(word)
        } else {
            self.register(15).wrapping_add(word) & !3
        };
        self.set_register(destination_register, address);
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
        let byte_address = self.register(base_register).wrapping_add(u32::from(offset)) as usize;
        let word_address = self
            .register(base_register)
            .wrapping_add(u32::from(offset << 2)) as usize;
        match (load, byte) {
            (true, true) => self.set_register(
                source_destination_register,
                u32::from(bus.read_8(byte_address)),
            ),
            (true, false) => {
                self.set_register(source_destination_register, bus.read_32(word_address))
            }
            (false, true) => bus.write_8(
                byte_address,
                self.register(source_destination_register).truncate(),
            ),
            (false, false) => {
                bus.write_32(word_address, self.register(source_destination_register));
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
            .register(base_register)
            .wrapping_add(self.register(offset_register)) as usize;
        match (load, byte) {
            (true, true) => {
                self.set_register(source_destination_register, u32::from(bus.read_8(address)))
            }
            (true, false) => self.set_register(source_destination_register, bus.read_32(address)),
            (false, true) => bus.write_8(
                address,
                self.register(source_destination_register).truncate(),
            ),
            (false, false) => bus.write_32(address, self.register(source_destination_register)),
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
            .register(base_register)
            .wrapping_add(self.register(offset_register)) as usize;
        match (sign_extend_flag, h_flag) {
            (false, false) => bus.write_16(
                address,
                self.register(source_destination_register).truncate(),
            ),
            (false, true) => {
                self.set_register(source_destination_register, u32::from(bus.read_16(address)));
            }
            (true, false) => self.set_register(
                source_destination_register,
                bus.read_8(address).sign_extend().cast_unsigned(),
            ),
            (true, true) => self.set_register(
                source_destination_register,
                bus.read_16(address).sign_extend().cast_unsigned(),
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
        let value = bus.read_32((self.register(15) & !3).wrapping_add(u32::from(offset)) as usize);
        self.set_register(destination_register, value);
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
        let value = self.register(dest).wrapping_add(self.register(src));
        self.set_register(dest, value);
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
        self.test_sub(dest, src);
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
        self.set_register(dest, self.register(src));
    }

    fn hi_register_bx(&mut self, h: bool, register: u16) {
        let r = (u16::from(h) << 3) | register;
        let mode = if self.register(r) & 0x01 > 0 {
            CpuMode::Thumb
        } else {
            CpuMode::Arm
        };
        self.program_status_register.set_state(mode);
        self.program_counter = self.register(r) & !1;
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
        let (value, carry, overflow) = match sub_opcode {
            0x00 => and(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x01 => xor(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x02 => shift_left(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x03 => shift_right(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x04 => arithmetic_shift_right(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x05 => add_carry(
                self.register(source_register),
                self.register(destination_register),
                self.program_status_register.carry().into(),
            ),
            0x06 => sub_carry(
                self.register(source_register),
                self.register(destination_register),
                self.program_status_register.carry().into(),
            ),
            0x07 => rotate_right(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x08 => and(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x09 => subtract(0, self.register(source_register)),
            0x0A => subtract(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x0B => add(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x0C => or(
                self.register(destination_register),
                self.register(source_register),
            ),
            0x0D => (
                self.register(destination_register)
                    .wrapping_mul(self.register(source_register)),
                None,
                None,
            ),
            0x0E => and(
                self.register(destination_register),
                !self.register(source_register),
            ),
            0x0F => (!self.register(source_register), None, None),
            _ => unreachable!(),
        };
        self.set_conditions(value, carry, overflow);
        if !matches!(sub_opcode, 0x08 | 0x0A | 0x0B) {
            self.set_register(destination_register, value);
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
        let (value, carry, overflow) = match sub_opcode {
            0b00 => (u32::from(immediate_value), None, None),
            0b01 => subtract(
                self.register(destination_register),
                u32::from(immediate_value),
            ),
            0b10 => add(
                self.register(destination_register),
                u32::from(immediate_value),
            ),
            0b11 => subtract(
                self.register(destination_register),
                u32::from(immediate_value),
            ),
            _ => unreachable!(),
        };
        self.set_conditions(value, carry, overflow);
        if sub_opcode == 0b01 {
            return;
        }
        self.set_register(destination_register, value);
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
        let value = match (sub_opcode, i) {
            (false, false) => self
                .register(source_register)
                .wrapping_add(self.register(r3)),
            (false, true) => self
                .register(source_register)
                .wrapping_add(u32::from(immediate_value)),
            (true, false) => self
                .register(source_register)
                .wrapping_sub(self.register(r3)),
            (true, true) => self
                .register(source_register)
                .wrapping_sub(u32::from(immediate_value)),
        };
        self.set_conditions(value, None, None);
        self.set_register(destination_register, value);
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
        let (value, carry, overflow) = match sub_opcode {
            0b00 => shift_left(self.register(source_register), immediate_value.into()),
            0b01 => shift_right(self.register(source_register), immediate_value.into()),
            0b10 => arithmetic_shift_right(self.register(source_register), immediate_value.into()),
            _ => unreachable!(),
        };
        self.set_conditions(value, carry, overflow);
        self.set_register(destination_register, value);
    }

    pub fn register(&self, register: u16) -> u32 {
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
            15 => {
                if self.program_status_register.state() == CpuMode::Arm {
                    self.program_counter.wrapping_add(4)
                } else {
                    self.program_counter.wrapping_add(2)
                }
            }
            _ => unreachable!(),
        }
    }

    fn set_register(&mut self, register: u16, value: u32) {
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
            _ => unreachable!(),
        }
    }

    fn ror(&mut self, destination_register: u16, source_register: u16) -> u32 {
        let mut rotated = self.register(destination_register);
        for _ in 0..self.register(source_register) {
            let temp = u32::from(self.program_status_register.carry());
            self.program_status_register.set_carry(rotated & 0x01 > 0);
            rotated = (rotated >> 1) | (temp << 31);
        }
        rotated
    }

    fn test_and(&mut self, source_register: u16, destination_register: u16) {
        let temp = self.register(source_register) & self.register(destination_register);
        self.program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
    }

    fn test_add(&mut self, source_register: u16, destination_register: u16) {
        let (temp, carry) = self
            .register(source_register)
            .overflowing_add(self.register(destination_register));
        let (_, overflow) = self
            .register(source_register)
            .cast_signed()
            .overflowing_add(self.register(destination_register).cast_signed());
        self.program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(carry);
        self.program_status_register.set_overflow(overflow);
    }

    fn test_sub(&mut self, source_register: u16, destination_register: u16) {
        let (temp, borrow) = self
            .register(source_register)
            .overflowing_sub(self.register(destination_register));
        let (_, overflow) = self
            .register(source_register)
            .cast_signed()
            .overflowing_sub(self.register(destination_register).cast_signed());
        self.program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(!borrow);
        self.program_status_register.set_overflow(overflow);
    }

    fn test_cmp(&mut self, source_register: u16, immediate: u32) {
        let (temp, borrow) = self.register(source_register).overflowing_sub(immediate);
        let (_, overflow) = self
            .register(source_register)
            .cast_signed()
            .overflowing_sub(immediate.cast_signed());
        self.program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.program_status_register.set_zero(temp == 0x0000_0000);
        self.program_status_register.set_carry(!borrow);
        self.program_status_register.set_overflow(overflow);
    }
}

const ARM_DECODE_TABLE: [(u32, u32, ArmInstruction); 14] = [
    // Mask         Value                      Instruction
    (0x0FFF_FFF0, 0x012F_FF10, ArmInstruction::BranchAndExchange),
    (0x0E00_0000, 0x0800_0000, ArmInstruction::BlockDataTransfer),
    (0x0E00_0000, 0x0A00_0000, ArmInstruction::Branch),
    (0x0F00_0000, 0x0F00_0000, ArmInstruction::SoftwareInterrupt),
    (0x0E00_0010, 0x0600_0010, ArmInstruction::Undefined),
    (0x0C00_0000, 0x0400_0000, ArmInstruction::SingleDataTransfer),
    (0x0F80_0FF0, 0x0100_0090, ArmInstruction::SingleDataSwap),
    (0x0F80_00F0, 0x0000_0090, ArmInstruction::Multiply),
    (0x0F80_00F0, 0x0080_0090, ArmInstruction::MultiplyLong),
    (
        0x0E40_0F90,
        0x0000_0090,
        ArmInstruction::HalfwordDataTransferRegisterOffset,
    ),
    (
        0x0E40_0090,
        0x0040_0090,
        ArmInstruction::HalfwordDataTransferImmediateOffset,
    ),
    (0x0FBF_0000, 0x010F_0000, ArmInstruction::PsrTransferMRS),
    (0x0DB0_F000, 0x0120_F000, ArmInstruction::PsrTransferMSR),
    (0x0C00_0000, 0x0000_0000, ArmInstruction::DataProcessing),
];

fn decode_arm_instruction(opcode: u32) -> ArmInstruction {
    ARM_DECODE_TABLE
        .iter()
        .find(|&&(mask, value, _)| opcode & mask == value)
        .map_or(ArmInstruction::Undefined, |&(_, _, instruction)| {
            instruction
        })
}

// Look into `find` over a const table of (mask, value, ThumbInstruction)
fn decode_thumb_instruction(opcode: u16) -> ThumbInstruction {
    match () {
        () if is_software_interrupt(opcode) => ThumbInstruction::SoftwareInterrupt,
        () if is_unconditional_branch(opcode) => ThumbInstruction::UnconditionalBranch,
        () if is_conditional_branch(opcode) => ThumbInstruction::ConditionalBranch,
        () if is_multiple_loadstore(opcode) => ThumbInstruction::MultipleLoadstore,
        () if is_long_branch_with_link(opcode) => ThumbInstruction::LongBranchWithLink,
        () if is_add_offset_to_stack_pointer(opcode) => ThumbInstruction::AddOffsetToStackPointer,
        () if is_push_pop_registers(opcode) => ThumbInstruction::PushPopRegisters,
        () if is_load_store_halfword(opcode) => ThumbInstruction::LoadStoreHalfword,
        () if is_sp_relative_load_store(opcode) => ThumbInstruction::SPRelativeLoadStore,
        () if is_load_address(opcode) => ThumbInstruction::LoadAddress,
        () if is_load_store_with_immediate_offset(opcode) => {
            ThumbInstruction::LoadStoreWithImmediateOffset
        }
        () if is_load_store_with_register_offset(opcode) => {
            ThumbInstruction::LoadStoreWithRegisterOffset
        }
        () if is_load_store_sign_extended_byte_halfword(opcode) => {
            ThumbInstruction::LoadStoreSignExtendedByteHalfword
        }
        () if is_pc_relative_load(opcode) => ThumbInstruction::PCRelativeLoad,
        () if is_hi_register_operations_branch_exchange(opcode) => {
            ThumbInstruction::HiRegisterOperationsBranchExchange
        }
        () if is_alu_operations(opcode) => ThumbInstruction::ALUOperations,
        () if is_move_compare_add_subtract_immediate(opcode) => {
            ThumbInstruction::MoveCompareAddSubtractImmediate
        }
        () if is_add_subtract(opcode) => ThumbInstruction::AddSubtract,
        () if is_move_shifted_register(opcode) => ThumbInstruction::MoveShiftedRegister,
        () => ThumbInstruction::Unimplemented,
    }
}

const fn is_software_interrupt(opcode: u16) -> bool {
    const MASK: u16 = 0xFF00;
    const SOFTWARE_INTERRUPT: u16 = 0xDF00;
    opcode & MASK == SOFTWARE_INTERRUPT
}

const fn is_unconditional_branch(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const UNCONDITIONAL_BRANCH: u16 = 0xE000;
    opcode & MASK == UNCONDITIONAL_BRANCH
}

const fn is_conditional_branch(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const CONDITIONAL_BRANCH: u16 = 0xD000;
    opcode & MASK == CONDITIONAL_BRANCH
}

const fn is_multiple_loadstore(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const MULTIPLE_LOADSTORE: u16 = 0xC000;
    opcode & MASK == MULTIPLE_LOADSTORE
}

const fn is_long_branch_with_link(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LONG_BRANCH_WITH_LINK: u16 = 0xF000;
    opcode & MASK == LONG_BRANCH_WITH_LINK
}

const fn is_add_offset_to_stack_pointer(opcode: u16) -> bool {
    const MASK: u16 = 0xFF00;
    const ADD_OFFSET_TO_STACK_POINTER: u16 = 0xB000;
    opcode & MASK == ADD_OFFSET_TO_STACK_POINTER
}

const fn is_push_pop_registers(opcode: u16) -> bool {
    const MASK: u16 = 0xF600;
    const PUSH_POP_REGISTERS: u16 = 0xB400;
    opcode & MASK == PUSH_POP_REGISTERS
}

const fn is_load_store_halfword(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LOAD_STORE_HALFWORD: u16 = 0x8000;
    opcode & MASK == LOAD_STORE_HALFWORD
}

const fn is_sp_relative_load_store(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const SP_RELATIVE_LOAD_STORE: u16 = 0x9000;
    opcode & MASK == SP_RELATIVE_LOAD_STORE
}

const fn is_load_address(opcode: u16) -> bool {
    const MASK: u16 = 0xF000;
    const LOAD_ADDRESS: u16 = 0xA000;
    opcode & MASK == LOAD_ADDRESS
}

const fn is_load_store_with_immediate_offset(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const LOAD_STORE_WITH_IMMEDIATE_OFFSET: u16 = 0x6000;
    opcode & MASK == LOAD_STORE_WITH_IMMEDIATE_OFFSET
}

const fn is_load_store_with_register_offset(opcode: u16) -> bool {
    const MASK: u16 = 0xF200;
    const LOAD_STORE_WITH_REGISTER_OFFSET: u16 = 0x5000;
    opcode & MASK == LOAD_STORE_WITH_REGISTER_OFFSET
}

const fn is_load_store_sign_extended_byte_halfword(opcode: u16) -> bool {
    const MASK: u16 = 0xF200;
    const LOAD_STORE_SIGN_EXTENDED_BYTE_HALFWORD: u16 = 0x5200;
    opcode & MASK == LOAD_STORE_SIGN_EXTENDED_BYTE_HALFWORD
}

const fn is_pc_relative_load(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const PC_RELATIVE_LOAD: u16 = 0x4800;
    opcode & MASK == PC_RELATIVE_LOAD
}

const fn is_hi_register_operations_branch_exchange(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE: u16 = 0x4400;
    opcode & MASK == HI_REGISTER_OPERATIONS_BRANCH_EXCHANGE
}

const fn is_alu_operations(opcode: u16) -> bool {
    const MASK: u16 = 0xFC00;
    const ALU_OPERATIONS: u16 = 0x4000;
    opcode & MASK == ALU_OPERATIONS
}

const fn is_move_compare_add_subtract_immediate(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const MOVE_COMPARE_ADD_SUBTRACT_IMMEDIATE: u16 = 0x2000;
    opcode & MASK == MOVE_COMPARE_ADD_SUBTRACT_IMMEDIATE
}

const fn is_add_subtract(opcode: u16) -> bool {
    const MASK: u16 = 0xF800;
    const ADD_SUBTRACT: u16 = 0x1800;
    opcode & MASK == ADD_SUBTRACT
}

const fn is_move_shifted_register(opcode: u16) -> bool {
    const MASK: u16 = 0xE000;
    const MOVE_SHIFTED_REGISTER: u16 = 0x0000;
    opcode & MASK == MOVE_SHIFTED_REGISTER
}

fn conditions(opcode: u32) -> u8 {
    const MASK: u32 = 0xF000_0000;
    const SHIFT: usize = 28;
    ((opcode & MASK) >> SHIFT).truncate()
}

trait Bit {
    fn bit(&self, bit: usize) -> bool;
}

impl Bit for u32 {
    fn bit(&self, bit: usize) -> bool {
        self & (1 << (bit % 32)) > 0
    }
}

fn sign_extend_32(value: u32, size: u32) -> i32 {
    ((value << (32 - size)) as i32) >> (32 - size)
}

trait Extendable {
    fn sign_extend(self) -> i32;
}

impl Extendable for u8 {
    fn sign_extend(self) -> i32 {
        if self & 0x80 == 0x80 {
            0xFFFF_FF00u32.cast_signed() + i32::from(self)
        } else {
            i32::from(self)
        }
    }
}

impl Extendable for u16 {
    fn sign_extend(self) -> i32 {
        if self & 0x8000 == 0x8000 {
            0xFFFF_0000u32.cast_signed() + i32::from(self)
        } else {
            i32::from(self)
        }
    }
}

trait Extendable12 {
    fn sign_extend_12(self) -> i32;
}

impl Extendable12 for u16 {
    fn sign_extend_12(self) -> i32 {
        if self & 0x0800 > 0 {
            0xFFFF_F000u32.cast_signed() + i32::from(self)
        } else {
            i32::from(self)
        }
    }
}

fn shift_left(x: u32, shift: u32) -> (u32, Option<bool>, Option<bool>) {
    match shift {
        0 => (x, None, None),
        1..=31 => (
            x << shift,
            Some(x.bit(32usize.wrapping_sub(shift as usize))),
            None,
        ),
        32 => (0, Some(x.bit(0)), None),
        _ => (0, Some(false), None),
    }
}

fn shift_right(x: u32, shift: u32) -> (u32, Option<bool>, Option<bool>) {
    match shift {
        0 => (x, None, None),
        1..=31 => (
            x >> shift,
            Some(x.bit((shift as usize).wrapping_sub(1))),
            None,
        ),
        32 => (0, Some(x.bit(31)), None),
        _ => (0, Some(false), None),
    }
}

fn arithmetic_shift_right(x: u32, shift: u32) -> (u32, Option<bool>, Option<bool>) {
    if shift > 31 {
        let bit = x.bit(31);
        let value = if bit { 0xFFFF_FFFF } else { 0x0000_0000 };
        (value, Some(bit), None)
    } else {
        (
            (x.cast_signed() >> shift).cast_unsigned(),
            Some(x.bit((shift as usize).wrapping_sub(1))),
            None,
        )
    }
}

fn rotate_right(x: u32, shift: u32) -> (u32, Option<bool>, Option<bool>) {
    if shift > 31 {
        let s = shift % 32;
        let bit = if s == 0 {
            31
        } else {
            (s as usize).wrapping_sub(1)
        };
        (x.rotate_right(shift), Some(x.bit(bit)), None)
    } else {
        (
            x.rotate_right(shift),
            Some(x.bit((shift as usize).wrapping_sub(1))),
            None,
        )
    }
}

fn and(x: u32, y: u32) -> (u32, Option<bool>, Option<bool>) {
    (x & y, None, None)
}

fn xor(x: u32, y: u32) -> (u32, Option<bool>, Option<bool>) {
    (x ^ y, None, None)
}

fn or(x: u32, y: u32) -> (u32, Option<bool>, Option<bool>) {
    (x | y, None, None)
}

fn add(x: u32, y: u32) -> (u32, Option<bool>, Option<bool>) {
    let (result, carry) = x.overflowing_add(y);
    let (_, overflow) = x.cast_signed().overflowing_add(y.cast_signed());
    (result, Some(carry), Some(overflow))
}

fn subtract(x: u32, y: u32) -> (u32, Option<bool>, Option<bool>) {
    let (result, borrow) = x.overflowing_sub(y);
    let (_, overflow) = x.cast_signed().overflowing_sub(y.cast_signed());
    (result, Some(!borrow), Some(overflow))
}

fn add_carry(x: u32, y: u32, carry: u32) -> (u32, Option<bool>, Option<bool>) {
    let (temp, c1) = x.overflowing_add(y);
    let (result, c2) = temp.overflowing_add(carry);
    let (temp, o1) = x.cast_signed().overflowing_add(y.cast_signed());
    let (_, o2) = temp.overflowing_add(carry.cast_signed());
    (result, Some(c1 | c2), Some(o1 | o2))
}

fn sub_carry(x: u32, y: u32, carry: u32) -> (u32, Option<bool>, Option<bool>) {
    let (temp, b1) = x.overflowing_sub(y);
    let (result, b2) = temp.wrapping_add(carry).overflowing_sub(1);
    let (temp, o1) = x.cast_signed().overflowing_sub(y.cast_signed());
    let (_, o2) = temp.wrapping_add(carry.cast_signed()).overflowing_sub(1);
    (result, Some(!(b1 | b2)), Some(o1 | o2))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_shifted_register_sets_register() {
        let mut cpu = Cpu::thumb();
        let mut bus = Bus::new();

        cpu.r00 = 0xFFFF_FFFF;
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
