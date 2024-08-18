use std::{collections::HashSet, path::Path};

use itertools::Itertools;
use vk_parse::{CommandDefinition, CommandParam, Enum, Extension, NameWithType, Registry};
use vulkan_types::{generate_vulkan_types, TypeVulkan};

const DESIRED_API: &str = "vulkan";
const INDENTATION: &str = "    ";

mod driver;
pub mod enums;
mod function_address_table;
pub mod function_data;
mod listener;
mod pfn_functions;
mod transport;
mod vulkan_bitmasks;
mod vulkan_types;

const VULKAN_HANDLERS_BEGIN: u64 = 1_000_001_000;

fn main() {
    let mut directory = std::env::current_dir().unwrap();
    while !directory.ends_with("wie") {
        directory = directory
            .parent()
            .expect("expected command to be called inside wie directory")
            .to_path_buf()
    }

    generate(
        &directory.join("generators/driver-vulkan-generator/Vulkan-Headers/registry/"),
        &directory,
    );
}

fn generate(vk_headers_path: &Path, project_directory: &Path) {
    let (spec, _errors) =
        vk_parse::parse_file(&vk_headers_path.join("vk.xml")).expect("invalid XML file");
    let (required_types, required_commands, _enums) =
        get_required_types_commands_and_extensions(&spec);

    let commands: Vec<&CommandDefinition> = spec
        .0
        .iter()
        .filter_map(|x| match x {
            vk_parse::RegistryChild::Commands(cmds) => Some(cmds),
            _ => None,
        })
        .flat_map(|x| &x.children)
        .filter_map(|x| match x {
            vk_parse::Command::Definition(def) => Some(def),
            _ => None,
        })
        .filter(|cmd| required_commands.contains(&cmd.proto.name.as_str()))
        .unique_by(|x| &x.proto.name)
        .collect();

    let (spec_video, _errors) =
        vk_parse::parse_file(&vk_headers_path.join("video.xml")).expect("invalid XML file");
    let (required_types_video, _, enums_video) =
        get_required_types_commands_and_extensions(&spec_video);

    let registry: vkxml::Registry =
        vk_parse::parse_file_as_vkxml(&vk_headers_path.join("vk.xml")).unwrap();

    let mut types = TypeVulkan::new(&spec, &registry, &required_types);
    let video_types = TypeVulkan::new(&spec_video, &registry, &required_types_video);
    types.chain(&video_types);

    println!("Generating Vulkan types...");
    generate_vulkan_types(project_directory, &types);
    println!("Generating Vulkan enums...");
    enums::generate_enums(project_directory, &enums_video);
    println!("Generating Vulkan bitmasks...");
    vulkan_bitmasks::generate(project_directory, &types);
    println!("Generating Vulkan PFN functions...");
    pfn_functions::generate(project_directory, &commands, &types);
    println!("Generating driver...");
    driver::generate(project_directory, &commands, &required_commands, &types);
    println!("Generating listener...");
    println!("Generating function address table...");
    function_address_table::generate(project_directory, &commands, &required_commands, &types);
    println!("Generating transport...");
    listener::generate(project_directory, &commands, &types);
}

fn get_required_types_commands_and_extensions(
    spec: &Registry,
) -> (HashSet<&str>, HashSet<&str>, Vec<&Enum>) {
    let extensions: Vec<&Extension> = spec
        .0
        .iter()
        .find_map(|x| match x {
            vk_parse::RegistryChild::Extensions(ext) => Some(ext),
            _ => None,
        })
        .expect("extension")
        .children
        .iter()
        .filter(|e| {
            if let Some(supported) = &e.supported {
                contains_desired_api(supported) ||
                // VK_ANDROID_native_buffer is for internal use only, but types defined elsewhere
                // reference enum extension constants.  Exempt the extension from this check until
                // types are properly folded in with their extension (where applicable).
                e.name == "VK_ANDROID_native_buffer"
            } else {
                true
            }
        })
        .collect();

    let features_children = spec
        .0
        .iter()
        .filter_map(|x| match x {
            vk_parse::RegistryChild::Feature(f) => Some(f),
            _ => None,
        })
        .filter(|feature| contains_desired_api(&feature.api))
        .flat_map(|features| &features.children);

    let extension_children = extensions.iter().flat_map(|extension| &extension.children);

    features_children
        .chain(extension_children)
        .filter_map(|x| match x {
            vk_parse::FeatureChild::Require { api, items, .. } => Some((api, items)),
            _ => None,
        })
        .filter(|(api, _items)| matches!(api.as_deref(), None | Some(DESIRED_API)))
        .flat_map(|(_api, items)| items)
        .fold(
            (HashSet::new(), HashSet::new(), Vec::new()),
            |mut acc, elem| {
                match elem {
                    vk_parse::InterfaceItem::Type { name, .. } => {
                        acc.0.insert(name.as_str());
                    }
                    vk_parse::InterfaceItem::Command { name, .. } => {
                        acc.1.insert(name.as_str());
                    }
                    vk_parse::InterfaceItem::Enum(e) => {
                        acc.2.push(e);
                    }
                    _ => {}
                };
                acc
            },
        )
}

fn trace(builder: &mut String, definition: &CommandDefinition) {
    push_indentation(builder, 1);
    builder.push_str("unsafe { trace!(\"called ");
    builder.push_str(&definition.proto.name);
    builder.push('(');

    let mut first = true;
    for _ in definition.params.iter().unique_by(|x| &x.definition.name) {
        if !first {
            builder.push_str(", ");
        } else {
            first = false;
        }

        builder.push_str("{:?}");
    }
    builder.push_str(")\"");

    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        builder.push_str(", ");

        if param.definition.code.starts_with("const char*") {
            builder.push_str("unpack_cstr(");
            push_param_name(builder, param);
            builder.push(')');
        } else if let Some(len) = param.altlen.as_ref().or(param.len.as_ref()) {
            builder.push_str("unpack_vk_array(");
            push_param_name(builder, param);
            builder.push_str(", (");

            match len.as_str() {
                "(samples + 31) / 32" => builder.push_str("(samples.as_raw() + 31) / 32"),
                _ => to_rust_expression(builder, len),
            };

            builder.push_str(") as usize)");
        } else {
            let is_reference = param.definition.code.chars().any(|x| x == '*')
                && !param.definition.name.ends_with("Count");

            push_param_name(builder, param);
            if is_reference {
                builder.push_str(".as_ref()");
            }
        }
    }

    builder.push_str("); }\n\n");
}

fn push_param_name(builder: &mut String, param: &CommandParam) {
    push_element_name(builder, &param.definition.name);
}

fn push_element_name(builder: &mut String, name: &str) {
    match name == "type" {
        true => builder.push_str("type_"),
        false => to_snake_case(builder, name),
    }
}

fn push_indentation(builder: &mut String, count: usize) {
    for _ in 0..count {
        builder.push_str(INDENTATION);
    }
}

fn to_snake_case(builder: &mut String, text: &str) {
    builder.push_str(&text[..1].to_ascii_lowercase());

    let mut last_floor = false;
    for c in text[1..].chars() {
        if c.is_ascii_uppercase() {
            if !last_floor {
                builder.push('_');
                last_floor = true;
            }
            builder.push(c.to_ascii_lowercase());
        } else {
            builder.push(c);
            last_floor = c == '_';
        }
    }
}

fn to_screaming_snake_case(builder: &mut String, text: &str) {
    let len = builder.len();
    to_snake_case(builder, text);
    builder.replace_range(len.., &builder[len..].to_ascii_uppercase());
}

fn to_rust_expression(builder: &mut String, text: &str) {
    let mut buf = String::new();
    to_snake_case(&mut buf, text);

    if let Some(index) = buf.find("->") {
        buf.replace_range(index..index + 2, ").");
        buf.insert_str(0, "(&*");
    }

    builder.push_str(&buf);
}

fn to_rust_type_without_ptr(type_name: &Option<String>, types: &TypeVulkan) -> String {
    let n = type_name.as_ref().unwrap();

    match n.as_str() {
        "void" => "c_void".into(),
        "uint64_t" => "u64".into(),
        "uint32_t" => "u32".into(),
        "uint16_t" => "u16".into(),
        "uint8_t" => "u8".into(),
        "size_t" => "usize".into(),
        "int" => "std::os::raw::c_int".into(),
        "int64_t" => "i64".into(),
        "int32_t" => "i32".into(),
        "int16_t" => "i16".into(),
        "int8_t" => "i8".into(),
        "float" => "f32".into(),
        "double" => "f64".into(),
        "char" => "c_char".into(),
        "VkSurfaceCounterFlagBitsEXT" => "vk::SurfaceCounterFlagsEXT".into(),
        "VkDebugUtilsMessageSeverityFlagBitsEXT" => "vk::DebugUtilsMessageSeverityFlagsEXT".into(),
        "VkPipelineStageFlagBits" => "vk::PipelineStageFlags".into(),
        "VkExternalMemoryHandleTypeFlagBits" => "vk::ExternalMemoryHandleTypeFlags".into(),
        "VkSampleCountFlagBits" => "vk::SampleCountFlags".into(),
        "VkShaderStageFlagBits" => "vk::ShaderStageFlags".into(),
        "DWORD" => "vk::DWORD".into(),
        "HANDLE" => "vk::HANDLE".into(),
        "HWND" => "vk::HANDLE".into(),
        "HMONITOR" => "vk::HMONITOR".into(),
        "SECURITY_ATTRIBUTES" => "usize".into(),
        "LPCWSTR" => "vk::LPCWSTR".into(),
        "HINSTANCE" => "vk::HINSTANCE".into(),
        "Display" => "usize".into(),
        "Window" => "usize".into(),
        "xcb_connection_t" => "usize".into(),
        "xcb_window_t" => "usize".into(),
        "IDirectFB" => "usize".into(),
        "IDirectFBSurface" => "usize".into(),
        "zx_handle_t" => "vk::zx_handle_t".into(),
        "GgpStreamDescriptor" => "vk::GgpStreamDescriptor".into(),
        "GgpFrameToken" => "vk::GgpFrameToken".into(),
        "RROutput" => "vk::RROutput".into(),
        "VisualID" => "vk::VisualID".into(),
        "xcb_visualid_t" => "vk::xcb_visualid_t".into(),
        "_screen_context" => "usize".into(),
        "_screen_window" => "usize".into(),
        "_screen_buffer" => "usize".into(),
        "wl_display" => "usize".into(),
        "wl_surface" => "usize".into(),
        "ANativeWindow" => "usize".into(),
        "AHardwareBuffer" => "usize".into(),
        "CAMetalLayer" => "c_void".into(),
        "IOSurfaceRef" => "usize".into(),
        _ => {
            if !types.contains_type(n)
                && (n.starts_with("Vk")
                    || n.starts_with("VK_")
                    || n.starts_with("Std")
                    || n.starts_with("MTL"))
            {
                if let Some(stripped) = n.strip_prefix("Vk") {
                    match types.contains_bitmask(n) || types.contains_enumeration(n) {
                        true => return n.to_owned(),
                        false => {}
                    };

                    return match types.contains_handle(n) {
                        true => "NonDisposableHandle".to_owned(),
                        false => format!("vk::{}", stripped),
                    };
                }

                return format!(
                    "vk::{}",
                    match n.strip_prefix("VK_") {
                        Some(stripped) => stripped.to_owned(),
                        None => match n.starts_with("Std") {
                            true => format!("native::{}", n),
                            false => n.to_owned(),
                        },
                    }
                );
            }
            n.clone()
        }
    }
}

fn to_rust_type(name_with_type: &NameWithType, types: &TypeVulkan) -> String {
    to_rust_type_from_name(&name_with_type.type_name, &name_with_type.code, types)
}

fn to_rust_type_from_name(name: &Option<String>, code: &str, types: &TypeVulkan) -> String {
    let n = to_rust_type_without_ptr(name, types);
    append_ptr_to_rust_type(n, code, types)
}

fn append_ptr_to_rust_type(mut name: String, code: &str, types: &TypeVulkan) -> String {
    let mut i = 0;
    while let Some(p) = code[i..].chars().position(|x| x == '*') {
        match code[i..].contains("const") {
            true => name.insert_str(0, "*const "),
            false => name.insert_str(0, "*mut "),
        }
        i += p + 1;
    }

    // Array
    if let Some(p) = code.chars().position(|x| x == '[') {
        // If it is a const array, we need trait this as a pointer
        if code.contains("const") && !name.contains("*const") && !name.contains("*mut") {
            name.insert_str(0, "*const [");
        } else {
            name.insert(0, '[');
        }

        name.push_str("; ");
        name.push_str(&to_rust_type_without_ptr(
            &Some(code[p + 1..code.chars().position(|x| x == ']').unwrap()].to_owned()),
            types,
        ));
        name.push(']');
    }

    // Special cases
    match name.as_str() {
        "*const vk::SampleCountFlags" => "*const u32".to_owned(),
        _ => name,
    }
}

fn contains_desired_api(api: &str) -> bool {
    api.split(',').any(|n| n == DESIRED_API)
}
