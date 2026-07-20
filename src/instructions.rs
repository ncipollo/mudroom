pub mod abilities;
pub mod attributes;
pub mod classes;
pub mod entities;
pub mod maps;
pub mod mud_config;
pub mod top_level;

pub enum InstructionType {
    TopLevel,
    MudConfig,
    Abilities,
    Attributes,
    Classes,
    Entities,
    Maps,
}

pub fn print_instructions(topic: InstructionType) {
    let text = match topic {
        InstructionType::TopLevel => top_level::render(),
        InstructionType::MudConfig => mud_config::render(),
        InstructionType::Abilities => abilities::render(),
        InstructionType::Attributes => attributes::render(),
        InstructionType::Classes => classes::render(),
        InstructionType::Entities => entities::render(),
        InstructionType::Maps => maps::render(),
    };
    println!("{text}");
}
