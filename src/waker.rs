use std::{
    sync::mpsc::Sender, task::{RawWaker, RawWakerVTable}
};

use crate::WakeResult;

#[derive(Clone)]
pub struct WakerData {
    pub tasks_sender: Sender<(usize, WakeResult)>,
    pub id: usize,
}

pub const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone(data: *const ()) -> RawWaker {
    let old_data = std::ptr::read(data as *const WakerData);
    let new_data = WakerData {
        id: old_data.id,
        tasks_sender: old_data.tasks_sender.clone()
    };
    std::mem::forget(old_data);

    let boxed = Box::new(new_data);
    let raw_ptr = Box::into_raw(boxed);

    RawWaker::new(raw_ptr as *const (), &VTABLE)
}

unsafe fn wake(data: *const ()) {
    let data = std::ptr::read(data as *const WakerData);
    // println!("[vtable] wake: {}", data.id);
    data.tasks_sender
        .send((data.id, WakeResult::Owned))
        .expect("unable to send task id to executor");
    std::mem::forget(data);
}

unsafe fn wake_by_ref(data: *const ()) {
    let data = std::ptr::read(data as *const WakerData);
    // println!("[vtable] wake_by_ref: {}", data.id);
    data.tasks_sender
        .send((data.id, WakeResult::ByRef))
        .expect("unable to send task id to executor");
    std::mem::forget(data);
}

unsafe fn drop(data: *const ()) {
    let data = std::ptr::read(data as *const WakerData);
    // println!("[vtable] drop: {}", data.id);
    std::mem::drop(data);
}
