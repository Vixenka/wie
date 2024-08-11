use std::{fs, path::Path};

use itertools::Itertools;
use vk_parse::CommandDefinition;

use crate::{push_indentation, push_param_name, to_rust_type, vulkan_types::TypeVulkan};

pub fn generate_function_definition(
    builder: &mut String,
    definition: &CommandDefinition,
    start_indentation: usize,
    types: &TypeVulkan,
) {
    builder.push('\n');
    push_indentation(builder, start_indentation);

    builder.push_str("pub type PFN_");
    builder.push_str(&definition.proto.name);
    builder.push_str(" = unsafe extern \"system\" fn(");

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
    builder.push_str(") -> ");
    let return_type = to_rust_type(&definition.proto, types);
    builder.push_str(&return_type);

    builder.push_str(";\n");
}

pub fn generate(project_directory: &Path, commands: &[&CommandDefinition], types: &TypeVulkan) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse ash::vk;\nuse crate::{*, generated::vulkan_types::*};\nuse std::ffi::{c_char, c_void};\n");
    builder.push_str(
        "\npub use ash::vk::{
    PFN_vkAllocationFunction, PFN_vkDebugReportCallbackEXT, PFN_vkDebugUtilsMessengerCallbackEXT,
    PFN_vkDeviceMemoryReportCallbackEXT, PFN_vkFreeFunction, PFN_vkGetInstanceProcAddrLUNARG,
    PFN_vkInternalAllocationNotification, PFN_vkInternalFreeNotification,
    PFN_vkReallocationFunction, PFN_vkVoidFunction,
};\n",
    );

    for definition in commands {
        generate_command(&mut builder, definition, types);
    }

    let path =
        project_directory.join("crates/driver-common-vulkan/src/generated/vulkan_pfn_functions.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

fn generate_command(builder: &mut String, definition: &CommandDefinition, types: &TypeVulkan) {
    builder.push_str(
        "\n#[allow(non_camel_case_types)]\n#[doc = \"<https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/",
    );
    builder.push_str(&definition.proto.name);
    builder.push_str(".html>\"]");

    generate_function_definition(builder, definition, 0, types);
}
