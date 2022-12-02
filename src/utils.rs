use async_executor::LocalExecutor;
use futures_lite::Future;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, task::Context};
pub fn merge_value(value: &mut Value, new_value: &Value) {
    if let Some(a) = value.as_object_mut() {
        if let Some(b) = new_value.as_object() {
            for (k, obj) in b {
                match a.entry(k) {
                    serde_json::map::Entry::Vacant(e) => {
                        e.insert(obj.clone());
                    }
                    serde_json::map::Entry::Occupied(o) => {
                        merge_value(o.into_mut(), obj);
                    }
                }
            }
        }
    }
}
pub fn run_ex(ex: LocalExecutor<'_>) -> ! {
    let this = std::thread::current();
    let waker = waker_fn::waker_fn(move || {
        this.unpark();
    });
    let mut cx = Context::from_waker(&waker);
    loop {
        while ex.try_tick() {}

        let fut = ex.tick();
        futures_lite::pin!(fut);

        match fut.poll(&mut cx) {
            std::task::Poll::Ready(_) => (),
            std::task::Poll::Pending => std::thread::park(),
        }
    }
}
