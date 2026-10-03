#![allow(dead_code)]

use crate::{
    bus::Bus,
    program_status_register::{CpuMode, Mode, ProgramStatusRegister},
};

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
    current_program_status_register: ProgramStatusRegister,
    saved_program_status_register: ProgramStatusRegister,
}

impl Cpu {
    //temp
    pub const fn r00(&self) -> u32 {
        self.r00
    }

    //temp
    pub const fn r01(&self) -> u32 {
        self.r01
    }

    //temp
    pub const fn set_r00(&mut self, value: u32) {
        self.r00 = value;
    }

    pub const fn program_counter(&self) -> u32 {
        self.program_counter
    }

    pub fn next_instruction(&self, bus: &Bus) -> (u16, ThumbInstruction) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_thumb_instruction(opcode);
        (opcode, instruction)
    }

    pub fn cpu_cycle(&mut self, bus: &mut Bus) {
        match self.current_program_status_register.state() {
            CpuMode::Arm => self.arm_cycle(bus),
            CpuMode::Thumb => self.thumb_cycle(bus),
        }
    }

    pub fn arm_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_32(self.program_counter as usize);
        let instruction = decode_arm_instruction(opcode);
        self.program_counter = self.program_counter.wrapping_add(4);
        self.run_arm_instruction(instruction, opcode, bus);
    }

    fn run_arm_instruction(&self, _instruction: u32, _opcode: u32, _bus: &mut Bus) {
        todo!();
    }

    pub fn thumb_cycle(&mut self, bus: &mut Bus) {
        let opcode = bus.read_16(self.program_counter as usize);
        let instruction = decode_thumb_instruction(opcode);
        self.program_counter = self.program_counter.wrapping_add(2);
        self.run_thumb_instruction(instruction, opcode, bus);
    }

    fn run_thumb_instruction(&mut self, instruction: ThumbInstruction, opcode: u16, bus: &mut Bus) {
        match instruction {
            ThumbInstruction::SoftwareInterrupt => self.software_interrupt(),
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
            ThumbInstruction::Unimplemented => unreachable!(),
        }
    }

    const fn software_interrupt(&mut self) {
        const SOFTWARE_INTERRUPT_ADDRESS: u32 = 0x0000_0008;
        self.link_register = self.program_counter;
        self.saved_program_status_register = self.current_program_status_register;
        self.program_counter = SOFTWARE_INTERRUPT_ADDRESS;
        self.current_program_status_register.set_state(CpuMode::Arm);
        self.current_program_status_register
            .set_mode(Mode::Supervisor);
    }

    fn unconditional_branch(&mut self, opcode: u16) {
        // Offset is a 12 bit value, but is stored as 11 bits (lsb is dropped) because it must be halfword aligned
        const MASK: u16 = 0x07FF;
        let offset = ((opcode & MASK) << 1).sign_extend_12();
        self.program_counter = self.program_counter.wrapping_add_signed(offset);
    }

    fn conditional_branch(&mut self, opcode: u16) {
        const CONDITIONS_MASK: u16 = 0x0F00;
        const CONDITIONS_SHIFT: usize = 8;
        const SIGNED_OFFSET_MASK: u16 = 0x00FF;
        let conditions = (opcode & CONDITIONS_MASK) >> CONDITIONS_SHIFT;
        let offset = ((opcode & SIGNED_OFFSET_MASK) as i8) << 1;
        let branch = self.check_conditions(conditions);
        if branch {
            self.program_counter = self.program_counter.wrapping_add_signed(i32::from(offset));
        }
    }

    fn check_conditions(&self, conditions: u16) -> bool {
        match conditions {
            0b0000 => self.current_program_status_register.zero(),
            0b0001 => !self.current_program_status_register.zero(),
            0b0010 => self.current_program_status_register.carry(),
            0b0011 => !self.current_program_status_register.carry(),
            0b0100 => self.current_program_status_register.negative(),
            0b0101 => !self.current_program_status_register.negative(),
            0b0110 => self.current_program_status_register.overflow(),
            0b0111 => !self.current_program_status_register.overflow(),
            0b1000 => {
                self.current_program_status_register.carry()
                    && !self.current_program_status_register.zero()
            }
            0b1001 => {
                !self.current_program_status_register.carry()
                    || self.current_program_status_register.zero()
            }
            0b1010 => {
                self.current_program_status_register.negative()
                    == self.current_program_status_register.overflow()
            }
            0b1011 => {
                self.current_program_status_register.negative()
                    != self.current_program_status_register.overflow()
            }
            0b1100 => {
                !self.current_program_status_register.zero()
                    && (self.current_program_status_register.negative()
                        == self.current_program_status_register.overflow())
            }
            0b1101 => {
                self.current_program_status_register.zero()
                    && (self.current_program_status_register.negative()
                        != self.current_program_status_register.overflow())
            }
            _ => unreachable!(),
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
        const H_SHIFT: usize = 11;
        const OFFSET_MASK: u16 = 0x07FF;
        let h = (opcode & H_MASK) >> H_SHIFT > 0;
        // TODO: I think this needs to use an i32 instead of u32 for the offset. Not sure how to
        // implement that
        let offset = opcode & OFFSET_MASK;
        if h {
            let return_address = self.program_counter;
            self.link_register = self.link_register.wrapping_add(u32::from(offset << 1));
            self.program_counter = self.program_counter.wrapping_add(self.link_register);
            self.link_register = return_address;
        } else {
            self.link_register = self.program_counter.wrapping_add(u32::from(offset) << 12);
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
            self.program_counter.wrapping_add(word)
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
        let value = bus
            .read_32((self.program_counter & 0xFFFF_FFFE).wrapping_add(u32::from(offset)) as usize);
        self.set_register(destination_register, value);
    }

    fn hi_register_operations_branch_exchange(&mut self, opcode: u16) {
        const OPCODE_MASK: u16 = 0x0300;
        const OPCODE_SHIFT: usize = 8;
        const H1_MASK: u16 = 0x0080;
        const H2_MASK: u16 = 0x0040;
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
        let dest = if h1 {
            destination_register | 0x08
        } else {
            destination_register
        };
        let src = if h2 {
            source_register | 0x08
        } else {
            source_register
        };
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
        let dest = if h1 {
            destination_register | 0x08
        } else {
            destination_register
        };
        let src = if h2 {
            source_register | 0x08
        } else {
            source_register
        };
        self.test_sub(dest, src);
    }

    fn hi_register_mov(
        &mut self,
        h1: bool,
        h2: bool,
        source_register: u16,
        destination_register: u16,
    ) {
        let dest = if h1 {
            destination_register | 0x08
        } else {
            destination_register
        };
        let src = if h2 {
            source_register | 0x08
        } else {
            source_register
        };
        self.set_register(dest, self.register(src));
    }

    fn hi_register_bx(&mut self, h: bool, register: u16) {
        let r = if h { register | 0x08 } else { register };
        let mode = if self.register(r) & 0x01 > 0 {
            CpuMode::Thumb
        } else {
            CpuMode::Arm
        };
        self.current_program_status_register.set_state(mode);
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
        // TODO: set flags
        let value = match sub_opcode {
            0x00 => self.register(destination_register) & self.register(source_register),
            0x01 => self.register(destination_register) ^ self.register(source_register),
            0x02 => self.register(destination_register) << self.register(source_register),
            0x03 => self.register(destination_register) >> self.register(source_register),
            0x04 => (self.register(destination_register).cast_signed()
                >> self.register(source_register))
            .cast_unsigned(),
            0x05 => self
                .register(destination_register)
                .wrapping_add(self.register(source_register))
                .wrapping_add(u32::from(self.current_program_status_register.carry())),
            0x06 => self
                .register(destination_register)
                .wrapping_sub(self.register(source_register))
                .wrapping_sub(u32::from(!self.current_program_status_register.carry())),
            0x07 => self.ror(destination_register, source_register),
            0x08 => {
                self.test_and(destination_register, source_register);
                self.register(destination_register)
            }
            0x09 => 0u32.wrapping_sub(self.register(source_register)),
            0x0A => {
                self.test_sub(destination_register, source_register);
                self.register(destination_register)
            }
            0x0B => {
                self.test_add(destination_register, source_register);
                self.register(destination_register)
            }
            0x0C => self.register(destination_register) | self.register(source_register),
            0x0D => self
                .register(destination_register)
                .wrapping_mul(self.register(source_register)),
            0x0E => self.register(destination_register) & !self.register(source_register),
            0x0F => !self.register(source_register),
            _ => unreachable!(),
        };
        self.set_register(destination_register, value);
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
        // TODO: set flags
        match sub_opcode {
            0b00 => self.set_register(destination_register, u32::from(immediate_value)),
            0b01 => self.test_cmp(destination_register, u32::from(immediate_value)),
            0b10 => self.set_register(
                destination_register,
                self.register(destination_register)
                    .wrapping_add(u32::from(immediate_value)),
            ),
            0b11 => self.set_register(
                destination_register,
                self.register(destination_register)
                    .wrapping_sub(u32::from(immediate_value)),
            ),
            _ => unreachable!(),
        }
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
        // TODO: set flags
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
        // TODO: set flags during shifts
        let value = match sub_opcode {
            0b00 => self.register(source_register) << immediate_value,
            0b01 => self.register(source_register) >> immediate_value,
            // Arithmatic shift (signed right shift)
            0b10 => {
                (self.register(source_register).cast_signed() >> immediate_value).cast_unsigned()
            }
            _ => unreachable!(),
        };
        self.set_register(destination_register, value);
    }

    fn register(&self, register: u16) -> u32 {
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
            let temp = u32::from(self.current_program_status_register.carry());
            self.current_program_status_register
                .set_carry(rotated & 0x01 > 0);
            rotated = (rotated >> 1) | (temp << 31);
        }
        rotated
    }

    fn test_and(&mut self, source_register: u16, destination_register: u16) {
        let temp = self.register(source_register) & self.register(destination_register);
        self.current_program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.current_program_status_register
            .set_zero(temp == 0x0000_0000);
    }

    fn test_add(&mut self, source_register: u16, destination_register: u16) {
        let (temp, carry) = self
            .register(source_register)
            .overflowing_add(self.register(destination_register));
        self.current_program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.current_program_status_register
            .set_zero(temp == 0x0000_0000);
        self.current_program_status_register.set_carry(carry);
        self.current_program_status_register.set_overflow(carry); // TODO: fix this
    }

    fn test_sub(&mut self, source_register: u16, destination_register: u16) {
        let (temp, overflow) = self
            .register(source_register)
            .overflowing_sub(self.register(destination_register));
        self.current_program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.current_program_status_register
            .set_zero(temp == 0x0000_0000);
        self.current_program_status_register
            .set_carry(self.register(source_register) >= self.register(destination_register));
        self.current_program_status_register.set_overflow(overflow);
    }

    fn test_cmp(&mut self, source_register: u16, immediate: u32) {
        let (temp, overflow) = self.register(source_register).overflowing_sub(immediate);
        self.current_program_status_register
            .set_negative(temp & 0x8000_0000 == 0x8000_0000);
        self.current_program_status_register
            .set_zero(temp == 0x0000_0000);
        self.current_program_status_register
            .set_carry(self.register(source_register) >= immediate);
        self.current_program_status_register.set_overflow(overflow);
    }
}

pub enum ArmInstruction {
    DataProcessingPsrTransfer,
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

const ARM_DECODE_TABLE: [(u32, u32, ArmInstruction); 2] = [
    (0x0FFF_FFF0, 0x012F_FF10, ArmInstruction::BranchAndExchange),
    (0x000000000, 0x000000000, ArmInstruction::Undefined), // TODO: fix
];

fn decode_arm_instruction(_opcode: u32) -> u32 {
    todo!();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_shifted_register_sets_register() {
        let mut cpu = Cpu::default();
        let mut bus = Bus::new();

        cpu.r00 = 0xFFFF_FFFF;
        bus.write_16(cpu.program_counter() as usize, 0x0001);
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
