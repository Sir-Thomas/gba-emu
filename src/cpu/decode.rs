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
struct ArmDecoder {
    mask: u32,
    value: u32,
    instruction: ArmInstruction,
}

const ARM_DECODE_TABLE: [ArmDecoder; 14] = [
    ArmDecoder {
        mask: 0x0FFF_FFF0,
        value: 0x012F_FF10,
        instruction: ArmInstruction::BranchAndExchange,
    },
    ArmDecoder {
        mask: 0x0E00_0000,
        value: 0x0800_0000,
        instruction: ArmInstruction::BlockDataTransfer,
    },
    ArmDecoder {
        mask: 0x0E00_0000,
        value: 0x0A00_0000,
        instruction: ArmInstruction::Branch,
    },
    ArmDecoder {
        mask: 0x0F00_0000,
        value: 0x0F00_0000,
        instruction: ArmInstruction::SoftwareInterrupt,
    },
    ArmDecoder {
        mask: 0x0E00_0010,
        value: 0x0600_0010,
        instruction: ArmInstruction::Undefined,
    },
    ArmDecoder {
        mask: 0x0C00_0000,
        value: 0x0400_0000,
        instruction: ArmInstruction::SingleDataTransfer,
    },
    ArmDecoder {
        mask: 0x0F80_0FF0,
        value: 0x0100_0090,
        instruction: ArmInstruction::SingleDataSwap,
    },
    ArmDecoder {
        mask: 0x0F80_00F0,
        value: 0x0000_0090,
        instruction: ArmInstruction::Multiply,
    },
    ArmDecoder {
        mask: 0x0F80_00F0,
        value: 0x0080_0090,
        instruction: ArmInstruction::MultiplyLong,
    },
    ArmDecoder {
        mask: 0x0E40_0F90,
        value: 0x0000_0090,
        instruction: ArmInstruction::HalfwordDataTransferRegisterOffset,
    },
    ArmDecoder {
        mask: 0x0E40_0090,
        value: 0x0040_0090,
        instruction: ArmInstruction::HalfwordDataTransferImmediateOffset,
    },
    ArmDecoder {
        mask: 0x0FBF_0000,
        value: 0x010F_0000,
        instruction: ArmInstruction::PsrTransferMRS,
    },
    ArmDecoder {
        mask: 0x0DB0_F000,
        value: 0x0120_F000,
        instruction: ArmInstruction::PsrTransferMSR,
    },
    ArmDecoder {
        mask: 0x0C00_0000,
        value: 0x0000_0000,
        instruction: ArmInstruction::DataProcessing,
    },
];

pub fn arm_instruction(opcode: u32) -> ArmInstruction {
    ARM_DECODE_TABLE
        .iter()
        .find(|decoder| opcode & decoder.mask == decoder.value)
        .map_or(ArmInstruction::Undefined, |decoder| decoder.instruction)
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
