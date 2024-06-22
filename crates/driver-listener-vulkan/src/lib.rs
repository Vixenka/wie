use std::{collections::HashMap, sync::OnceLock};

use wie_transport::Handler;
use wie_transport_vsock::VsockStream;

#[macro_use]
extern crate log;

pub(crate) mod entry;
pub(crate) mod generated;

static ENTRY: OnceLock<ash::Entry> = OnceLock::new();

type HandlerMap = HashMap<u64, Handler<VsockStream>>;
type Packet<'c> = wie_transport::packet::Packet<'c, VsockStream>;

pub fn register_handlers_to(map: &mut HandlerMap) {
    entry::register_handlers_to(map);
    generated::handlers::register_handlers_to(map);
}

/// # Safety
/// This functions loads native libraries which cannot be simply dropped.
pub unsafe fn get_or_init_entry() -> &'static ash::Entry {
    ENTRY.get_or_init(|| ash::Entry::load().unwrap())
}

#[inline]
pub fn get_entry() -> &'static ash::Entry {
    ENTRY.get().unwrap()
}

#[inline]
pub fn get_entry_v1_0() -> &'static ash::EntryFnV1_0 {
    get_entry().fp_v1_0()
}
