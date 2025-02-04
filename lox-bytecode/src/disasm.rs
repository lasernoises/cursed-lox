use crate::bytecode::Module;
use crate::opcode::Opcode;

pub fn disassemble_module(module: &Module) {
    println!("=== Start of Dump ===");
    println!();

    for (index, chunk) in module.chunks().iter().enumerate() {
        println!("=== Chunk {} ===", index);
        disassemble_chunk(chunk.as_slice(), module);
        println!();
    }

    println!("=== Classes ===");
    for (index, class) in module.classes().iter().enumerate() {
        println!("{} {}", index, class.name);
    }
    println!();

    println!("=== Closures ===");
    for (index, closure) in module.closures().iter().enumerate() {
        println!("{} {:?}", index, closure);
    }
    println!();

    println!("=== Identifiers ===");
    for (index, identifier) in module.identifiers().iter().enumerate() {
        println!("{} {}", index, identifier);
    }
    println!();

    println!("=== Numbers ===");
    for (index, constant) in module.numbers.iter().enumerate() {
        println!("{} {:?}", index, constant);
    }
    println!();

    println!("=== Strings ===");
    for (index, constant) in module.strings.iter().enumerate() {
        println!("{} {:?}", index, constant);
    }
    println!();

    println!("=== End of Dump ===");
    println!();
}

pub fn disassemble_chunk(chunk: &[u8], module: &Module) {
    let chunk = crate::opcode::OpcodeIterator::new(chunk.iter().cloned());
    for (offset, opcode) in chunk {
        let instruction = format!("{:?}", opcode);
        match opcode {
            Opcode::Jump(relative) => println!(
                "{:04X} {:<18} {:04X}",
                offset,
                instruction,
                absolute(offset, relative)
            ),
            Opcode::JumpIfFalse(relative) => println!(
                "{:04X} {:<18} {:04X}",
                offset,
                instruction,
                absolute(offset, relative)
            ),
            Opcode::DefineGlobal(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            Opcode::GetGlobal(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            Opcode::SetGlobal(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            Opcode::Number(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.number(index as _)
            ),
            Opcode::String(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.string(index as _)
            ),
            Opcode::Invoke(_arity, index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            Opcode::GetProperty(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            Opcode::SetProperty(index) => println!(
                "{:04X} {:<18} {}",
                offset,
                instruction,
                module.identifier(index as _)
            ),
            _ => println!("{:04X} {:<18}", offset, instruction),
        }
    }
}

fn absolute(offset: usize, relative: i16) -> usize {
    let offset = offset as i64;
    let relative = relative as i64;
    let absolute = offset + relative + 3;
    absolute as usize
}
