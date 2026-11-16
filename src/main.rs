use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use steamworks::{Client, SIResult, UGC};

pub const LOR_ID: u32 = 1256670;

fn main() {}

#[derive(Default)]
struct Hook {
    b: AtomicBool,
}

impl Hook {
    fn ping(&self) {
        self.b.store(true, Ordering::SeqCst);
    }

    fn is_ping(&self) -> bool {
        self.b.load(Ordering::SeqCst)
    }
}

struct TaskRunner {
    client: Client,
    ugc: UGC,
}

impl TaskRunner {
    fn new(id: u32) -> SIResult<Self> {
        let client = Client::init_app(id)?;

        Ok(Self {
            ugc: client.ugc(),
            client,
        })
    }

    fn ugc(&self) -> &UGC {
        &self.ugc
    }

    fn run_task<F: FnOnce(&Self, Arc<Hook>)>(&self, task: F) {
        let hook = Arc::new(Hook::default());

        task(self, hook.clone());

        loop {
            if hook.is_ping() {
                break;
            }

            self.client.run_callbacks();

            thread::sleep(Duration::from_millis(100));
        }
    }
}
