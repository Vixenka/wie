use std::{fs, path::Path};

use vk_parse::CommandDefinition;

use crate::{
    function_data::{CommandExt, FunctionType},
    push_indentation, to_snake_case,
};

pub fn generate(project_directory: &Path, commands: &[&CommandDefinition]) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse std::cell::UnsafeCell;\n");

    generate_impl(&mut builder, commands);

    let path = project_directory
        .join("crates/driver-listener-vulkan/src/generated/function_address_table.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

fn generate_impl(builder: &mut String, commands: &[&CommandDefinition]) {
    builder.push_str("\npub struct FunctionAddressTable {\n");

    for command in commands {
        if command.function_type() == FunctionType::Entry {
            continue;
        }

        push_indentation(builder, 1);
        builder.push_str("pub ");
        to_snake_case(builder, &command.proto.name);
        builder.push_str(": UnsafeCell<vk::PFN_");
        builder.push_str(&command.proto.name);
        builder.push_str(">,\n");
    }

    builder.push_str("}\n");
}
