use crate::id::Id;
use crate::prt;
use std::{
    sync::mpsc::Sender,
    task::{RawWaker, RawWakerVTable},
};
use std::task::Waker;

#[derive(Debug, Clone)]
pub struct WakerData {
    poll_me: Sender<Id>,
    id: Id,
}


impl WakerData {
    pub fn new_waker(poll_me: Sender<Id>, id: Id) -> Waker {
        prt!("[wakerdata] create new {id:?}");
        let waker_data = Box::new(Self {
            poll_me,
            id
        });
        let waker_data = Box::into_raw(waker_data);

        let raw_waker = RawWaker::new(waker_data as *const _, &VTABLE);
        unsafe { Waker::from_raw(raw_waker) }
    }
}

const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, drop);

unsafe fn clone(data: *const ()) -> RawWaker {
    let old_data = &*(data.cast::<WakerData>());
    prt!("[vtable] clone: {:?}", old_data.id);

    let new_data = Box::new(old_data.clone());

    RawWaker::new(Box::into_raw(new_data) as *const _, &VTABLE)
}

unsafe fn wake(data: *const ()) {
    let data = Box::from_raw(data as *mut WakerData);
    prt!("[vtable] wake owned {:?}", data.id);
    data.poll_me
        .send(data.id)
        .expect("unable to send task id to executor");
}

unsafe fn wake_by_ref(data: *const ()) {
    let data = &*(data.cast::<WakerData>());
    prt!("[vtable] wake reference {:?}", data.id);
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
    const fn send_and_sync() {
        const fn test_sync_send<T: Sync + Send>() {}

        test_sync_send::<WakerData>();
    }
}
