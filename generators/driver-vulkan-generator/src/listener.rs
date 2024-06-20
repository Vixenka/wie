use std::{fs, path::Path};

use itertools::Itertools;
use vk_parse::CommandDefinition;

use crate::{
    push_indentation, push_param_name, to_rust_type, to_rust_type_without_ptr, to_snake_case,
    trace,
    transport::{self, check_if_count_ptr},
    VULKAN_HANDLERS_BEGIN,
};

pub fn generate(project_directory: &Path, commands: &[&CommandDefinition]) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse std::ffi::c_void;\nuse crate::Packet;\n");

    generate_function_handler_map(&mut builder, commands);

    for definition in commands {
        generate_command(&mut builder, definition);
    }

    let path = project_directory.join("crates/driver-listener-vulkan/src/generated/handlers.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

fn generate_function_handler_map(builder: &mut String, commands: &[&CommandDefinition]) {
    builder.push_str("\npub(crate) fn register_handlers_to(map: &mut crate::HandlerMap) {\n");

    let mut i = VULKAN_HANDLERS_BEGIN;
    for definition in commands {
        push_indentation(builder, 1);
        builder.push_str("map.insert(");
        builder.push_str(&i.to_string());
        builder.push_str(", Box::new(");
        to_snake_case(builder, &definition.proto.name);
        builder.push_str("));\n");

        i += 1;
    }

    builder.push_str("}\n");
}

fn generate_command(builder: &mut String, definition: &CommandDefinition) {
    builder.push_str(
        "\n#[doc = \"<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/",
    );
    builder.push_str(&definition.proto.name);
    builder.push_str(".html>\"]\nfn ");
    to_snake_case(builder, &definition.proto.name);
    builder.push_str("(mut packet: Packet) {\n");

    let mut last_is_count = false;
    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        let is_count = check_if_count_ptr(param);

        if last_is_count {
            push_param_name(builder, param);
            builder.push_str(") = packet.read_and_allocate_vk_array_count::<");
            builder.push_str(&to_rust_type_without_ptr(&param.definition));
            builder.push_str(">();\n");
        } else {
            push_indentation(builder, 1);
            builder.push_str("let ");

            if is_count {
                builder.push('(');
                push_param_name(builder, param);
                builder.push_str(", ");
            } else {
                push_param_name(builder, param);
                builder.push_str(": ");
                builder.push_str(&to_rust_type(&param.definition));
                builder.push_str(" = packet.read");
                transport::read_packet_param(builder, param, last_is_count, is_count);
            }
        }

        last_is_count = is_count;
    }

    trace(builder, definition);

    builder.push_str("}\n");
}
