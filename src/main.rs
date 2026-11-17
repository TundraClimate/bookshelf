use crossterm::cursor::MoveToColumn;
use crossterm::queue;
use crossterm::style::{Print, Stylize};
use gag::Gag;
use inquire::InquireError;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::mem;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use steamworks::{Client, PublishedFileId, SIResult, UGC};

const LOR_ID: u32 = 1256670;

fn main() {
    if let Err(e) = init() {
        eprintln!("Initialize failed: {e}");

        return;
    }

    let Ok(tr) = TaskRunner::new(LOR_ID) else {
        eprintln!("Exit application: steam client not launched");

        return;
    };

    println!("Hello manager.");

    if let Err(e) = run_app(tr) {
        eprintln!("Exit application: {e}");
    }

    println!();
    println!("Goodbye manager.");
}

fn save_slot_dir() -> Result<PathBuf, String> {
    let exd = env::current_exe()
        .map_err(|_| "exec file not found".to_string())
        .and_then(|p| {
            p.parent()
                .ok_or("exec dir not found".to_string())
                .map(|p| p.to_path_buf())
        })?;

    Ok(exd.join("save_slots"))
}

fn is_empties<T: AsRef<str>>(s: T) -> bool {
    matches!(s.as_ref(), "" | "\n" | "\r\n" | "\r" | "\t")
}

fn init() -> Result<(), String> {
    let save_slots = save_slot_dir()?;

    if !save_slots.exists() {
        fs::create_dir_all(&save_slots)
            .map_err(|_| format!("creates directory '{}'", save_slots.display()))?;

        println!("Creates directory '{}'", save_slots.display());
    }

    Ok(())
}

fn run_app(tr: TaskRunner) -> Result<(), String> {
    loop {
        println!();
        println!("commands: help quit save load forgot");

        match inquire::prompt_text("Choose your command") {
            Ok(txt) => {
                if command(txt, &tr)? {
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

fn command(input: String, tr: &TaskRunner) -> Result<bool, String> {
    match input.trim() {
        "help" => help(),
        "quit" => return Ok(true),
        "save" => save(tr)?,
        "load" => {}
        "forgot" => {}
        txt if is_empties(txt) => {}

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

fn save(tr: &TaskRunner) -> Result<(), String> {
    let save_slots = save_slot_dir()?;

    let Ok(slot_name) = inquire::prompt_text("Profile name") else {
        println!("Save cancelled.");

        return Ok(());
    };

    if is_empties(&slot_name) {
        println!("Profile needs name have 1 or higher characters.");

        return Ok(());
    }

    let slot_path = save_slots.join(format!("{slot_name}.txt"));

    if slot_path.exists() {
        println!("Profile '{}' is already exists.", slot_name);

        return Ok(());
    }

    let items = tr.ugc().subscribed_items(false);
    let items = items
        .into_iter()
        .map(|i| i.0.to_string())
        .collect::<Vec<_>>()
        .join(":");

    fs::write(&slot_path, &items).map_err(|_| "profile slots not writable".to_string())?;

    println!("New profile '{slot_name}' successfully saved.");

    Ok(())
}

fn subscribe(tr: &TaskRunner, id: PublishedFileId) {
    tr.run_task(|runner, hook| {
        let mut stdout = io::stdout();

        queue!(
            stdout,
            Print("| ".yellow()),
            Print("Subscribe item: "),
            Print(id.0.to_string().yellow()),
        )
        .unwrap();

        stdout.flush().unwrap();

        let gst = Gag::stdout().unwrap();
        let ger = Gag::stderr().unwrap();

        runner.ugc().subscribe_item(id, move |res| {
            mem::drop(gst);
            mem::drop(ger);

            match res {
                Ok(_) => queue!(
                    stdout,
                    MoveToColumn(0),
                    Print("| ".green()),
                    Print("Subscribe success: "),
                    Print(id.0.to_string().green()),
                )
                .unwrap(),
                Err(_) => queue!(
                    stdout,
                    MoveToColumn(0),
                    Print("| ".red()),
                    Print("Subscribe failed: "),
                    Print(id.0.to_string().red()),
                )
                .unwrap(),
            }

            println!();

            hook.ping();
        });
    });
}

fn unsubscribe(tr: &TaskRunner, id: PublishedFileId) {}

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
        let client;

        {
            let _stdout = Gag::stdout().unwrap();
            let _stderr = Gag::stderr().unwrap();

            client = Client::init_app(id)?;
        }

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
