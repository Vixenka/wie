use std::collections::HashSet;

use crate::{to_rust_type, vulkan_types::TypeVulkan};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionType {
    Static,
    Entry,
    Instance,
    Device,
}

pub trait CommandExt {
    fn function_type(&self) -> FunctionType;
    fn is_return_data(&self, types: &TypeVulkan) -> bool;
    fn get_alias(&self, required_commands: &HashSet<&str>) -> Option<String>;
}

impl CommandExt for vk_parse::CommandDefinition {
    fn function_type(&self) -> FunctionType {
        let is_first_param_device = self.params.first().map_or(false, |field| {
            matches!(
                field.definition.type_name.as_deref(),
                Some("VkDevice" | "VkCommandBuffer" | "VkQueue")
            )
        });
        match self.proto.name.as_str() {
            "vkGetInstanceProcAddr" => FunctionType::Static,
            "vkCreateInstance"
            | "vkEnumerateInstanceLayerProperties"
            | "vkEnumerateInstanceExtensionProperties"
            | "vkEnumerateInstanceVersion" => FunctionType::Entry,
            // This is actually not a device level function
            "vkGetDeviceProcAddr" => FunctionType::Instance,
            _ if is_first_param_device => FunctionType::Device,
            _ => FunctionType::Instance,
        }
    }

    fn is_return_data(&self, types: &TypeVulkan) -> bool {
        if to_rust_type(&self.proto, types) != "std::ffi::c_void" {
            return true;
        }

        for param in self.params.iter() {
            if param.is_return_data(types) {
                return true;
            }
        }
        false
    }

    fn get_alias(&self, required_commands: &HashSet<&str>) -> Option<String> {
        fn get_alias_inner(
            name: &str,
            suffix: &str,
            required_commands: &HashSet<&str>,
        ) -> Option<String> {
            let alias = format!("{name}{suffix}");
            match required_commands.contains(alias.as_str()) {
                true => Some(alias),
                false => None,
            }
        }

        if self.proto.name.ends_with("KHR") || self.proto.name.ends_with("EXT") {
            if let Some(alias) = get_alias_inner(
                &self.proto.name[..self.proto.name.len() - 3],
                "EXT",
                required_commands,
            ) {
                return Some(alias);
            }
        }

        if let Some(alias) = get_alias_inner(&self.proto.name, "KHR", required_commands) {
            return Some(alias);
        }
        if let Some(alias) = get_alias_inner(&self.proto.name, "EXT", required_commands) {
            return Some(alias);
        }
        None
    }
}

pub trait CommandParamExt {
    fn is_return_data(&self, types: &TypeVulkan) -> bool;
}

impl CommandParamExt for vk_parse::CommandParam {
    fn is_return_data(&self, types: &TypeVulkan) -> bool {
        to_rust_type(&self.definition, types).contains("*mut")
    }
}
