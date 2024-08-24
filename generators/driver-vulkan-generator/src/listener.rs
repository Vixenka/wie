use std::{fs, path::Path};

use itertools::Itertools;
use vk_parse::CommandDefinition;

use crate::{
    function_data::{CommandExt, CommandParamExt},
    push_indentation, push_param_name, to_rust_type, to_rust_type_without_ptr, to_snake_case,
    trace,
    transport::{self, check_if_count_ptr},
    vulkan_types::TypeVulkan,
    VULKAN_HANDLERS_BEGIN,
};

pub fn generate(project_directory: &Path, commands: &[&CommandDefinition], types: &TypeVulkan) {
    let overrided_commands = OverridedCommands::new(project_directory);

    let mut builder = String::new();
    builder.push_str(
        "//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse wie_driver_common_vulkan::{*, generated::vulkan_types::*, generated::vulkan_bitmasks::*};\nuse crate::Packet;\nuse std::ffi::{c_char, c_void};\n",
    );

    generate_function_handler_map(&mut builder, commands);

    for definition in commands {
        generate_command(&mut builder, definition, types, &overrided_commands);
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

fn generate_command(
    builder: &mut String,
    definition: &CommandDefinition,
    types: &TypeVulkan,
    overrided_commands: &OverridedCommands,
) {
    builder.push_str(
        "\n#[doc = \"<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/",
    );
    builder.push_str(&definition.proto.name);
    builder.push_str(".html>\"]\nfn ");
    to_snake_case(builder, &definition.proto.name);
    builder.push_str("(mut packet: Packet) {\n");

    unpack_packet(builder, definition, types);
    trace(builder, definition, true);

    let return_type = to_rust_type(&definition.proto, types);
    let is_void = return_type == "std::ffi::c_void";

    call_vulkan_function(builder, definition, is_void, overrided_commands);
    if definition.is_return_data(types) {
        write_response(builder, definition, is_void, types);
    }

    builder.push_str("}\n");
}

fn unpack_packet(builder: &mut String, definition: &CommandDefinition, types: &TypeVulkan) {
    let mut last_is_count = false;
    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        let is_count = check_if_count_ptr(param);

        if last_is_count {
            push_param_name(builder, param);
            builder.push_str(") = unsafe { packet.read_vk_array_ref_mut::<");
            builder.push_str(&to_rust_type_without_ptr(
                &param.definition.type_name,
                types,
            ));
            builder.push_str(">() };\n");
        } else {
            push_indentation(builder, 1);
            builder.push_str("let ");

            if is_count {
                builder.push_str("(mut ");
                push_param_name(builder, param);
                builder.push_str(", ");
            } else {
                push_param_name(builder, param);
                builder.push_str(": ");
                builder.push_str(&to_rust_type(&param.definition, types));
                builder.push_str(" = packet");
                transport::read_packet_param(builder, param, false, types);
            }
        }

        last_is_count = is_count;
    }
}

fn call_vulkan_function(
    builder: &mut String,
    definition: &CommandDefinition,
    is_void: bool,
    overrided_commands: &OverridedCommands,
) {
    push_indentation(builder, 1);
    if !is_void {
        builder.push_str("let result = ");
    }
    builder.push_str("unsafe {\n");

    push_indentation(builder, 2);
    if overrided_commands.is_overrided(&definition.proto.name) {
        call_vulkan_overrided_function(builder, definition);
    } else {
        builder.push_str("(crate::FUNCTION_ADDRESS_TABLE.");
        to_snake_case(builder, &definition.proto.name);
        builder.push_str(")(\n");
    }

    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        push_indentation(builder, 3);

        if check_if_count_ptr(param) {
            builder.push_str("&mut ");
        }

        push_param_name(builder, param);
        builder.push_str(",\n");
    }
    push_indentation(builder, 2);
    builder.push_str(")\n");

    push_indentation(builder, 1);
    builder.push_str("};\n");
}

fn call_vulkan_overrided_function(builder: &mut String, definition: &CommandDefinition) {
    builder.push_str("crate::overrided_commands::");
    to_snake_case(builder, &definition.proto.name);
    builder.push_str("(\n");
}

fn write_response(
    builder: &mut String,
    definition: &CommandDefinition,
    is_void: bool,
    types: &TypeVulkan,
) {
    builder.push('\n');
    push_indentation(builder, 1);
    builder.push_str("let mut response = packet.write_response(None);\n");

    let params: Vec<_> = definition
        .params
        .iter()
        .unique_by(|x| &x.definition.name)
        .filter(|x| x.is_return_data(types))
        .collect();

    if !params.is_empty() {
        push_indentation(builder, 1);
        builder.push_str("unsafe {\n");
    }

    let mut last_is_count = false;
    for param in &params {
        let is_count = check_if_count_ptr(param);

        if last_is_count {
            push_param_name(builder, param);
            builder.push_str(");\n");
        } else {
            push_indentation(builder, 2);
            builder.push_str("response");

            if is_count {
                builder.push_str(".write_vk_array(");
                push_param_name(builder, param);
                builder.push_str(", ");
                last_is_count = true;
                continue;
            } else {
                transport::write_packet_param(builder, param, true, types);
            }
        }

        last_is_count = is_count;
    }

    if !params.is_empty() {
        push_indentation(builder, 1);
        builder.push_str("}\n");
    }

    if !is_void {
        push_indentation(builder, 1);
        builder.push_str("response.write_shallow(result);\n");
    }

    push_indentation(builder, 1);
    builder.push_str("response.send();\n");
}

struct OverridedCommands {
    mod_file: String,
}

impl OverridedCommands {
    fn new(project_directory: &Path) -> Self {
        Self {
            mod_file: fs::read_to_string(
                project_directory
                    .join("crates/driver-listener-vulkan/src/overrided_commands/mod.rs"),
            )
            .expect("read file"),
        }
    }

    fn is_overrided(&self, name: &str) -> bool {
        let mut snake_case = String::new();
        to_snake_case(&mut snake_case, name);
        self.mod_file.contains(&snake_case)
    }
}
