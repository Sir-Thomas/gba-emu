#![allow(dead_code)]

use std::num::TryFromIntError;

use thiserror::Error;

pub use crate::cpu::decode::Instruction;
use crate::{
    bus::Bus,
    cpu::{
        alu::AluResult,
        decode::*,
        registers::{CpuMode, InstructionSet, RegisterError, Registers},
    },
};

mod alu;
mod decode;
pub mod registers;

const SOFTWARE_INTERRUPT_ADDRESS: u32 = 0x0000_0008;

#[derive(Clone, Error, Debug)]
pub enum CpuError {
    #[error("Error converting integers")]
    UsizeError(#[from] TryFromIntError),
    #[error("Register Error: {0}")]
    Register(#[from] RegisterError),
    #[error("Unimplemented opcode {opcode:#010X} at {pc:#010X}")]
    Unimplemented { opcode: u32, pc: u32 },
    #[error("Data Processing: Invalid Operation {operation:#03X}")]
    InvalidDataProcessingOperation { operation: u8 },
    #[error("Sign extend should not be used with store")]
    SignExtendWithStore,
    #[error("Undefined Arm Instruction {opcode:#010X}")]
    UndefinedArmInstruction { opcode: u32 },
    #[error("Invalid shift type: {shift_type:#06X}")]
    InvalidShiftType { shift_type: u32 },
    #[error("Unimplemented Thumb opcode: {opcode:#06X}")]
    UnimplementedThumbInstruction { opcode: u16 },
    #[error("Unreachable Hi Register Operations/Branch Exchange sub opcode {sub_opcode:#04X}")]
    UnreachableHiRegisterOperationsBranchExchangeSubOpcode { sub_opcode: u8 },
    #[error("Unreachable Alu Operations sub opcode {sub_opcode:#04X}")]
    UnreachableAluOperationsSubOpcode { sub_opcode: u8 },
    #[error("Unreachable Move Compare Add Subtract Immediate Sub Opcode {sub_opcode:#06X}")]
    UnreachableMoveCompareAddSubtractImmediateSubOpcode { sub_opcode: u16 },
    #[error("Unreachable Move Shifted Register Sub Opcode {sub_opcode:#06X}")]
    UnreachableMoveShiftedRegisterSubOpcode { sub_opcode: u16 },
    #[error("Attempted to Multiply Long with Program Counter")]
    MultiplyLongWithProgramCounter,
    #[error(
        "Error executing instruction
        Program Counter: {program_counter:#010X}
        Opcode: {opcode:#010X}
        {source}"
    )]
    InstructionContext {
        program_counter: u32,
        opcode: u32,
        #[source]
        source: Box<Self>,
    },
}

#[derive(Default)]
pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    pub fn new() -> Self {
        let mut cpu = Self::default();
        cpu.registers.set_program_counter(0x0800_0000);
        cpu.registers.set_stack_pointer(0x0300_7F00);
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

    pub fn next_instruction(&self, bus: &Bus) -> Result<(u32, Instruction), CpuError> {
        if self.registers.state() == InstructionSet::Arm {
            let opcode = bus.read_32(usize::try_from(self.registers.program_counter)?);
            let instruction = decode::arm_instruction(opcode);
            Ok((opcode, Instruction::Arm(instruction)))
        } else {
            let opcode = bus.read_16(usize::try_from(self.registers.program_counter)?);
            let instruction = decode::thumb_instruction(opcode);
            Ok((u32::from(opcode), Instruction::Thumb(instruction)))
        }
    }

    // This doesn't really work
    // It can't handle branches or mode changes, but it gets me what I need for now
    pub const fn back(&mut self) {
        match self.registers.state() {
            InstructionSet::Arm => self.registers.program_counter -= 4,
            InstructionSet::Thumb => self.registers.program_counter -= 2,
        }
    }

    pub fn cpu_cycle(&mut self, bus: &mut Bus) -> Result<(), CpuError> {
        match self.registers.state() {
            InstructionSet::Arm => self.arm_cycle(bus),
            InstructionSet::Thumb => self.thumb_cycle(bus),
        }
    }

    pub fn arm_cycle(&mut self, bus: &mut Bus) -> Result<(), CpuError> {
        let program_counter = self.registers.program_counter;
        let opcode = bus.read_32(usize::try_from(self.registers.program_counter)?);
        self.registers.program_counter = self.registers.program_counter.wrapping_add(4);
        if self.registers.check_conditions(conditions(opcode))? {
            let instruction = decode::arm_instruction(opcode);
            self.run_arm_instruction(instruction, opcode, bus)
                .map_err(|e| CpuError::InstructionContext {
                    program_counter,
                    opcode,
                    source: Box::new(e),
                })?;
        }
        Ok(())
    }

    fn run_arm_instruction(
        &mut self,
        instruction: ArmInstruction,
        opcode: u32,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        match instruction {
            ArmInstruction::DataProcessing(operands) => self.data_processing(operands),
            ArmInstruction::PsrTransferMrs {
                saved,
                destination_register,
            } => self.psr_transfer_mrs(saved, destination_register),
            ArmInstruction::PsrTransferMsr(operands) => self.psr_transfer_msr(operands),
            ArmInstruction::Multiply(operands) => self.multiply(operands),
            ArmInstruction::MultiplyLong(operands) => self.multiply_long(operands),
            ArmInstruction::SingleDataSwap(operands) => self.single_data_swap(operands, bus),
            ArmInstruction::BranchAndExchange { register } => self.branch_and_exchange(register),
            ArmInstruction::HalfwordDataTransferRegisterOffset { operands, register } => {
                self.halfword_data_transfer_register_offset(operands, register, bus)
            }
            ArmInstruction::HalfwordDataTransferImmediateOffset { operands, offset } => {
                self.halfword_data_transfer_immediate_offset(operands, offset, bus)
            }
            ArmInstruction::SingleDataTransfer(operands) => {
                self.single_data_transfer(operands, bus)
            }
            ArmInstruction::Undefined => Err(CpuError::UndefinedArmInstruction { opcode }),
            ArmInstruction::BlockDataTransfer(operands) => self.block_data_transfer(operands, bus),
            ArmInstruction::Branch { link, offset } => self.branch(link, offset),
            ArmInstruction::CoprocessorDataTransfer => self.coprocessor_data_transfer(opcode),
            ArmInstruction::CoprocessorDataOperation => self.coprocessor_data_operation(opcode),
            ArmInstruction::CoprocessorRegisterTransfer => {
                self.coprocessor_register_transfer(opcode)
            }
            ArmInstruction::SoftwareInterrupt => self.arm_software_interrupt(),
        }
    }

    fn shift(&self, operand: u32) -> Result<AluResult, CpuError> {
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
            self.registers.get(register.truncate())? & 0x0000_00FF
        };
        let source_register = (operand & SOURCE_REGISTER_MASK).truncate();
        // instruction pre-fetching work around
        let source_value = if source_register == 15 && !immediate {
            self.registers.get(source_register)?.wrapping_add(4)
        } else {
            self.registers.get(source_register)?
        };
        match (shift_type, immediate && shift == 0) {
            (0b00, _) => Ok(alu::shift_left(source_value, shift)),
            (0b01, true) => Ok(alu::shift_right(source_value, 32)),
            (0b01, false) => Ok(alu::shift_right(source_value, shift)),
            (0b10, true) => Ok(alu::arithmetic_shift_right(source_value, 32)),
            (0b10, false) => Ok(alu::arithmetic_shift_right(source_value, shift)),
            (0b11, true) => Ok(alu::rotate_right_carry(
                source_value,
                self.registers.carry(),
            )),
            (0b11, false) => Ok(alu::rotate_right(source_value, shift)),
            _ => Err(CpuError::InvalidShiftType { shift_type }),
        }
    }

    fn data_processing(&mut self, operands: DataProcessingOperands) -> Result<(), CpuError> {
        let alu_result = match operands.second_operand {
            SecondOperand::Immediate { rotate, value } => {
                Ok(alu::rotate_right(value.into(), rotate.into()))
            }
            SecondOperand::Shift(s) => self.shift(s.into()),
        }?;
        // instruction pre-fetching work around
        let first_operand = if operands.add_four {
            self.registers
                .get(operands.first_operand_register)?
                .wrapping_add(4)
        } else {
            self.registers.get(operands.first_operand_register)?
        };
        let second_operand = alu_result.result;
        if operands.set_conditions
            && matches!(
                operands.operation,
                DataProcessingOperation::And
                    | DataProcessingOperation::Xor
                    | DataProcessingOperation::TestAnd
                    | DataProcessingOperation::TestXor
                    | DataProcessingOperation::Or
                    | DataProcessingOperation::Move
                    | DataProcessingOperation::BitClear
                    | DataProcessingOperation::MoveNot
            )
        {
            self.registers.status_registers.set_carry(alu_result.carry);
        }
        let carry_flag = u32::from(self.registers.carry());
        let result = match operands.operation {
            DataProcessingOperation::And | DataProcessingOperation::TestAnd => {
                alu::and(first_operand, second_operand)
            }
            DataProcessingOperation::Xor | DataProcessingOperation::TestXor => {
                alu::xor(first_operand, second_operand)
            }
            DataProcessingOperation::Subtract | DataProcessingOperation::TestSubtract => {
                alu::subtract(first_operand, second_operand)
            }
            DataProcessingOperation::ReverseSubtract => {
                alu::subtract(second_operand, first_operand)
            }
            DataProcessingOperation::Add | DataProcessingOperation::TestAdd => {
                alu::add(first_operand, second_operand)
            }
            DataProcessingOperation::AddWithCarry => {
                alu::add_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::SubtractWithCarry => {
                alu::sub_carry(first_operand, second_operand, carry_flag)
            }
            DataProcessingOperation::ReverseSubtractWithCarry => {
                alu::sub_carry(second_operand, first_operand, carry_flag)
            }
            DataProcessingOperation::Or => alu::or(first_operand, second_operand),
            DataProcessingOperation::Move => alu::nop(second_operand),
            DataProcessingOperation::BitClear => alu::and(first_operand, !second_operand),
            DataProcessingOperation::MoveNot => alu::nop(!second_operand),
        };
        if operands.set_conditions {
            self.registers.status_registers.set_conditions(result);
        }
        if operands.state_change {
            self.registers.status_registers.load()?;
        }
        if !matches!(
            operands.operation,
            DataProcessingOperation::TestAnd
                | DataProcessingOperation::TestXor
                | DataProcessingOperation::TestSubtract
                | DataProcessingOperation::TestAdd
        ) {
            self.registers
                .set(operands.destination_register, result.result)?;
        }
        Ok(())
    }

    fn psr_transfer_mrs(&mut self, saved: bool, destination_register: u16) -> Result<(), CpuError> {
        if !saved {
            self.registers
                .set(destination_register.truncate(), self.registers.current())?;
            return Ok(());
        }
        let value = self.registers.status_registers.get_saved()?;
        self.registers.set(destination_register.truncate(), value)?;
        Ok(())
    }

    fn psr_transfer_msr(&mut self, operands: PsrTransferMsrOperands) -> Result<(), CpuError> {
        let mut value = match operands.value {
            PsrTransferValue::Immediate(v) => v,
            PsrTransferValue::Register(r) => self.registers.get(r)?,
        };
        if !operands.mode_included {
            let old = if operands.saved {
                self.registers.status_registers.get_saved()?
            } else {
                self.registers.current()
            };
            value = (value & !0b11111) | (old & 0b11111);
        }
        if operands.saved {
            self.registers.status_registers.save(value)?;
        } else {
            self.registers.status_registers.set_current(value)?;
        }
        Ok(())
    }

    fn multiply(&mut self, operands: MultiplyOperands) -> Result<(), CpuError> {
        let value = if operands.accumulate {
            self.registers
                .get(operands.source_register_m)?
                .wrapping_mul(self.registers.get(operands.source_register_s)?)
                .wrapping_add(self.registers.get(operands.source_register_n)?)
        } else {
            self.registers
                .get(operands.source_register_m)?
                .wrapping_mul(self.registers.get(operands.source_register_s)?)
        };
        self.registers
            .set(operands.destination_register.truncate(), value)?;
        if operands.set_conditions {
            self.registers.status_registers.set_conditions(AluResult {
                result: value,
                carry: None,
                overflow: None,
            });
        }
        Ok(())
    }

    fn multiply_long(&mut self, operands: MultiplyLongOperands) -> Result<(), CpuError> {
        if matches!(
            operands.destination_register_hi
                | operands.destination_register_low
                | operands.source_register_s
                | operands.source_register_m,
            15
        ) {
            return Err(CpuError::MultiplyLongWithProgramCounter);
        }
        let mut result = if operands.accumulate {
            (u64::from(self.registers.get(operands.destination_register_hi)?) << 32).wrapping_add(
                u64::from(self.registers.get(operands.destination_register_low)?),
            )
        } else {
            0
        };
        result = if operands.signed {
            result
                .cast_signed()
                .wrapping_add(
                    i64::from(
                        self.registers
                            .get(operands.source_register_m)?
                            .cast_signed(),
                    )
                    .wrapping_mul(i64::from(
                        self.registers
                            .get(operands.source_register_s)?
                            .cast_signed(),
                    )),
                )
                .cast_unsigned()
        } else {
            result.wrapping_add(
                u64::from(self.registers.get(operands.source_register_m)?)
                    .wrapping_mul(u64::from(self.registers.get(operands.source_register_s)?)),
            )
        };
        let hi32 = (result >> 32).truncate();
        let low32 = result.truncate();
        self.registers.set(operands.destination_register_hi, hi32)?;
        self.registers
            .set(operands.destination_register_low, low32)?;
        if operands.set_conditions {
            self.registers.set_zero(result == 0);
            self.registers.set_negative(result.cast_signed() < 0);
        }
        Ok(())
    }

    fn single_data_swap(
        &mut self,
        operands: DataSwapOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let address = self.registers.get(operands.base_register)?;
        let temp = self.registers.get(operands.source_register)?;
        self.data_transfer(
            operands.byte,
            true,
            operands.destination_register,
            bus,
            address,
        )?;
        self.data_write(operands.byte, temp, bus, address)?;
        Ok(())
    }

    fn data_write(
        &self,
        byte: bool,
        value: u32,
        bus: &mut Bus,
        address: u32,
    ) -> Result<(), CpuError> {
        if byte {
            bus.write_8(usize::try_from(address)?, value.truncate());
        } else {
            bus.write_32(usize::try_from(address & !3)?, value);
        }
        Ok(())
    }

    fn branch_and_exchange(&mut self, register: u16) -> Result<(), CpuError> {
        let address = self.registers.get(register)?;
        let thumb = address & 1 > 0;
        if thumb {
            self.registers.set_state(InstructionSet::Thumb);
            self.registers.program_counter = address & !1;
        } else {
            self.registers.set_state(InstructionSet::Arm);
            self.registers.program_counter = address & !3;
        }
        Ok(())
    }

    fn halfword_data_transfer_register_offset(
        &mut self,
        operands: HalfwordDataTransferOperands,
        offset_register: u16,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let offset = self.registers.get(offset_register)?;
        let address = match (operands.pre_index, operands.up) {
            (true, true) => self
                .registers
                .get(operands.base_register)?
                .wrapping_add(offset),
            (true, false) => self
                .registers
                .get(operands.base_register)?
                .wrapping_sub(offset),
            (false, _) => self.registers.get(operands.base_register)?,
        };
        self.halfword_data_transfer(operands, offset, bus, address)?;
        Ok(())
    }

    fn halfword_data_transfer_immediate_offset(
        &mut self,
        operands: HalfwordDataTransferOperands,
        offset: u32,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let address = match (operands.pre_index, operands.up) {
            (true, true) => self
                .registers
                .get(operands.base_register)?
                .wrapping_add(offset),
            (true, false) => self
                .registers
                .get(operands.base_register)?
                .wrapping_sub(offset),
            (false, _) => self.registers.get(operands.base_register)?,
        };
        self.halfword_data_transfer(operands, offset, bus, address)?;
        Ok(())
    }

    fn halfword_data_transfer(
        &mut self,
        operands: HalfwordDataTransferOperands,
        offset: u32,
        bus: &mut Bus,
        address: u32,
    ) -> Result<(), CpuError> {
        if operands.load {
            self.apply_write_back(
                operands.write_back,
                operands.pre_index,
                operands.up,
                operands.base_register,
                address,
                offset,
            )?;
        }
        match (operands.halfword, operands.load, operands.signed) {
            (true, true, true) => self.registers.set(
                operands.source_destination_register,
                (sign_extend(bus.read_16(usize::try_from(address & !1)?).into(), 16).cast_signed()
                    >> ((address & 1) * 8))
                    .cast_unsigned(),
            )?,
            (true, true, false) => self.registers.set(
                operands.source_destination_register,
                u32::from(bus.read_16(usize::try_from(address & !1)?))
                    .rotate_right((address & 1) * 8),
            )?,
            (true, false, false) => bus.write_16(
                usize::try_from(address & !1)?,
                self.registers
                    .get(operands.source_destination_register)?
                    .truncate(),
            ),
            (false, true, true) => self.registers.set(
                operands.source_destination_register,
                sign_extend(bus.read_8(usize::try_from(address)?).into(), 8),
            )?,
            (false, true, false) => self.registers.set(
                operands.source_destination_register,
                u32::from(bus.read_8(usize::try_from(address)?)),
            )?,
            (false, false, false) => bus.write_8(
                usize::try_from(address)?,
                self.registers
                    .get(operands.source_destination_register)?
                    .truncate(),
            ),
            _ => return Err(CpuError::SignExtendWithStore),
        }
        if !operands.load {
            self.apply_write_back(
                operands.write_back,
                operands.pre_index,
                operands.up,
                operands.base_register,
                address,
                offset,
            )?;
        }
        Ok(())
    }

    fn single_data_transfer(
        &mut self,
        operands: SingleDataTransferOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let offset = if operands.immediate {
            operands.offset
        } else {
            self.shift(operands.offset)?.result
        };
        let address = match (operands.pre_index, operands.up) {
            (true, true) => self
                .registers
                .get(operands.base_register)?
                .wrapping_add(offset),
            (true, false) => self
                .registers
                .get(operands.base_register)?
                .wrapping_sub(offset),
            (false, _) => self.registers.get(operands.base_register)?,
        };
        if operands.load {
            self.apply_write_back(
                operands.write_back,
                operands.pre_index,
                operands.up,
                operands.base_register,
                address,
                offset,
            )?;
        }
        self.data_transfer(
            operands.byte,
            operands.load,
            operands.source_destination_register,
            bus,
            address,
        )?;
        if !operands.load {
            self.apply_write_back(
                operands.write_back,
                operands.pre_index,
                operands.up,
                operands.base_register,
                address,
                offset,
            )?;
        }
        Ok(())
    }

    fn data_transfer(
        &mut self,
        byte: bool,
        load: bool,
        source_destination_register: u16,
        bus: &mut Bus,
        address: u32,
    ) -> Result<(), CpuError> {
        match (byte, load) {
            (true, true) => self.registers.set(
                source_destination_register,
                u32::from(bus.read_8(usize::try_from(address)?)),
            )?,
            (true, false) => bus.write_8(
                usize::try_from(address)?,
                self.registers
                    .get_store(source_destination_register)?
                    .truncate(),
            ),
            (false, true) => self.registers.set(
                source_destination_register,
                bus.read_32(usize::try_from(address & !3)?)
                    .rotate_right((address & 3) * 8),
            )?,
            (false, false) => bus.write_32(
                usize::try_from(address & !3)?,
                self.registers.get_store(source_destination_register)?,
            ),
        }
        Ok(())
    }

    fn apply_write_back(
        &mut self,
        write_back: bool,
        pre_index: bool,
        up: bool,
        base_register: u16,
        address: u32,
        offset: u32,
    ) -> Result<(), CpuError> {
        match (write_back, pre_index, up) {
            (false, true, _) => {}
            (_, false, false) => self
                .registers
                .set(base_register, address.wrapping_sub(offset))?,
            (_, false, true) => self
                .registers
                .set(base_register, address.wrapping_add(offset))?,
            (true, true, _) => self.registers.set(base_register, address)?,
        }
        Ok(())
    }

    fn block_data_transfer(
        &mut self,
        operands: BlockDataTransferOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let base_in_list = operands.register_list & (1 << operands.base_register) > 0;
        let base_value = self.registers.get(operands.base_register)?;
        let current_mode = self.registers.mode();
        match (
            operands.psr,
            operands.load,
            operands.register_list & (1 << 15) > 0,
        ) {
            (true, true, true) => self.registers.status_registers.load()?,
            (true, _, _) => self.registers.set_mode(CpuMode::User),
            _ => {}
        }
        let mut address = match (operands.up, operands.pre_index) {
            (true, true) => base_value.wrapping_add(4),
            (true, false) => base_value,
            (false, true) => base_value.wrapping_sub(operands.byte_count),
            (false, false) => base_value.wrapping_sub(operands.byte_count).wrapping_add(4),
        };
        if operands.load {
            self.write_back(
                operands.write_back,
                operands.up,
                base_value,
                operands.byte_count,
                operands.base_register,
            )?;
        }
        let mut lowest = true;
        for register in (0..=15).filter(|r| operands.register_list & (1 << r) > 0) {
            if operands.load {
                self.registers
                    .set(register, bus.read_32(usize::try_from(address)?))?;
            } else {
                // PC is +8 instead of +4 due to pipelining
                let value = if register == 15 {
                    self.registers.program_counter.wrapping_add(8)
                } else if register == operands.base_register
                    && operands.write_back
                    && base_in_list
                    && !lowest
                {
                    if operands.up {
                        base_value.wrapping_add(operands.byte_count)
                    } else {
                        base_value.wrapping_sub(operands.byte_count)
                    }
                } else {
                    lowest = false;
                    self.registers.get(register)?
                };
                bus.write_32(usize::try_from(address)?, value);
            }
            address = address.wrapping_add(4);
        }
        if !operands.load {
            self.write_back(
                operands.write_back,
                operands.up,
                base_value,
                operands.byte_count,
                operands.base_register,
            )?;
        }
        self.registers.set_mode(current_mode);
        Ok(())
    }

    fn write_back(
        &mut self,
        write_back: bool,
        up: bool,
        base_value: u32,
        byte_count: u32,
        base_register: u16,
    ) -> Result<(), CpuError> {
        if write_back {
            let final_address = if up {
                base_value.wrapping_add(byte_count)
            } else {
                base_value.wrapping_sub(byte_count)
            };
            self.registers.set(base_register, final_address)?;
        }
        Ok(())
    }

    fn branch(&mut self, link: bool, offset: i32) -> Result<(), CpuError> {
        if link {
            self.registers.link_register = self.registers.program_counter;
        }
        self.registers.program_counter = self.registers.get(15)?.wrapping_add_signed(offset);
        Ok(())
    }

    fn coprocessor_data_transfer(&self, opcode: u32) -> Result<(), CpuError> {
        todo!("CoprocessorDataTransfer {opcode:#010X}");
    }

    fn coprocessor_data_operation(&self, opcode: u32) -> Result<(), CpuError> {
        todo!("CoprocessorDataOperation {opcode:#010X}");
    }

    fn coprocessor_register_transfer(&self, opcode: u32) -> Result<(), CpuError> {
        todo!("CoprocessorRegisterTransfer {opcode:#010X}");
    }

    fn arm_software_interrupt(&mut self) -> Result<(), CpuError> {
        self.registers.link_register = self.registers.program_counter;
        self.registers
            .status_registers
            .set_supervisor(self.registers.current())?;
        self.registers.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.registers.set_mode(CpuMode::Supervisor);
        Ok(())
    }

    pub fn thumb_cycle(&mut self, bus: &mut Bus) -> Result<(), CpuError> {
        let opcode = bus.read_16(usize::try_from(self.registers.program_counter)?);
        let instruction = decode::thumb_instruction(opcode);
        self.registers.program_counter = self.registers.program_counter.wrapping_add(2);
        self.run_thumb_instruction(instruction, opcode, bus)
    }

    fn run_thumb_instruction(
        &mut self,
        instruction: ThumbInstruction,
        opcode: u16,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        match instruction {
            ThumbInstruction::SoftwareInterrupt => self.thumb_software_interrupt(),
            ThumbInstruction::UnconditionalBranch(offset) => self.unconditional_branch(offset),
            ThumbInstruction::ConditionalBranch { conditions, offset } => {
                self.conditional_branch(conditions, offset)
            }
            ThumbInstruction::MultipleLoadstore(operands) => self.multiple_loadstore(operands, bus),
            ThumbInstruction::LongBranchWithLink { h, offset } => {
                self.long_branch_with_link(h, offset)
            }
            ThumbInstruction::AddOffsetToStackPointer { subtract, offset } => {
                self.add_offset_to_stack_pointer(subtract, offset);
                Ok(())
            }
            ThumbInstruction::PushPopRegisters(operands) => self.push_pop_registers(operands, bus),
            ThumbInstruction::LoadStoreHalfword(operands) => {
                self.load_store_halfword(operands, bus)
            }
            ThumbInstruction::SpRelativeLoadStore(operands) => {
                self.sp_relative_load_store(operands, bus)
            }
            ThumbInstruction::LoadAddress(operands) => self.load_address(operands),
            ThumbInstruction::LoadStoreWithOffset(operands) => {
                self.load_store_with_offset(operands, bus)
            }
            ThumbInstruction::LoadStoreSignExtendedByteHalfword(operands) => {
                self.load_store_sign_extended_byte_halfword(operands, bus)
            }
            ThumbInstruction::PCRelativeLoad {
                destination_register,
                offset,
            } => self.pc_relative_load(destination_register, offset, bus),
            ThumbInstruction::HiRegisterOperationsBranchExchange(operands) => {
                self.hi_register_operations_branch_exchange(operands)
            }
            ThumbInstruction::AluOperations(operands) => self.alu_operations(operands),
            ThumbInstruction::MoveCompareAddSubtractImmediate(operands) => {
                self.move_compare_add_subtract_immediate(operands)
            }
            ThumbInstruction::AddSubtract(operands) => self.add_subtract(operands),
            ThumbInstruction::MoveShiftedRegister(operands) => self.move_shifted_register(operands),
            ThumbInstruction::Unimplemented => {
                Err(CpuError::UnimplementedThumbInstruction { opcode })
            }
        }
    }

    fn thumb_software_interrupt(&mut self) -> Result<(), CpuError> {
        self.registers.link_register = self.registers.program_counter;
        self.registers
            .status_registers
            .set_supervisor(self.registers.current())?;
        self.registers.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.registers.set_state(InstructionSet::Arm);
        self.registers.set_mode(CpuMode::Supervisor);
        Ok(())
    }

    fn unconditional_branch(&mut self, offset: i16) -> Result<(), CpuError> {
        self.registers.program_counter = self.registers.get(15)?.wrapping_add_signed(offset.into());
        Ok(())
    }

    fn conditional_branch(&mut self, conditions: u8, offset: i32) -> Result<(), CpuError> {
        let branch = self.registers.check_conditions(conditions)?;
        if branch {
            self.registers.program_counter = self.registers.get(15)?.wrapping_add_signed(offset);
        }
        Ok(())
    }

    fn multiple_loadstore(
        &mut self,
        operands: MultipleLoadstoreOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        if operands.load {
            self.multiple_load(operands, bus)
        } else {
            self.multiple_store(operands, bus)
        }
    }

    fn multiple_load(
        &mut self,
        operands: MultipleLoadstoreOperands,
        bus: &Bus,
    ) -> Result<(), CpuError> {
        let mut address = usize::try_from(self.registers.get(operands.base_register)?)?;
        for register in (0..=7).filter(|r| operands.register_list & (1 << r) > 0) {
            self.registers.set(register, bus.read_32(address))?;
            address = address.wrapping_add(4);
        }
        self.registers
            .set(operands.base_register, u32::try_from(address)?)?;
        Ok(())
    }

    fn multiple_store(
        &mut self,
        operands: MultipleLoadstoreOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let mut address = usize::try_from(self.registers.get(operands.base_register)?)?;
        for register in (0..=7).filter(|r| operands.register_list & (1 << r) > 0) {
            bus.write_32(address, self.registers.get(register)?);
            address = address.wrapping_add(4);
        }
        self.registers
            .set(operands.base_register, u32::try_from(address)?)?;
        Ok(())
    }

    fn long_branch_with_link(&mut self, h: bool, offset: u32) -> Result<(), CpuError> {
        if h {
            let return_address = self.registers.program_counter;
            self.registers.link_register = self.registers.link_register.wrapping_add(offset);
            self.registers.program_counter = self.registers.link_register;
            self.registers.link_register = return_address | 1;
        } else {
            self.registers.link_register = self
                .registers
                .get(15)?
                .wrapping_add(sign_extend(offset, 12) << 11);
        }
        Ok(())
    }

    const fn add_offset_to_stack_pointer(&mut self, subtract: bool, offset: i32) {
        if subtract {
            self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub_signed(offset);
        } else {
            self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add_signed(offset);
        }
    }

    fn push_pop_registers(
        &mut self,
        operands: PushPopRegistersOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        if operands.load {
            self.pop_registers(operands, bus)
        } else {
            self.push_registers(operands, bus)
        }
    }

    fn pop_registers(
        &mut self,
        operands: PushPopRegistersOperands,
        bus: &Bus,
    ) -> Result<(), CpuError> {
        for register in (0..=7).filter(|r| operands.register_list & (1 << r) > 0) {
            let val = self.pop(bus)?;
            self.registers.set(register, val)?;
        }
        if operands.pc_lr {
            self.registers.program_counter = self.pop(bus)?;
        }
        Ok(())
    }

    fn push_registers(
        &mut self,
        operands: PushPopRegistersOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        if operands.pc_lr {
            self.push(self.registers.link_register, bus)?;
        }
        for register in (0..=7)
            .rev()
            .filter(|r| operands.register_list & (1 << r) > 0)
        {
            self.push(self.registers.get(register)?, bus)?;
        }
        Ok(())
    }

    fn pop(&mut self, bus: &Bus) -> Result<u32, CpuError> {
        let value = bus.read_32(usize::try_from(self.registers.stack_pointer)?);
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_add(4);
        Ok(value)
    }

    fn push(&mut self, value: u32, bus: &mut Bus) -> Result<(), CpuError> {
        self.registers.stack_pointer = self.registers.stack_pointer.wrapping_sub(4);
        bus.write_32(usize::try_from(self.registers.stack_pointer)?, value);
        Ok(())
    }

    fn load_store_halfword(
        &mut self,
        operands: LoadStoreHalfwordOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let address = usize::try_from(
            self.registers
                .get(operands.base_register)?
                .wrapping_add(u32::from(operands.offset)),
        )?;
        if operands.load {
            self.registers.set(
                operands.source_destination_register,
                u32::from(bus.read_16(address)),
            )?;
        } else {
            bus.write_16(
                address,
                self.registers
                    .get(operands.source_destination_register)?
                    .truncate(),
            );
        }
        Ok(())
    }

    fn sp_relative_load_store(
        &mut self,
        operands: SpRelativeLoadStoreOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        if operands.load {
            self.registers.set(
                operands.destination_register,
                bus.read_32(usize::try_from(
                    self.registers
                        .stack_pointer
                        .wrapping_add(operands.offset.into()),
                )?),
            )?;
        } else {
            bus.write_32(
                usize::try_from(
                    self.registers
                        .stack_pointer
                        .wrapping_add(operands.offset.into()),
                )?,
                self.registers.get(operands.destination_register)?,
            );
        }
        Ok(())
    }

    fn load_address(&mut self, operands: LoadAddressOperands) -> Result<(), CpuError> {
        let address = if operands.source {
            self.registers
                .stack_pointer
                .wrapping_add(operands.word.into())
        } else {
            self.registers.get(15)?.wrapping_add(operands.word.into()) & !3
        };
        self.registers.set(operands.destination_register, address)?;
        Ok(())
    }

    fn load_store_with_offset(
        &mut self,
        operands: LoadStoreWithOffsetOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let address = match (operands.offset, operands.byte) {
            (Offset::Register(register), _) => usize::try_from(
                self.registers
                    .get(operands.base_register)?
                    .wrapping_add(self.registers.get(register)?),
            )?,
            (Offset::Immediate(offset), true) => usize::try_from(
                self.registers
                    .get(operands.base_register)?
                    .wrapping_add(offset),
            )?,
            (Offset::Immediate(offset), false) => usize::try_from(
                self.registers
                    .get(operands.base_register)?
                    .wrapping_add(u32::from(offset << 2)),
            )?,
        };
        match (operands.load, operands.byte) {
            (true, true) => self.registers.set(
                operands.source_destination_register,
                u32::from(bus.read_8(address)),
            )?,
            (true, false) => self
                .registers
                .set(operands.source_destination_register, bus.read_32(address))?,
            (false, true) => bus.write_8(
                address,
                self.registers
                    .get(operands.source_destination_register)?
                    .truncate(),
            ),
            (false, false) => {
                bus.write_32(
                    address,
                    self.registers.get(operands.source_destination_register)?,
                );
            }
        }
        Ok(())
    }

    fn load_store_sign_extended_byte_halfword(
        &mut self,
        operands: LoadStoreSignExtendedByteHalfwordOperands,
        bus: &mut Bus,
    ) -> Result<(), CpuError> {
        let address = usize::try_from(
            self.registers
                .get(operands.base_register)?
                .wrapping_add(self.registers.get(operands.offset_register)?),
        )?;
        match (operands.signed, operands.halfword) {
            (false, false) => bus.write_16(
                address,
                self.registers
                    .get(operands.source_destination_register)?
                    .truncate(),
            ),
            (false, true) => {
                self.registers.set(
                    operands.source_destination_register,
                    u32::from(bus.read_16(address)),
                )?;
            }
            (true, false) => self.registers.set(
                operands.source_destination_register,
                sign_extend(bus.read_8(address).into(), 8),
            )?,
            (true, true) => self.registers.set(
                operands.source_destination_register,
                sign_extend(bus.read_16(address).into(), 16),
            )?,
        }
        Ok(())
    }

    fn pc_relative_load(
        &mut self,
        destination_register: u16,
        offset: u8,
        bus: &Bus,
    ) -> Result<(), CpuError> {
        let value = bus.read_32(usize::try_from(
            (self.registers.get(15)? & !1).wrapping_add(u32::from(offset)),
        )?);
        self.registers.set(destination_register, value)?;
        Ok(())
    }

    fn hi_register_operations_branch_exchange(
        &mut self,
        operands: ThumbOperands,
    ) -> Result<(), CpuError> {
        let source = self.registers.get(operands.source_register)?;
        let destination = self.registers.get(operands.destination_register)?;
        match operands.sub_opcode {
            0b00 => self.registers.set(
                operands.destination_register,
                alu::add(source, destination).result,
            )?,
            0b01 => self
                .registers
                .status_registers
                .set_conditions(alu::subtract(destination, source)),
            0b10 => self.registers.set(operands.destination_register, source)?,
            0b11 => {
                let mode = if source & 0x01 > 0 {
                    InstructionSet::Thumb
                } else {
                    InstructionSet::Arm
                };
                self.registers.set_state(mode);
                self.registers.program_counter = source & !1;
            }
            _ => {
                return Err(
                    CpuError::UnreachableHiRegisterOperationsBranchExchangeSubOpcode {
                        sub_opcode: operands.sub_opcode,
                    },
                );
            }
        }
        Ok(())
    }

    fn alu_operations(&mut self, operands: ThumbOperands) -> Result<(), CpuError> {
        let alu_result = match operands.sub_opcode {
            0x00 | 0x08 => alu::and(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x01 => alu::xor(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x02 => alu::shift_left(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x03 => alu::shift_right(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x04 => alu::arithmetic_shift_right(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x05 => alu::add_carry(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
                self.registers.carry().into(),
            ),
            0x06 => alu::sub_carry(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
                self.registers.carry().into(),
            ),
            0x07 => alu::rotate_right(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x09 => alu::subtract(0, self.registers.get(operands.source_register)?),
            0x0A => alu::subtract(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x0B => alu::add(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x0C => alu::or(
                self.registers.get(operands.destination_register)?,
                self.registers.get(operands.source_register)?,
            ),
            0x0D => AluResult {
                result: self
                    .registers
                    .get(operands.destination_register)?
                    .wrapping_mul(self.registers.get(operands.source_register)?),
                carry: None,
                overflow: None,
            },
            0x0E => alu::and(
                self.registers.get(operands.destination_register)?,
                !self.registers.get(operands.source_register)?,
            ),
            0x0F => alu::nop(!self.registers.get(operands.source_register)?),
            _ => {
                return Err(CpuError::UnreachableAluOperationsSubOpcode {
                    sub_opcode: operands.sub_opcode,
                });
            }
        };
        self.registers.status_registers.set_conditions(alu_result);
        if !matches!(operands.sub_opcode, 0x08 | 0x0A | 0x0B) {
            self.registers
                .set(operands.destination_register, alu_result.result)?;
        }
        Ok(())
    }

    fn move_compare_add_subtract_immediate(
        &mut self,
        operands: MoveCompareAddSubtractImmediateOperands,
    ) -> Result<(), CpuError> {
        let result = match operands.operation {
            MoveCompareAddSubtractOperation::Move => alu::nop(operands.immediate_value),
            MoveCompareAddSubtractOperation::Compare
            | MoveCompareAddSubtractOperation::Subtract => alu::subtract(
                self.registers.get(operands.destination_register)?,
                operands.immediate_value,
            ),
            MoveCompareAddSubtractOperation::Add => alu::add(
                self.registers.get(operands.destination_register)?,
                operands.immediate_value,
            ),
        };
        self.registers.status_registers.set_conditions(result);
        if operands.operation != MoveCompareAddSubtractOperation::Compare {
            self.registers
                .set(operands.destination_register, result.result)?;
        }
        Ok(())
    }

    fn add_subtract(&mut self, operands: AddSubtractOperands) -> Result<(), CpuError> {
        let result = match operands.operation {
            AddSubtractOperation::AddRegister => alu::add(
                self.registers.get(operands.source_register)?,
                self.registers.get(operands.register_3)?,
            ),
            AddSubtractOperation::AddImmediate => alu::add(
                self.registers.get(operands.source_register)?,
                operands.immediate_value,
            ),
            AddSubtractOperation::SubtractRegister => alu::subtract(
                self.registers.get(operands.source_register)?,
                self.registers.get(operands.register_3)?,
            ),
            AddSubtractOperation::SubtractImmediate => alu::subtract(
                self.registers.get(operands.source_register)?,
                operands.immediate_value,
            ),
        };
        self.registers.status_registers.set_conditions(result);
        self.registers
            .set(operands.destination_register, result.result)?;
        Ok(())
    }

    fn move_shifted_register(
        &mut self,
        operands: MoveShiftedRegisterOperands,
    ) -> Result<(), CpuError> {
        let result = match operands.opcode {
            0b00 => alu::shift_left(
                self.registers.get(operands.source_register)?,
                operands.immediate_value,
            ),
            0b01 => alu::shift_right(
                self.registers.get(operands.source_register)?,
                operands.immediate_value,
            ),
            0b10 => alu::arithmetic_shift_right(
                self.registers.get(operands.source_register)?,
                operands.immediate_value,
            ),
            _ => {
                return Err(CpuError::UnreachableMoveShiftedRegisterSubOpcode {
                    sub_opcode: operands.opcode.into(),
                });
            }
        };
        self.registers.status_registers.set_conditions(result);
        self.registers
            .set(operands.destination_register, result.result)?;
        Ok(())
    }
}

fn conditions(opcode: u32) -> u8 {
    const MASK: u32 = 0xF000_0000;
    const SHIFT: usize = 28;
    ((opcode & MASK) >> SHIFT).truncate()
}

const fn sign_extend(value: u32, size: u32) -> u32 {
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
