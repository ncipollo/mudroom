use std::sync::atomic::{AtomicI64, Ordering};

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

pub fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}
