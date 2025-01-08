use std::{
    sync::mpsc::Sender, task::{RawWaker, RawWakerVTable}
};
use crate::{prt, Id};

#[derive(Debug, Clone)]
pub struct WakerData {
    tasks_sender: Sender<Id>,
    id: Id,
}

impl WakerData {
    pub fn new (tasks_sender: Sender<Id>, id: Id) -> Self {
        prt!("[wakerdata] create new {id:?}");
        Self {
            tasks_sender, id
        }
    }
}

pub const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone (data: *const ()) -> RawWaker {
    let old_data = std::ptr::read(data as *const WakerData);
    let new_data = old_data.clone();
    prt!("[vtable] clone: {new_data:?}");
    std::mem::forget(old_data);

    let boxed = Box::new(new_data);
    let raw_ptr = Box::into_raw(boxed);

    RawWaker::new(raw_ptr as *const (), &VTABLE)
}

unsafe fn wake(data: *const ()) {
    let data = std::ptr::read(data as *const WakerData);
    prt!("[vtable] wake O {:?}", data.id);
    data.tasks_sender
        .send(data.id)
        .expect("unable to send task id to executor");
    std::mem::forget(data);
}

unsafe fn wake_by_ref(data: *const ()) {
    let data = std::ptr::read(data as *const WakerData);
    prt!("[vtable] wake R {:?}", data.id);
    data.tasks_sender
        .send(data.id)
        .expect("unable to send task id to executor");
    std::mem::forget(data);
}

unsafe fn drop (data: *const ()) {
    std::ptr::drop_in_place(data as *mut WakerData);
}