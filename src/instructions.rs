pub mod top_level;

pub enum InstructionType {
    TopLevel,
}

pub fn print_instructions(topic: InstructionType) {
    let text = match topic {
        InstructionType::TopLevel => top_level::render(),
    };
    println!("{text}");
}
