use std::num::TryFromIntError;

use thiserror::Error;

#[derive(Clone, Error, Debug)]
pub enum DecodeError {
    #[error("Error casting integer")]
    IntegerCastError(#[from] TryFromIntError),
    #[error("Attempting to decode u32 with start > 31")]
    StartBitOutOfRange,
    #[error("Attempting to decode u32 with end >= start")]
    EndBitOutOfRange,
    #[error("Attempting to decode bool from u32 bit > 31")]
    BoolBitOutOfRange,
    #[error("Could not decode {0}")]
    UndefinedArmInstruction(u32),
}

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

pub fn arm_instruction(opcode: u32) -> Result<ArmInstruction, DecodeError> {
    ARM_DECODE_TABLE
        .iter()
        .find(|decoder| opcode & decoder.mask == decoder.value)
        .map_or(
            Err(DecodeError::UndefinedArmInstruction(opcode)),
            |decoder| Ok((decoder.instruction)(opcode)),
        )
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

fn branch(opcode: u32) -> ArmInstruction {
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
    const IMMEDIATE_VALUE_BIT: u32 = 1 << 25;
    const SOURCE_PSR_BIT: u32 = 1 << 22;
    const MODE_INCLUDED_BIT: u32 = 1 << 16;
    const ROTATE_MASK: u32 = 0x0000_0F00;
    const ROTATE_SHIFT: usize = 8;
    const IMMEDIATE_VALUE_MASK: u32 = 0x0000_00FF;
    const REGISTER_MASK: u32 = 0x0000_000F;
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
    const IMMEDIATE_VALUE_BIT: u32 = 1 << 25;
    const OPCODE_MASK: u32 = 0x01E0_0000;
    const OPCODE_SHIFT: usize = 21;
    const SET_CONDITIONS_BIT: u32 = 1 << 20;
    const FIRST_OPERAND_REGISTER_MASK: u32 = 0x000F_0000;
    const FIRST_OPERAND_REGISTER_SHIFT: usize = 16;
    const DESTINATION_REGISTER_MASK: u32 = 0x0000_F000;
    const DESTINATION_REGISTER_SHIFT: usize = 12;
    const SECOND_OPERAND_MASK: u32 = 0x0000_0FFF;
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
    AluOperations,
    MoveCompareAddSubtractImmediate,
    AddSubtract,
    MoveShiftedRegister,
    Unimplemented,
}

#[derive(Debug, Clone, Copy)]
struct ThumbDecoder {
    mask: u16,
    value: u16,
    instruction: ThumbInstruction,
}

const THUMB_DECODE_TABLE: [ThumbDecoder; 19] = [
    ThumbDecoder {
        mask: 0xFF00,
        value: 0xDF00,
        instruction: ThumbInstruction::SoftwareInterrupt,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0xE000,
        instruction: ThumbInstruction::UnconditionalBranch,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xD000,
        instruction: ThumbInstruction::ConditionalBranch,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xC000,
        instruction: ThumbInstruction::MultipleLoadstore,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xF000,
        instruction: ThumbInstruction::LongBranchWithLink,
    },
    ThumbDecoder {
        mask: 0xFF00,
        value: 0xB000,
        instruction: ThumbInstruction::AddOffsetToStackPointer,
    },
    ThumbDecoder {
        mask: 0xF600,
        value: 0xB400,
        instruction: ThumbInstruction::PushPopRegisters,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0x8000,
        instruction: ThumbInstruction::LoadStoreHalfword,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0x9000,
        instruction: ThumbInstruction::SPRelativeLoadStore,
    },
    ThumbDecoder {
        mask: 0xF000,
        value: 0xA000,
        instruction: ThumbInstruction::LoadAddress,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x6000,
        instruction: ThumbInstruction::LoadStoreWithImmediateOffset,
    },
    ThumbDecoder {
        mask: 0xF200,
        value: 0x5000,
        instruction: ThumbInstruction::LoadStoreWithRegisterOffset,
    },
    ThumbDecoder {
        mask: 0xF200,
        value: 0x5200,
        instruction: ThumbInstruction::LoadStoreSignExtendedByteHalfword,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0x4800,
        instruction: ThumbInstruction::PCRelativeLoad,
    },
    ThumbDecoder {
        mask: 0xFC00,
        value: 0x4400,
        instruction: ThumbInstruction::HiRegisterOperationsBranchExchange,
    },
    ThumbDecoder {
        mask: 0xFC00,
        value: 0x4000,
        instruction: ThumbInstruction::AluOperations,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x2000,
        instruction: ThumbInstruction::MoveCompareAddSubtractImmediate,
    },
    ThumbDecoder {
        mask: 0xF800,
        value: 0x1800,
        instruction: ThumbInstruction::AddSubtract,
    },
    ThumbDecoder {
        mask: 0xE000,
        value: 0x0000,
        instruction: ThumbInstruction::MoveShiftedRegister,
    },
];

pub fn thumb_instruction(opcode: u16) -> ThumbInstruction {
    THUMB_DECODE_TABLE
        .iter()
        .find(|decoder| opcode & decoder.mask == decoder.value)
        .map_or(ThumbInstruction::Unimplemented, |decoder| {
            decoder.instruction
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
