use crate::instructions::{self, InstructionType};

use super::InstructionsTopic;

pub fn run(topic: Option<InstructionsTopic>) {
    let instruction_type = match topic {
        None => InstructionType::TopLevel,
        Some(InstructionsTopic::MudConfig) => InstructionType::MudConfig,
        Some(InstructionsTopic::Abilities) => InstructionType::Abilities,
        Some(InstructionsTopic::Attributes) => InstructionType::Attributes,
        Some(InstructionsTopic::Classes) => InstructionType::Classes,
        Some(InstructionsTopic::Entities) => InstructionType::Entities,
        Some(InstructionsTopic::Inventory) => InstructionType::Inventory,
        Some(InstructionsTopic::Items) => InstructionType::Items,
        Some(InstructionsTopic::Maps) => InstructionType::Maps,
    };
    instructions::print_instructions(instruction_type);
}
