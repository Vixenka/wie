use std::{collections::HashSet, fs, path::Path};

use itertools::Itertools;
use vk_parse::CommandDefinition;

use crate::{
    function_data::{CommandExt, CommandParamExt},
    push_indentation, push_param_name, to_rust_type, trace,
    transport::{self, check_if_count_ptr},
    vulkan_types::TypeVulkan,
    VULKAN_HANDLERS_BEGIN,
};

pub fn generate_function_header(
    builder: &mut String,
    definition: &CommandDefinition,
    start_indentation: usize,
    types: &TypeVulkan,
) {
    builder.push('\n');
    push_indentation(builder, start_indentation);
    builder.push_str("unsafe extern \"system\" fn ");
    builder.push_str(&definition.proto.name);
    builder.push('(');

    if !definition.params.is_empty() {
        builder.push('\n');
        for param in definition.params.iter().unique_by(|x| &x.definition.name) {
            push_indentation(builder, start_indentation + 1);
            push_param_name(builder, param);
            builder.push_str(": ");
            builder.push_str(&to_rust_type(&param.definition, types));
            builder.push_str(",\n");
        }
    }

    push_indentation(builder, start_indentation);
    builder.push(')');
    let return_type = to_rust_type(&definition.proto, types);
    if return_type != "std::ffi::c_void" {
        builder.push_str(" -> ");
        builder.push_str(&return_type);
    }

    builder.push_str(" {\n");
}

pub fn generate(
    project_directory: &Path,
    commands: &[&CommandDefinition],
    required_commands: &HashSet<&str>,
    types: &TypeVulkan,
) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse wie_driver_common_vulkan::{generated::vulkan_types::*, generated::vulkan_bitmasks::*, generated::vulkan_pfn_functions::*, *};\nuse std::ffi::{c_char, c_void};\nuse wie_transport_guest::new_packet;\n");

    generate_function_name_map(&mut builder, commands, required_commands);

    let mut i = VULKAN_HANDLERS_BEGIN;
    for definition in commands {
        generate_command(&mut builder, definition, i, types);
        i += 1;
    }

    let path = project_directory.join("crates/driver-vulkan/src/generated/definitions.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

fn generate_function_name_map(
    builder: &mut String,
    commands: &[&CommandDefinition],
    required_commands: &HashSet<&str>,
) {
    fn push_address_match(builder: &mut String, definition: &CommandDefinition, name: &str) {
        push_indentation(builder, 2);
        builder.push('"');
        builder.push_str(name);
        builder.push_str("\" => ");
        builder.push_str(&definition.proto.name);
        builder.push_str(" as *const c_void,\n");
    }

    builder.push_str("\npub(crate) fn get_function_address(name: &str) -> *const c_void {\n");
    push_indentation(builder, 1);
    builder.push_str("match name {\n");

    for definition in commands {
        push_address_match(builder, definition, &definition.proto.name);
        if let Some(alias) = definition.get_alias(required_commands) {
            push_address_match(builder, definition, &alias);
        }
    }

    push_indentation(builder, 2);
    builder.push_str("_ => std::ptr::null(),\n");
    push_indentation(builder, 1);
    builder.push_str("}\n}\n\n");
}

fn generate_command(
    builder: &mut String,
    definition: &CommandDefinition,
    handler_id: u64,
    types: &TypeVulkan,
) {
    builder.push_str("\n#[no_mangle]\n#[doc = \"<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/");
    builder.push_str(&definition.proto.name);
    builder.push_str(".html>\"]");

    generate_function_header(builder, definition, 0, types);

    push_indentation(builder, 1);
    builder.push_str("trace!(\"called ");
    builder.push_str(&definition.proto.name);
    builder.push_str("\");\n");

    trace(builder, definition, false);

    // Packet creation
    push_indentation(builder, 1);
    builder.push_str("let mut packet = new_packet(");
    builder.push_str(&handler_id.to_string());
    builder.push_str(");\n");

    let mut last_is_count = false;
    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        let is_count = check_if_count_ptr(param);
        if is_count {
            push_indentation(builder, 1);
            builder.push_str("packet.write_vk_array(*");
            push_param_name(builder, param);
            builder.push_str(", ");
        } else if last_is_count {
            push_param_name(builder, param);
            builder.push_str(");\n");
        } else {
            push_indentation(builder, 1);
            builder.push_str("packet");
            transport::write_packet_param(builder, param, false, types);
        }

        last_is_count = is_count;
    }

    builder.push('\n');
    push_indentation(builder, 1);
    if definition.is_return_data(types) {
        builder.push_str("let mut response = packet.send_with_response();\n");
        unpack_response(builder, definition, types);
    } else {
        builder.push_str("packet.send();\n");
    }

    builder.push_str("}\n");
}

fn unpack_response(builder: &mut String, definition: &CommandDefinition, types: &TypeVulkan) {
    let mut last_is_count = false;
    for param in definition
        .params
        .iter()
        .unique_by(|x| &x.definition.name)
        .filter(|x| x.is_return_data(types))
    {
        let is_count = check_if_count_ptr(param);

        if last_is_count {
            push_param_name(builder, param);
            builder.push_str(");\n");
        } else {
            push_indentation(builder, 1);

            if is_count {
                builder.push_str("response.read_vk_array(");
                push_param_name(builder, param);
                builder.push_str(", ");
            } else {
                builder.push_str("response");
                transport::read_packet_param(builder, param, true, types);
            }
        }

        last_is_count = is_count;
    }

    push_indentation(builder, 1);
    builder.push_str("response.read_shallow()\n");
}
