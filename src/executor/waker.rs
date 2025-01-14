use crate::id::Id;
use crate::prt;
use std::{
    sync::mpsc::Sender,
    task::{RawWaker, RawWakerVTable},
};

#[derive(Debug)]
pub struct WakerData {
    poll_me: Sender<Id>,
    id: Id,
}

impl WakerData {
    pub fn new(poll_me: Sender<Id>, id: Id) -> Self {
        prt!("[wakerdata] create new {id:?}");
        Self { poll_me, id }
    }
}

pub const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone(data: *const ()) -> RawWaker {
    let old_data = &*(data.cast::<WakerData>());
    prt!("[vtable] clone: {old_data:?}");

    let new_data = Box::new(WakerData {
        poll_me: old_data.poll_me.clone(),
        id: old_data.id,
    });

    RawWaker::new(Box::into_raw(new_data) as *const _, &VTABLE)
}

unsafe fn wake(data: *const ()) {
    let data = Box::from_raw(data as *mut WakerData);
    prt!("[vtable] wake O {:?}", data.id);
    data.poll_me
        .send(data.id)
        .expect("unable to send task id to executor");
}

unsafe fn wake_by_ref(data: *const ()) {
    let data = &*(data.cast::<WakerData>());
    prt!("[vtable] wake R {:?}", data.id);
    data.poll_me
        .send(data.id)
        .expect("unable to send task id to executor");
}

unsafe fn drop(data: *const ()) {
    let _ = Box::from_raw(data as *mut WakerData);
}

#[cfg(test)]
mod tests {
    use crate::executor::waker::WakerData;

    #[test]
    fn send_and_sync() {
        fn test_sync<T: Sync>() {}
        fn test_send<T: Send>() {}

        test_sync::<WakerData>();
        test_send::<WakerData>();
    }
}
