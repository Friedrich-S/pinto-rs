use crate::threads::Thread;
use alloc::string::String;
use alloc::sync::Arc;

#[derive(Debug)]
pub struct Process {
    page_dir: (), // ToDo,
    name: String,
    main_thread: Arc<Thread>,
}
