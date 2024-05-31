use std::{collections::HashSet, path::Path};

use itertools::Itertools;
use vk_parse::{CommandDefinition, CommandParam, Extension, NameWithType};

const DESIRED_API: &str = "vulkan";
const INDENTATION: &str = "    ";

mod driver;
mod listener;
mod transport;

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
        &directory.join("generators/driver-vulkan-generator/Vulkan-Headers/registry/vk.xml"),
        &directory,
    );
}

fn generate(vk_headers_path: &Path, project_directory: &Path) {
    let (spec, _errors) = vk_parse::parse_file(vk_headers_path).expect("invalid XML file");

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

    let required_commands = features_children
        .chain(extension_children)
        .filter_map(|x| match x {
            vk_parse::FeatureChild::Require { api, items, .. } => Some((api, items)),
            _ => None,
        })
        .filter(|(api, _items)| matches!(api.as_deref(), None | Some(DESIRED_API)))
        .flat_map(|(_api, items)| items)
        .fold(HashSet::new(), |mut acc, elem| {
            if let vk_parse::InterfaceItem::Command { name, .. } = elem {
                acc.insert(name.as_str());
            }
            acc
        });

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

    println!("Generating driver...");
    driver::generate(project_directory, &commands);
    println!("Generating listener...");
    listener::generate(project_directory, &commands);
}

fn trace(builder: &mut String, definition: &CommandDefinition, check_count: bool) {
    push_indentation(builder, 1);
    builder.push_str("trace!(\"called ");
    builder.push_str(&definition.proto.name);
    builder.push('(');

    let mut first = true;
    let mut last_is_count = false;
    for param in definition.params.iter().unique_by(|x| &x.definition.name) {
        if check_count {
            if last_is_count {
                continue;
            } else {
                last_is_count = param.definition.name.ends_with("Count");
            }
        }

        if !first {
            builder.push_str(", ");
        } else {
            first = false;
        }

        builder.push('{');
        push_param_name(builder, param);
        builder.push_str(":?}");
    }

    builder.push_str(")\");\n\n");
}

fn push_param_name(builder: &mut String, param: &CommandParam) {
    match param.definition.name == "type" {
        true => builder.push_str("type_"),
        false => to_snake_case(builder, &param.definition.name),
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
            last_floor = false;
        }
    }
}

fn to_rust_type_without_ptr(name_with_type: &NameWithType) -> String {
    let type_name = name_with_type.type_name.as_ref().unwrap();
    let mut n = type_name.replace("Vk", "");

    match n.as_str() {
        "void" => "std::ffi::c_void".into(),
        "uint64_t" => "u64".into(),
        "uint32_t" => "u32".into(),
        "uint16_t" => "u16".into(),
        "size_t" => "isize".into(),
        "int" => "std::os::raw::c_int".into(),
        "int32_t" => "i32".into(),
        "float" => "f32".into(),
        "char" => "std::os::raw::c_char".into(),
        "SurfaceCounterFlagBitsEXT" => "vk::SurfaceCounterFlagsEXT".into(),
        "DebugUtilsMessageSeverityFlagBitsEXT" => "vk::DebugUtilsMessageSeverityFlagsEXT".into(),
        "PipelineStageFlagBits" => "vk::PipelineStageFlags".into(),
        "ExternalMemoryHandleTypeFlagBits" => "vk::ExternalMemoryHandleTypeFlags".into(),
        "SampleCountFlagBits" => "vk::SampleCountFlags".into(),
        "ShaderStageFlagBits" => "vk::ShaderStageFlags".into(),
        _ => {
            n.insert_str(0, "vk::");
            n
        }
    }
}

fn to_rust_type(name_with_type: &NameWithType) -> String {
    let mut n = to_rust_type_without_ptr(name_with_type);

    let mut i = 0;
    while let Some(p) = name_with_type.code[i..].chars().position(|x| x == '*') {
        match name_with_type.code[i..].contains("const ") {
            true => n.insert_str(0, "*const "),
            false => n.insert_str(0, "*mut "),
        }
        i += p + 1;
    }

    n
}

fn contains_desired_api(api: &str) -> bool {
    api.split(',').any(|n| n == DESIRED_API)
}
