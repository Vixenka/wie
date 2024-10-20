use std::{collections::HashSet, fs, path::Path};

use vk_parse::CommandDefinition;

use crate::{
    driver::generate_function_header, function_data::CommandExt, push_indentation, to_snake_case,
    vulkan_types::TypeVulkan,
};

pub fn generate(
    project_directory: &Path,
    commands: &[&CommandDefinition],
    required_commands: &HashSet<&str>,
    types: &TypeVulkan,
) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse wie_driver_common_vulkan::{*, generated::vulkan_types::*, generated::vulkan_bitmasks::*, generated::vulkan_pfn_functions::*};\nuse std::ffi::{c_char, c_void};\n");

    generate_struct(&mut builder, commands);
    generate_impl(&mut builder, commands, required_commands, types);

    let path = project_directory
        .join("crates/driver-listener-vulkan/src/generated/function_address_table.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

fn generate_struct(builder: &mut String, commands: &[&CommandDefinition]) {
    builder.push_str("\npub struct FunctionAddressTable {\n");

    for command in commands {
        push_indentation(builder, 1);
        builder.push_str("pub ");
        to_snake_case(builder, &command.proto.name);
        builder.push_str(": PFN_");
        builder.push_str(&command.proto.name);
        builder.push_str(",\n");
    }

    builder.push_str("}\n");
}

fn generate_impl(
    builder: &mut String,
    commands: &[&CommandDefinition],
    required_commands: &HashSet<&str>,
    types: &TypeVulkan,
) {
    builder.push_str("\nimpl FunctionAddressTable {\n");
    generate_new(builder, commands, types);
    generate_set_address(builder, commands, required_commands);
    builder.push_str("}\n");

    builder.push_str("\nunsafe impl Sync for FunctionAddressTable {}\n");
    builder.push_str("\nunsafe impl Send for FunctionAddressTable {}\n");
}

fn generate_new(builder: &mut String, commands: &[&CommandDefinition], types: &TypeVulkan) {
    push_indentation(builder, 1);
    builder.push_str("pub const fn new() -> Self {\n");

    push_indentation(builder, 2);
    builder.push_str("Self {\n");

    for command in commands {
        push_indentation(builder, 3);
        to_snake_case(builder, &command.proto.name);
        builder.push_str(": {");

        generate_function_header(builder, command, 4, types);
        push_indentation(builder, 5);
        builder
            .push_str("panic!(\"attempted to invoke not initialized function `{}`\", stringify!(");
        builder.push_str(&command.proto.name);
        builder.push_str("))\n");
        push_indentation(builder, 4);
        builder.push_str("}\n");

        push_indentation(builder, 4);
        builder.push_str(&command.proto.name);
        builder.push('\n');
        push_indentation(builder, 3);
        builder.push_str("},\n");
    }

    push_indentation(builder, 2);
    builder.push_str("}\n");

    push_indentation(builder, 1);
    builder.push_str("}\n");
}

fn generate_set_address(
    builder: &mut String,
    commands: &[&CommandDefinition],
    required_commands: &HashSet<&str>,
) {
    fn push_address_match(builder: &mut String, command: &CommandDefinition, name: &str) {
        push_indentation(builder, 3);
        builder.push('"');
        builder.push_str(name);
        builder.push_str("\" => self.");
        to_snake_case(builder, &command.proto.name);
        builder.push_str(" = unsafe { std::mem::transmute(address) },\n");
    }

    builder.push('\n');
    push_indentation(builder, 1);
    builder
        .push_str("pub fn set_address(&mut self, name: &str, address: vk::PFN_vkVoidFunction) {\n");
    push_indentation(builder, 2);
    builder.push_str("match name {\n");

    for command in commands {
        push_address_match(builder, command, &command.proto.name);
        if let Some(alias) = command.get_alias(required_commands) {
            push_address_match(builder, command, &alias);
        }
    }

    push_indentation(builder, 3);
    builder.push_str("_ => panic!(\"unable to set address for function with name `{}`, field do not exists\", name)\n");
    push_indentation(builder, 2);
    builder.push_str("}\n");
    push_indentation(builder, 1);
    builder.push_str("}\n");
}
