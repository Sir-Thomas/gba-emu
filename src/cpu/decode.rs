#[derive(Clone, Copy, Debug)]
pub enum Instruction {
    Arm(ArmInstruction),
    Thumb(ThumbInstruction),
}

#[derive(Clone, Copy, Debug)]
pub enum ArmInstruction {
    DataProcessing(DataProcessingOperands),
    PsrTransferMrs {
        saved: bool,
        destination_register: u16,
    },
    PsrTransferMsr(PsrTransferMsrOperands),
    Multiply(MultiplyOperands),
    MultiplyLong(MultiplyLongOperands),
    SingleDataSwap(DataSwapOperands),
    BranchAndExchange {
        register: u16,
    },
    HalfwordDataTransferRegisterOffset {
        operands: HalfwordDataTransferOperands,
        register: u16,
    },
    HalfwordDataTransferImmediateOffset {
        operands: HalfwordDataTransferOperands,
        offset: u32,
    },
    SingleDataTransfer(SingleDataTransferOperands),
    Undefined,
    BlockDataTransfer(BlockDataTransferOperands),
    Branch {
        link: bool,
        offset: i32,
    },
    CoprocessorDataTransfer,
    CoprocessorDataOperation,
    CoprocessorRegisterTransfer,
    SoftwareInterrupt,
}

#[derive(Debug, Clone, Copy)]
struct ArmDecoder {
    mask: u32,
    value: u32,
    instruction: fn(u32) -> ArmInstruction,
}

const ARM_DECODE_TABLE: [ArmDecoder; 14] = [
    ArmDecoder {
        mask: 0x0FFF_FFF0,
        value: 0x012F_FF10,
        instruction: branch_and_exchange,
    },
    ArmDecoder {
        mask: 0x0E00_0000,
        value: 0x0800_0000,
        instruction: block_data_transfer,
    },
    ArmDecoder {
        mask: 0x0E00_0000,
        value: 0x0A00_0000,
        instruction: branch,
    },
    ArmDecoder {
        mask: 0x0F00_0000,
        value: 0x0F00_0000,
        instruction: |_| ArmInstruction::SoftwareInterrupt,
    },
    ArmDecoder {
        mask: 0x0E00_0010,
        value: 0x0600_0010,
        instruction: |_| ArmInstruction::Undefined,
    },
    ArmDecoder {
        mask: 0x0C00_0000,
        value: 0x0400_0000,
        instruction: single_data_transfer,
    },
    ArmDecoder {
        mask: 0x0F80_0FF0,
        value: 0x0100_0090,
        instruction: single_data_swap,
    },
    ArmDecoder {
        mask: 0x0F80_00F0,
        value: 0x0000_0090,
        instruction: multiply,
    },
    ArmDecoder {
        mask: 0x0F80_00F0,
        value: 0x0080_0090,
        instruction: multiply_long,
    },
    ArmDecoder {
        mask: 0x0E40_0F90,
        value: 0x0000_0090,
        instruction: halfword_data_transfer_register_offset,
    },
    ArmDecoder {
        mask: 0x0E40_0090,
        value: 0x0040_0090,
        instruction: halfword_data_transfer_immediate_offset,
    },
    ArmDecoder {
        mask: 0x0FBF_0000,
        value: 0x010F_0000,
        instruction: psr_transfer_mrs,
    },
    ArmDecoder {
        mask: 0x0DB0_F000,
        value: 0x0120_F000,
        instruction: psr_transfer_msr,
    },
    ArmDecoder {
        mask: 0x0C00_0000,
        value: 0x0000_0000,
        instruction: data_processing,
    },
];

pub fn arm_instruction(opcode: u32) -> ArmInstruction {
    ARM_DECODE_TABLE
        .iter()
        .find(|decoder| opcode & decoder.mask == decoder.value)
        .map_or(ArmInstruction::Undefined, |decoder| {
            (decoder.instruction)(opcode)
        })
}

fn branch_and_exchange(opcode: u32) -> ArmInstruction {
    let register = bits32::<3, 0>(opcode).truncate();
    ArmInstruction::BranchAndExchange { register }
}

#[derive(Clone, Copy, Debug)]
pub struct BlockDataTransferOperands {
    pub pre_index: bool,
    pub up: bool,
    pub psr: bool,
    pub write_back: bool,
    pub load: bool,
    pub base_register: u16,
    pub register_list: u16,
    pub byte_count: u32,
}

fn block_data_transfer(opcode: u32) -> ArmInstruction {
    let pre_index = bit32::<24>(opcode);
    let up = bit32::<23>(opcode);
    let psr = bit32::<22>(opcode);
    let write_back = bit32::<21>(opcode);
    let load = bit32::<20>(opcode);
    let base_register = bits32::<19, 16>(opcode).truncate();
    let mut register_list: u16 = bits32::<15, 0>(opcode).truncate();
    let mut byte_count = u32::from(register_list.count_ones().wrapping_mul(4));
    // Special Hardware Edge Case
    if register_list == 0 {
        register_list = 1 << 15;
        byte_count = 16 * 4;
    }
    ArmInstruction::BlockDataTransfer(BlockDataTransferOperands {
        pre_index,
        up,
        psr,
        write_back,
        load,
        base_register,
        register_list,
        byte_count,
    })
}

const fn branch(opcode: u32) -> ArmInstruction {
    let link = bit32::<24>(opcode);
    let offset = signed_bits32::<23, 0>(opcode) << 2;
    ArmInstruction::Branch { link, offset }
}

#[derive(Clone, Copy, Debug)]
pub struct SingleDataTransferOperands {
    pub immediate: bool,
    pub pre_index: bool,
    pub up: bool,
    pub byte: bool,
    pub write_back: bool,
    pub load: bool,
    pub base_register: u16,
    pub source_destination_register: u16,
    pub offset: u32,
}

fn single_data_transfer(opcode: u32) -> ArmInstruction {
    let immediate = !bit32::<25>(opcode);
    let pre_index = bit32::<24>(opcode);
    let up = bit32::<23>(opcode);
    let byte = bit32::<22>(opcode);
    let write_back = bit32::<21>(opcode);
    let load = bit32::<20>(opcode);
    let base_register = bits32::<19, 16>(opcode).truncate();
    let source_destination_register = bits32::<15, 12>(opcode).truncate();
    let offset = bits32::<11, 0>(opcode);
    ArmInstruction::SingleDataTransfer(SingleDataTransferOperands {
        immediate,
        pre_index,
        up,
        byte,
        write_back,
        load,
        base_register,
        source_destination_register,
        offset,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct DataSwapOperands {
    pub byte: bool,
    pub base_register: u16,
    pub destination_register: u16,
    pub source_register: u16,
}

fn single_data_swap(opcode: u32) -> ArmInstruction {
    let byte = bit32::<22>(opcode);
    let base_register = bits32::<19, 16>(opcode).truncate();
    let destination_register = bits32::<15, 12>(opcode).truncate();
    let source_register = bits32::<3, 0>(opcode).truncate();
    ArmInstruction::SingleDataSwap(DataSwapOperands {
        byte,
        base_register,
        destination_register,
        source_register,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct MultiplyOperands {
    pub accumulate: bool,
    pub set_conditions: bool,
    pub destination_register: u16,
    pub source_register_n: u16,
    pub source_register_s: u16,
    pub source_register_m: u16,
}

fn multiply(opcode: u32) -> ArmInstruction {
    let accumulate = bit32::<21>(opcode);
    let set_conditions = bit32::<20>(opcode);
    let destination_register = bits32::<19, 16>(opcode).truncate();
    let source_register_n = bits32::<15, 12>(opcode).truncate();
    let source_register_s = bits32::<11, 8>(opcode).truncate();
    let source_register_m = bits32::<3, 0>(opcode).truncate();
    ArmInstruction::Multiply(MultiplyOperands {
        accumulate,
        set_conditions,
        destination_register,
        source_register_n,
        source_register_s,
        source_register_m,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct MultiplyLongOperands {
    pub signed: bool,
    pub accumulate: bool,
    pub set_conditions: bool,
    pub destination_register_hi: u16,
    pub destination_register_low: u16,
    pub source_register_s: u16,
    pub source_register_m: u16,
}

fn multiply_long(opcode: u32) -> ArmInstruction {
    let signed = bit32::<22>(opcode);
    let accumulate = bit32::<21>(opcode);
    let set_conditions = bit32::<20>(opcode);
    let destination_register_hi = bits32::<19, 16>(opcode).truncate();
    let destination_register_low = bits32::<15, 12>(opcode).truncate();
    let source_register_s = bits32::<11, 8>(opcode).truncate();
    let source_register_m = bits32::<3, 0>(opcode).truncate();
    ArmInstruction::MultiplyLong(MultiplyLongOperands {
        signed,
        accumulate,
        set_conditions,
        destination_register_hi,
        destination_register_low,
        source_register_s,
        source_register_m,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct HalfwordDataTransferOperands {
    pub pre_index: bool,
    pub up: bool,
    pub write_back: bool,
    pub load: bool,
    pub base_register: u16,
    pub source_destination_register: u16,
    pub signed: bool,
    pub halfword: bool,
}

fn halfword_data_transfer_register_offset(opcode: u32) -> ArmInstruction {
    let pre_index = bit32::<24>(opcode);
    let up = bit32::<23>(opcode);
    let write_back = bit32::<21>(opcode);
    let load = bit32::<20>(opcode);
    let base_register = bits32::<19, 16>(opcode).truncate();
    let source_destination_register = bits32::<15, 12>(opcode).truncate();
    let signed = bit32::<6>(opcode);
    let halfword = bit32::<5>(opcode);
    let register = bits32::<3, 0>(opcode).truncate();
    ArmInstruction::HalfwordDataTransferRegisterOffset {
        operands: HalfwordDataTransferOperands {
            pre_index,
            up,
            write_back,
            load,
            base_register,
            source_destination_register,
            signed,
            halfword,
        },
        register,
    }
}

fn halfword_data_transfer_immediate_offset(opcode: u32) -> ArmInstruction {
    let pre_index = bit32::<24>(opcode);
    let up = bit32::<23>(opcode);
    let write_back = bit32::<21>(opcode);
    let load = bit32::<20>(opcode);
    let base_register = bits32::<19, 16>(opcode).truncate();
    let source_destination_register = bits32::<15, 12>(opcode).truncate();
    let signed = bit32::<6>(opcode);
    let halfword = bit32::<5>(opcode);
    let offset = ((bits32::<11, 8>(opcode) << 4) | bits32::<3, 0>(opcode)).truncate();
    ArmInstruction::HalfwordDataTransferImmediateOffset {
        operands: HalfwordDataTransferOperands {
            pre_index,
            up,
            write_back,
            load,
            base_register,
            source_destination_register,
            signed,
            halfword,
        },
        offset,
    }
}

fn psr_transfer_mrs(opcode: u32) -> ArmInstruction {
    let saved = bit32::<22>(opcode);
    let destination_register = bits32::<15, 12>(opcode).truncate();
    ArmInstruction::PsrTransferMrs {
        saved,
        destination_register,
    }
}

#[derive(Clone, Copy, Debug)]
pub enum PsrTransferValue {
    Immediate(u32),
    Register(u16),
}

#[derive(Clone, Copy, Debug)]
pub struct PsrTransferMsrOperands {
    pub saved: bool,
    pub mode_included: bool,
    pub value: PsrTransferValue,
}

fn psr_transfer_msr(opcode: u32) -> ArmInstruction {
    let immediate = bit32::<25>(opcode);
    let saved = bit32::<22>(opcode);
    let mode_included = bit32::<16>(opcode);
    let value = if immediate {
        let rotate = bits32::<11, 8>(opcode);
        PsrTransferValue::Immediate(bits32::<7, 0>(opcode).rotate_right(rotate.wrapping_mul(2)))
    } else {
        PsrTransferValue::Register(bits32::<3, 0>(opcode).truncate())
    };
    ArmInstruction::PsrTransferMsr(PsrTransferMsrOperands {
        saved,
        mode_included,
        value,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum DataProcessingOperation {
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
    fn from(operation: u32) -> DataProcessingOperation {
        match operation & 0x0F {
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
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SecondOperand {
    Immediate { rotate: u8, value: u8 },
    Shift(u16),
}

#[derive(Clone, Copy, Debug)]
pub struct DataProcessingOperands {
    pub immediate: bool,
    pub operation: DataProcessingOperation,
    pub destination_register: u16,
    pub set_conditions: bool,
    pub state_change: bool,
    pub first_operand_register: u16,
    pub second_operand: SecondOperand,
    pub add_four: bool,
}

fn data_processing(opcode: u32) -> ArmInstruction {
    let immediate = bit32::<25>(opcode);
    let operation = DataProcessingOperation::from(bits32::<24, 21>(opcode));
    let destination_register = bits32::<15, 12>(opcode).truncate();
    let set_conditions = bit32::<20>(opcode) && destination_register != 15;
    let state_change = bit32::<20>(opcode) && destination_register == 15;
    let first_operand_register = bits32::<19, 16>(opcode).truncate();
    let second_operand = parse_second_operand(immediate, bits32::<11, 0>(opcode).truncate());
    let add_four = first_operand_register == 15 && !immediate && opcode & (1 << 4) > 0;
    ArmInstruction::DataProcessing(DataProcessingOperands {
        immediate,
        operation,
        destination_register,
        set_conditions,
        state_change,
        first_operand_register,
        second_operand,
        add_four,
    })
}

fn parse_second_operand(immediate: bool, operand: u32) -> SecondOperand {
    if immediate {
        let rotate = bits32::<11, 8>(operand).wrapping_mul(2).truncate();
        let value = bits32::<7, 0>(operand).truncate();
        SecondOperand::Immediate { rotate, value }
    } else {
        SecondOperand::Shift(operand.truncate())
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ThumbInstruction {
    SoftwareInterrupt,
    UnconditionalBranch(i16),
    ConditionalBranch {
        conditions: u8,
        offset: i32,
    },
    MultipleLoadstore(MultipleLoadstoreOperands),
    LongBranchWithLink {
        h: bool,
        offset: u32,
    },
    AddOffsetToStackPointer {
        subtract: bool,
        offset: i32,
    },
    PushPopRegisters(PushPopRegistersOperands),
    LoadStoreHalfword(LoadStoreHalfwordOperands),
    SpRelativeLoadStore(SpRelativeLoadStoreOperands),
    LoadAddress(LoadAddressOperands),
    LoadStoreWithOffset(LoadStoreWithOffsetOperands),
    LoadStoreSignExtendedByteHalfword(LoadStoreSignExtendedByteHalfwordOperands),
    PCRelativeLoad {
        destination_register: u16,
        offset: u8,
    },
    HiRegisterOperationsBranchExchange(ThumbOperands),
    AluOperations(ThumbOperands),
    MoveCompareAddSubtractImmediate(MoveCompareAddSubtractImmediateOperands),
    AddSubtract(AddSubtractOperands),
    MoveShiftedRegister(MoveShiftedRegisterOperands),
    Unimplemented,
}

#[derive(Debug, Clone, Copy)]
struct ThumbDecoder {
    mask: u16,
    value: u16,
    instruction: fn(u16) -> ThumbInstruction,
}

const THUMB_DECODE_TABLE: [ThumbDecoder; 19] = [
    ThumbDecoder {
        mask: 0xFF00,
        value: 0xDF00,
        instruction: |_| ThumbInstruction::SoftwareInterrupt,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0xE000,
        instruction: unconditional_branch,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xD000,
        instruction: conditional_branch,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xC000,
        instruction: multiple_loadstore,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xF000,
        instruction: long_branch_with_link,
    },
    ThumbDecoder {
        mask: 0xFF00,
        value: 0xB000,
        instruction: add_offset_to_stack_pointer,
    },
    ThumbDecoder {
        mask: 0xF600,
        value: 0xB400,
        instruction: push_pop_registers,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0x8000,
        instruction: load_store_halfword,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0x9000,
        instruction: sp_relative_load_store,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xA000,
        instruction: load_address,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x6000,
        instruction: load_store_with_immediate_offset,
    },
    ThumbDecoder {
        mask: 0xF200,
        value: 0x5000,
        instruction: load_store_with_register_offset,
    },
    ThumbDecoder {
        mask: 0xF200,
        value: 0x5200,
        instruction: load_store_sign_extended_byte_halfword,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0x4800,
        instruction: pc_relative_load,
    },
    ThumbDecoder {
        mask: 0xFC00,
        value: 0x4400,
        instruction: hi_register_operations_branch_exchange,
    },
    ThumbDecoder {
        mask: 0xFC00,
        value: 0x4000,
        instruction: alu_operations,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x2000,
        instruction: move_compare_add_subtract_immediate,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0x1800,
        instruction: add_subtract,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x0000,
        instruction: move_shifted_register,
    },
];

pub fn thumb_instruction(opcode: u16) -> ThumbInstruction {
    THUMB_DECODE_TABLE
        .iter()
        .find(|decoder| opcode & decoder.mask == decoder.value)
        .map_or(ThumbInstruction::Unimplemented, |decoder| {
            (decoder.instruction)(opcode)
        })
}

const fn unconditional_branch(opcode: u16) -> ThumbInstruction {
    // Offset is a 12 bit value, but is stored as 11 bits (lsb is dropped) because it must be halfword aligned
    let offset = signed_bits16::<10, 0>(opcode) << 1;
    ThumbInstruction::UnconditionalBranch(offset)
}

fn conditional_branch(opcode: u16) -> ThumbInstruction {
    let conditions = bits16::<11, 8>(opcode).truncate();
    let offset = (signed_bits16::<7, 0>(opcode) << 1).into();
    ThumbInstruction::ConditionalBranch { conditions, offset }
}

#[derive(Clone, Copy, Debug)]
pub struct MultipleLoadstoreOperands {
    pub load: bool,
    pub base_register: u16,
    pub register_list: u8,
}

fn multiple_loadstore(opcode: u16) -> ThumbInstruction {
    let load = bit16::<11>(opcode);
    let base_register = bits16::<10, 8>(opcode);
    let register_list = bits16::<7, 0>(opcode).truncate();
    ThumbInstruction::MultipleLoadstore(MultipleLoadstoreOperands {
        load,
        base_register,
        register_list,
    })
}

fn long_branch_with_link(opcode: u16) -> ThumbInstruction {
    let h = bit16::<11>(opcode);
    let offset = (bits16::<10, 0>(opcode) << 1).into();
    ThumbInstruction::LongBranchWithLink { h, offset }
}

fn add_offset_to_stack_pointer(opcode: u16) -> ThumbInstruction {
    let subtract = bit16::<7>(opcode);
    let offset = (signed_bits16::<6, 0>(opcode) << 2).into();
    ThumbInstruction::AddOffsetToStackPointer { subtract, offset }
}

#[derive(Clone, Copy, Debug)]
pub struct PushPopRegistersOperands {
    pub load: bool,
    pub pc_lr: bool,
    pub register_list: u8,
}

fn push_pop_registers(opcode: u16) -> ThumbInstruction {
    let load = bit16::<11>(opcode);
    let pc_lr = bit16::<8>(opcode);
    let register_list = bits16::<7, 0>(opcode).truncate();
    ThumbInstruction::PushPopRegisters(PushPopRegistersOperands {
        load,
        pc_lr,
        register_list,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct LoadStoreHalfwordOperands {
    pub load: bool,
    pub offset: u8,
    pub base_register: u16,
    pub source_destination_register: u16,
}

fn load_store_halfword(opcode: u16) -> ThumbInstruction {
    let load = bit16::<11>(opcode);
    let offset = (bits16::<10, 6>(opcode) << 1).truncate();
    let base_register = bits16::<5, 3>(opcode);
    let source_destination_register = bits16::<2, 0>(opcode);
    ThumbInstruction::LoadStoreHalfword(LoadStoreHalfwordOperands {
        load,
        offset,
        base_register,
        source_destination_register,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct SpRelativeLoadStoreOperands {
    pub load: bool,
    pub destination_register: u16,
    pub offset: u8,
}

fn sp_relative_load_store(opcode: u16) -> ThumbInstruction {
    let load = bit16::<11>(opcode);
    let destination_register = bits16::<10, 8>(opcode);
    let offset = bits16::<7, 0>(opcode).truncate();
    ThumbInstruction::SpRelativeLoadStore(SpRelativeLoadStoreOperands {
        load,
        destination_register,
        offset,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct LoadAddressOperands {
    pub source: bool,
    pub destination_register: u16,
    pub word: u8,
}

fn load_address(opcode: u16) -> ThumbInstruction {
    let source = bit16::<11>(opcode);
    let destination_register = bits16::<10, 8>(opcode);
    let word = (bits16::<7, 0>(opcode) << 2).truncate();
    ThumbInstruction::LoadAddress(LoadAddressOperands {
        source,
        destination_register,
        word,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum Offset {
    Register(u16),
    Immediate(u32),
}

#[derive(Clone, Copy, Debug)]
pub struct LoadStoreWithOffsetOperands {
    pub load: bool,
    pub byte: bool,
    pub offset: Offset,
    pub base_register: u16,
    pub source_destination_register: u16,
}

fn load_store_with_immediate_offset(opcode: u16) -> ThumbInstruction {
    let byte = bit16::<12>(opcode);
    let load = bit16::<11>(opcode);
    let offset = bits16::<10, 6>(opcode).into();
    let base_register = bits16::<5, 3>(opcode);
    let source_destination_register = bits16::<2, 0>(opcode);
    ThumbInstruction::LoadStoreWithOffset(LoadStoreWithOffsetOperands {
        load,
        byte,
        offset: Offset::Immediate(offset),
        base_register,
        source_destination_register,
    })
}

fn load_store_with_register_offset(opcode: u16) -> ThumbInstruction {
    let load = bit16::<11>(opcode);
    let byte = bit16::<10>(opcode);
    let offset_register = bits16::<8, 6>(opcode);
    let base_register = bits16::<5, 3>(opcode);
    let source_destination_register = bits16::<2, 0>(opcode);
    ThumbInstruction::LoadStoreWithOffset(LoadStoreWithOffsetOperands {
        load,
        byte,
        offset: Offset::Register(offset_register),
        base_register,
        source_destination_register,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct LoadStoreSignExtendedByteHalfwordOperands {
    pub halfword: bool,
    pub signed: bool,
    pub offset_register: u16,
    pub base_register: u16,
    pub source_destination_register: u16,
}

const fn load_store_sign_extended_byte_halfword(opcode: u16) -> ThumbInstruction {
    let halfword = bit16::<11>(opcode);
    let signed = bit16::<10>(opcode);
    let offset_register = bits16::<8, 6>(opcode);
    let base_register = bits16::<5, 3>(opcode);
    let source_destination_register = bits16::<2, 0>(opcode);
    ThumbInstruction::LoadStoreSignExtendedByteHalfword(LoadStoreSignExtendedByteHalfwordOperands {
        halfword,
        signed,
        offset_register,
        base_register,
        source_destination_register,
    })
}

fn pc_relative_load(opcode: u16) -> ThumbInstruction {
    let destination_register = bits16::<10, 8>(opcode);
    let offset = (bits16::<7, 0>(opcode) << 2).truncate();
    ThumbInstruction::PCRelativeLoad {
        destination_register,
        offset,
    }
}

fn hi_register_operations_branch_exchange(opcode: u16) -> ThumbInstruction {
    let sub_opcode = bits16::<9, 8>(opcode).truncate();
    let hi1 = bit16::<7>(opcode);
    let hi2 = bit16::<6>(opcode);
    let mut source_register = bits16::<5, 3>(opcode);
    let mut destination_register = bits16::<2, 0>(opcode);
    if hi1 {
        destination_register = destination_register.wrapping_add(8);
    }
    if hi2 {
        source_register = source_register.wrapping_add(8);
    }
    ThumbInstruction::HiRegisterOperationsBranchExchange(ThumbOperands {
        sub_opcode,
        source_register,
        destination_register,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct ThumbOperands {
    pub sub_opcode: u8,
    pub source_register: u16,
    pub destination_register: u16,
}

fn alu_operations(opcode: u16) -> ThumbInstruction {
    let sub_opcode = bits16::<9, 6>(opcode).truncate();
    let source_register = bits16::<5, 3>(opcode);
    let destination_register = bits16::<2, 0>(opcode);
    ThumbInstruction::AluOperations(ThumbOperands {
        sub_opcode,
        source_register,
        destination_register,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveCompareAddSubtractOperation {
    Move,
    Compare,
    Add,
    Subtract,
}

#[derive(Clone, Copy, Debug)]
pub struct MoveCompareAddSubtractImmediateOperands {
    pub operation: MoveCompareAddSubtractOperation,
    pub destination_register: u16,
    pub immediate_value: u32,
}

fn move_compare_add_subtract_immediate(opcode: u16) -> ThumbInstruction {
    let operation = match (bit16::<12>(opcode), bit16::<11>(opcode)) {
        (false, false) => MoveCompareAddSubtractOperation::Move,
        (false, true) => MoveCompareAddSubtractOperation::Compare,
        (true, false) => MoveCompareAddSubtractOperation::Add,
        (true, true) => MoveCompareAddSubtractOperation::Subtract,
    };
    let destination_register = bits16::<10, 8>(opcode);
    let immediate_value = bits16::<7, 0>(opcode).into();
    ThumbInstruction::MoveCompareAddSubtractImmediate(MoveCompareAddSubtractImmediateOperands {
        operation,
        destination_register,
        immediate_value,
    })
}

#[derive(Clone, Copy, Debug)]
pub enum AddSubtractOperation {
    AddRegister,
    AddImmediate,
    SubtractRegister,
    SubtractImmediate,
}

#[derive(Clone, Copy, Debug)]
pub struct AddSubtractOperands {
    pub operation: AddSubtractOperation,
    pub register_3: u16,
    pub immediate_value: u32,
    pub source_register: u16,
    pub destination_register: u16,
}

fn add_subtract(opcode: u16) -> ThumbInstruction {
    let immediate = bit16::<10>(opcode);
    let sub_opcode = bit16::<9>(opcode);
    let register_3 = bits16::<8, 6>(opcode);
    let immediate_value = bits16::<8, 6>(opcode).into();
    let source_register = bits16::<5, 3>(opcode);
    let destination_register = bits16::<2, 0>(opcode);
    let operation = match (sub_opcode, immediate) {
        (false, false) => AddSubtractOperation::AddRegister,
        (false, true) => AddSubtractOperation::AddImmediate,
        (true, false) => AddSubtractOperation::SubtractRegister,
        (true, true) => AddSubtractOperation::SubtractImmediate,
    };
    ThumbInstruction::AddSubtract(AddSubtractOperands {
        operation,
        register_3,
        immediate_value,
        source_register,
        destination_register,
    })
}

#[derive(Clone, Copy, Debug)]
pub struct MoveShiftedRegisterOperands {
    pub opcode: u8,
    pub immediate_value: u32,
    pub source_register: u16,
    pub destination_register: u16,
}

fn move_shifted_register(opcode: u16) -> ThumbInstruction {
    let sub_opcode = bits16::<12, 11>(opcode).truncate();
    let immediate_value = bits16::<10, 6>(opcode).into();
    let source_register = bits16::<5, 3>(opcode);
    let destination_register = bits16::<2, 0>(opcode);
    // Not sure about this, but ai says to do it. Need to find the correct documentation.
    //if immediate_value == 0 {
    //immediate_value = 32;
    //}
    ThumbInstruction::MoveShiftedRegister(MoveShiftedRegisterOperands {
        opcode: sub_opcode,
        immediate_value,
        source_register,
        destination_register,
    })
}

const fn bits32<const START: usize, const END: usize>(opcode: u32) -> u32 {
    const {
        assert!(START < 32, "Start bit must be less than 32");
        assert!(END < START, "End bit must be less than start bit");
    }
    let left = 31 - START;
    (opcode << left) >> (left + END)
}

const fn signed_bits32<const START: usize, const END: usize>(opcode: u32) -> i32 {
    const {
        assert!(START < 32, "Start bit must be less than 32");
        assert!(END < START, "End bit must be less than start bit");
    }
    let left = 31 - START;
    (opcode.cast_signed() << left) >> (left + END)
}

const fn bit32<const BIT: usize>(opcode: u32) -> bool {
    const {
        assert!(BIT < 32, "Bit must be less than 32");
    }
    (opcode >> BIT) & 1 > 0
}

const fn bits16<const START: usize, const END: usize>(opcode: u16) -> u16 {
    const {
        assert!(START < 16, "Start bit must be less than 16");
        assert!(END < START, "End bit must be less than start bit");
    }
    let left = 15 - START;
    (opcode << left) >> (left + END)
}

const fn signed_bits16<const START: usize, const END: usize>(opcode: u16) -> i16 {
    const {
        assert!(START < 16, "Start bit must be less than 16");
        assert!(END < START, "End bit must be less than start bit");
    }
    let left = 15 - START;
    (opcode.cast_signed() << left) >> (left + END)
}

const fn bit16<const BIT: usize>(opcode: u16) -> bool {
    const {
        assert!(BIT < 16, "Bit must be less than 16");
    }
    (opcode >> BIT) & 1 > 0
}
