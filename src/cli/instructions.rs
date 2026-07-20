use crate::instructions::{self, InstructionType};

use super::InstructionsTopic;

pub fn run(_topic: Option<InstructionsTopic>) {
    instructions::print_instructions(InstructionType::TopLevel);
}
