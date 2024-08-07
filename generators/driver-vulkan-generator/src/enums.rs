use std::{fs, path::Path};

use vk_parse::{Enum, EnumSpec};

pub fn generate_enums(project_directory: &Path, enums: &[&Enum]) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse super::*;\n\n");

    for e in enums {
        let EnumSpec::Value { value, .. } = &e.spec else {
            continue;
        };

        let ty = match value.starts_with('"') {
            true => "&str",
            false => "usize",
        };

        builder.push_str("pub const ");
        builder.push_str(&e.name);
        builder.push_str(": ");
        builder.push_str(ty);
        builder.push_str(" = ");
        builder.push_str(value);
        builder.push_str(";\n");
    }

    let path = project_directory.join("crates/driver-common-vulkan/src/generated/vulkan_enums.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}
