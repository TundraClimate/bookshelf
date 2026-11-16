use inquire::InquireError;
use std::env;
use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use steamworks::{Client, SIResult, UGC};

pub const LOR_ID: u32 = 1256670;

fn main() {
    if let Err(e) = init() {
        eprintln!("Initialize failed: {e}");

        return;
    }

    println!("Hello manager.");

    if let Err(e) = run_app() {
        eprintln!("Exit application: {e}");
    }

    println!();
    println!("Goodbye manager.");
}

fn init() -> Result<(), String> {
    let exd = env::current_exe()
        .map_err(|_| "exec file not found".to_string())
        .and_then(|p| {
            p.parent()
                .ok_or("exec dir not found".to_string())
                .map(|p| p.to_path_buf())
        })?;

    let save_slots = exd.join("save_slots");

    if !save_slots.exists() {
        fs::create_dir_all(&save_slots)
            .map_err(|_| format!("creates directory '{}'", save_slots.display()))?;

        println!("Creates directory '{}'", save_slots.display());
    }

    Ok(())
}

fn run_app() -> Result<(), String> {
    loop {
        println!();
        println!("commands: help quit save load forgot");

        match inquire::prompt_text("Choose your command") {
            Ok(txt) => {
                if command(txt)? {
                    break;
                }
            }
            Err(e) => match e {
                InquireError::OperationInterrupted => break,

                _ => return Err("closed".to_string()),
            },
        }
    }

    Ok(())
}

fn command(input: String) -> Result<bool, String> {
    match input.trim() {
        "help" => help(),
        "quit" => return Ok(true),
        "save" => {}
        "load" => {}
        "forgot" => {}
        "" | "\n" | "\r\n" | "\r" | "\t" => {}

        txt => println!("'{txt}' is not command; try again."),
    }

    Ok(false)
}

fn help() {
    println!();
    println!("command help:");
    println!("- help:   prints command help");
    println!("- quit:   exit application");
    println!("- save:   saves current sub items to profile");
    println!("- load:   loads saved sub items from profile");
    println!("- forgot: removes saved profile");
    println!();
    println!("Ctrl+c exit application");
}

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
