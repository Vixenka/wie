#[derive(Debug, Copy, Clone, PartialEq)]
pub enum FunctionType {
    Static,
    Entry,
    Instance,
    Device,
}

pub trait CommandExt {
    fn function_type(&self) -> FunctionType;
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
}
