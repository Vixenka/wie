use crate::to_rust_type;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionType {
    Static,
    Entry,
    Instance,
    Device,
}

pub trait CommandExt {
    fn function_type(&self) -> FunctionType;
    fn is_return_data(&self) -> bool;
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

    fn is_return_data(&self) -> bool {
        if to_rust_type(&self.proto) != "std::ffi::c_void" {
            return true;
        }

        for param in self.params.iter() {
            if param.is_return_data() {
                return true;
            }
        }
        false
    }
}

pub trait CommandParamExt {
    fn is_return_data(&self) -> bool;
}

impl CommandParamExt for vk_parse::CommandParam {
    fn is_return_data(&self) -> bool {
        to_rust_type(&self.definition).contains("*mut")
    }
}
