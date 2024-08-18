pub mod instance;

use crate::generated::overrided_indices::*;

pub(crate) fn register_handlers_to(map: &mut crate::HandlerMap) {
    map.insert(VK_CREATE_INSTANCE, Box::new(instance::vk_create_instance));
}
