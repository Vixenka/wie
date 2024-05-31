use std::collections::HashMap;

use wie_transport_guest::Handler;

pub fn get() -> HashMap<u64, Handler> {
    HashMap::new()
}
