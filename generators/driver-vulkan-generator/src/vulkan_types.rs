use std::{collections::HashSet, fs, path::Path};

use itertools::Itertools;
use vk_parse::{Registry, Type, TypeMemberDefinition};
use vkxml::{
    Bitmask, DefinitionsElement, Enumeration, EnumerationDeclaration, EnumsElement, Handle,
    RegistryElement,
};

use crate::{push_element_name, push_indentation, to_rust_type_from_name};

pub struct TypeVulkan<'r> {
    pub types: Vec<&'r Type>,
    pub bitmasks: Vec<&'r Bitmask>,
    pub handles: Vec<&'r Handle>,
    pub enumerations: Vec<&'r EnumerationDeclaration>,
    pub enums: Vec<&'r Enumeration>,
}

impl<'r> TypeVulkan<'r> {
    pub fn new(
        registry: &'r Registry,
        vkxml_registry: &'r vkxml::Registry,
        required_types: &HashSet<&str>,
    ) -> Self {
        let types: Vec<_> = registry
            .0
            .iter()
            .filter_map(|x| match x {
                vk_parse::RegistryChild::Types(types) => Some(types),
                _ => None,
            })
            .flat_map(|x| &x.children)
            .filter_map(|x| match x {
                vk_parse::TypesChild::Type(def) => Some(def),
                _ => None,
            })
            .filter(|x| match &x.spec {
                vk_parse::TypeSpec::Members(members) => members
                    .iter()
                    .any(|x| matches!(x, vk_parse::TypeMember::Definition(_))),
                _ => false,
            })
            .filter(|x| required_types.contains(&x.name.as_ref().unwrap().as_str()))
            .collect();

        let bitmasks = vkxml_registry
            .elements
            .iter()
            .filter_map(|x| match x {
                RegistryElement::Definitions(def) => Some(def),
                _ => None,
            })
            .flat_map(|x| &x.elements)
            .filter_map(|x| match x {
                DefinitionsElement::Bitmask(bitmask) => Some(bitmask),
                _ => None,
            })
            .unique_by(|x| &x.name)
            .collect_vec();

        let handles = vkxml_registry
            .elements
            .iter()
            .filter_map(|x| match x {
                RegistryElement::Definitions(def) => Some(def),
                _ => None,
            })
            .flat_map(|x| &x.elements)
            .filter_map(|x| match x {
                DefinitionsElement::Handle(handle) => Some(handle),
                _ => None,
            })
            .unique_by(|x| &x.name)
            .collect_vec();

        let enumerations = vkxml_registry
            .elements
            .iter()
            .filter_map(|x| match x {
                RegistryElement::Definitions(def) => Some(def),
                _ => None,
            })
            .flat_map(|x| &x.elements)
            .filter_map(|x| match x {
                DefinitionsElement::Enumeration(enumeration) => Some(enumeration),
                _ => None,
            })
            .unique_by(|x| &x.name)
            .collect_vec();

        let enums = vkxml_registry
            .elements
            .iter()
            .filter_map(|x| match x {
                RegistryElement::Enums(def) => Some(def),
                _ => None,
            })
            .flat_map(|x| &x.elements)
            .filter_map(|x| match x {
                EnumsElement::Enumeration(enumeration) => Some(enumeration),
                _ => None,
            })
            .collect_vec();

        Self {
            types,
            bitmasks,
            handles,
            enumerations,
            enums,
        }
    }

    pub fn chain(&mut self, types: &'r TypeVulkan) {
        self.types.extend(
            types
                .types
                .iter()
                .filter(|x| !self.contains_type(x.name.as_ref().unwrap()))
                .collect_vec(),
        );
        self.bitmasks.extend(
            types
                .bitmasks
                .iter()
                .filter(|x| !self.contains_bitmask(&x.name))
                .collect_vec(),
        );
        self.handles.extend(
            types
                .handles
                .iter()
                .filter(|x| !self.contains_handle(&x.name))
                .collect_vec(),
        );
        self.enumerations.extend(
            types
                .enumerations
                .iter()
                .filter(|x| !self.contains_enumeration(&x.name))
                .collect_vec(),
        );
        self.enums.extend(
            types
                .enums
                .iter()
                .filter(|x| !self.contains_enum(&x.name))
                .collect_vec(),
        );
    }

    pub fn contains_type(&self, name: &str) -> bool {
        self.types.iter().any(|x| {
            if let Some(n) = &x.name {
                n == name
            } else {
                false
            }
        })
    }

    pub fn contains_bitmask(&self, name: &str) -> bool {
        self.bitmasks.iter().any(|x| x.name == name)
    }

    pub fn contains_handle(&self, name: &str) -> bool {
        self.handles.iter().any(|x| x.name == name)
    }

    pub fn contains_enumeration(&self, name: &str) -> bool {
        self.enumerations.iter().any(|x| x.name == name)
    }

    pub fn contains_enum(&self, name: &str) -> bool {
        self.enums.iter().any(|x| x.name == name)
    }
}

pub fn generate_vulkan_types(project_directory: &Path, types: &TypeVulkan) {
    let mut builder = String::new();
    builder.push_str("//! THIS FILE IS GENERATED BY TOOL, DO NOT MODIFY.\n\nuse std::ffi::{c_char, c_void};\nuse crate::{NonDisposableHandle, unimplemented_serializer, unimplemented_deserializer, unimplemented_deserializer_mut, generated::vulkan_enums::*, generated::vulkan_pfn_functions::*, generated::vulkan_bitmasks::*, generated::p_next::*};\nuse cdump::{CDeserialize, CSerialize, CDebug};\nuse ash::vk;\n");

    for ty in &types.types {
        generate_type(&mut builder, ty, types);
    }

    let path = project_directory.join("crates/driver-common-vulkan/src/generated/vulkan_types.rs");
    fs::create_dir_all(path.parent().unwrap()).expect("create directories");
    fs::write(path, builder).expect("write to a file");
}

pub fn generate_type(builder: &mut String, ty: &Type, types: &TypeVulkan) {
    let type_name = ty.name.as_ref().unwrap();

    builder
        .push_str("\n#[doc = \"https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/");
    builder.push_str(type_name);
    builder
        .push_str(".html\"]\n#[derive(CDebug, Clone, Copy, CDeserialize, CSerialize)]\n#[repr(C)]\npub struct ");
    builder.push_str(type_name);
    builder.push_str(" {\n");

    let members = match &ty.spec {
        vk_parse::TypeSpec::Members(m) => m,
        _ => unreachable!(),
    };

    let mut last_name = None;
    for member in members {
        let def = match member {
            vk_parse::TypeMember::Definition(def) => def,
            _ => continue,
        };

        let member_type = def
            .markup
            .iter()
            .find_map(|x| match x {
                vk_parse::TypeMemberMarkup::Type(t) => Some(t),
                _ => None,
            })
            .cloned();
        let member_name = def
            .markup
            .iter()
            .find_map(|x| match x {
                vk_parse::TypeMemberMarkup::Name(t) => Some(t),
                _ => None,
            })
            .unwrap();

        if last_name == Some(member_name) {
            continue;
        }
        last_name = Some(member_name);

        push_array_attribute(builder, def);

        let ty = to_rust_type_from_name(&member_type, &def.code, types);

        // Add dynamic attribute
        let is_dynamic =
            ty == "*const c_void" || ty == "*mut c_void" || ty == "*const *const c_void";
        if is_dynamic && (!is_dynamic_type_exception(member_name, type_name) || def.len.is_none()) {
            push_indentation(builder, 1);
            if member_name == "pNext" {
                builder.push_str("#[cdump(dynamic(serializer = p_next_serializer, deserializer = p_next_deserializer, cdebugger = p_next_cdebugger))]\n");
            } else {
                builder.push_str("#[cdump(dynamic(serializer = unimplemented_serializer, deserializer = unimplemented_deserializer");
                match ty == "*mut c_void" {
                    true => builder.push_str("_mut"),
                    false => (),
                };
                builder.push_str("))]\n");
            }
        }

        push_indentation(builder, 1);
        builder.push_str("pub ");
        push_element_name(builder, member_name);
        builder.push_str(": ");

        // Omit pointer for pData in types like https://registry.khronos.org/vulkan/specs/1.3-extensions/man/html/VkSpecializationInfo.html
        if is_dynamic && is_dynamic_type_exception(member_name, type_name) && def.len.is_some() {
            builder.push_str("*const u8");
        } else if type_name == "VkLayerSettingEXT" && member_name == "pValues" {
            builder.push_str("*const *const c_void /* TODO: Add correct type */")
        } else {
            builder.push_str(&ty);
        }

        builder.push_str(",\n");
    }

    builder.push_str("}\n");
}

fn is_dynamic_type_exception(member_name: &str, type_name: &str) -> bool {
    matches!(member_name, "pData" | "pInitialData" | "pTag" | "pCode")
        || (type_name == "VkPushConstantsInfoKHR" && matches!(member_name, "pValues"))
}

fn push_array_attribute(builder: &mut String, def: &TypeMemberDefinition) {
    let Some(len) = def.len.as_ref() else {
        return;
    };
    if len == "null-terminated" {
        return;
    }

    let len = if let Some(index) = len.find(',') {
        &len[0..index]
    } else {
        len
    };
    let len = len.trim();

    push_indentation(builder, 1);
    builder.push_str("#[cdump(array(len = ");

    if let Some(altlen) = def.altlen.as_ref() {
        builder.push_str(match altlen.as_str() {
            "codeSize / 4" => "self.code_size / 4",
            "(rasterizationSamples + 31) / 32" => "(self.rasterization_samples.as_raw() + 31) / 32",
            "2*VK_UUID_SIZE" => "2 * vk::UUID_SIZE",
            "(samples + 31) / 32" => "(self.samples.as_raw() + 31) / 32",
            _ => "unimplemented!(\"altlen for this type is not implemented\")",
        });
    } else {
        let len = if let Some(index) = len.find(',') {
            &len[0..index]
        } else {
            len
        };
        let len = len.trim();

        builder.push_str("self.");
        push_element_name(builder, len);
    }

    builder.push_str("))]\n");
}
