use clap::CommandFactory;
use clap::Parser;
use clap::Subcommand;
use clap_complete::{Shell, generate};
use crossbeam::channel::{Sender, unbounded};
use crossterm::{
    cursor::Show,
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::Write;
use std::{
    fs, io, panic,
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};
use turm::{App, AppExit, SqueueArgs};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Refresh rate for the job watcher.
    #[arg(long, value_name = "SECONDS", default_value_t = 2)]
    slurm_refresh: u64,

    /// Refresh rate for the file watcher.
    #[arg(long, value_name = "SECONDS", default_value_t = 2)]
    file_refresh: u64,

    /// Slurm job filters
    #[command(flatten)]
    squeue_args: SqueueArgs,

    /// Write the selected directory here when exiting.
    #[arg(long, value_name = "PATH")]
    cwd_file: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<CliCommand>,
}

#[derive(Subcommand)]
enum CliCommand {
    /// Print shell completion script to stdout.
    Completion {
        /// The shell to generate completion for.
        shell: Shell,
    },
}

fn main() -> io::Result<()> {
    let args = Cli::parse();
    match args.command {
        Some(CliCommand::Completion { shell }) => {
            let cmd = &mut Cli::command();
            generate(shell, cmd, cmd.get_name().to_string(), &mut io::stdout());
            return Ok(());
        }
        None => {}
    }

    install_panic_hook();

    run_app(args)
}

fn install_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            LeaveAlternateScreen,
            DisableBracketedPaste,
            DisableMouseCapture,
            Show
        );
        default_hook(panic_info);
    }));
}

struct TerminalGuard<W: Write> {
    terminal: Terminal<CrosstermBackend<W>>,
}

impl<W: Write> TerminalGuard<W> {
    fn new(mut writer: W) -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(
            writer,
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableMouseCapture
        )?;
        let backend = CrosstermBackend::new(writer);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<W>> {
        &mut self.terminal
    }
}

impl<W: Write> Drop for TerminalGuard<W> {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            self.terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableBracketedPaste,
            DisableMouseCapture
        );
        let _ = self.terminal.show_cursor();
    }
}

fn input_loop(tx: Sender<std::io::Result<Event>>, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match event::poll(Duration::from_millis(50)) {
            Ok(true) if tx.send(event::read()).is_err() => break,
            Ok(true) | Ok(false) => {}
            Err(_) => break,
        }
    }
}

fn run_app(args: Cli) -> io::Result<()> {
    let (input_tx, input_rx) = unbounded();
    let cwd_file = args.cwd_file;
    let mut app = App::new(
        input_rx,
        args.slurm_refresh,
        args.file_refresh,
        args.squeue_args.to_vec(),
    );
    loop {
        let mut terminal_guard = TerminalGuard::new(io::stdout())?;
        let stop_input = Arc::new(AtomicBool::new(false));
        let input_stop = Arc::clone(&stop_input);
        let input_tx = input_tx.clone();
        let input_thread = thread::spawn(move || input_loop(input_tx, input_stop));
        let action = app.run(terminal_guard.terminal_mut());
        stop_input.store(true, Ordering::Relaxed);
        let _ = input_thread.join();
        let action = action?;
        drop(terminal_guard);

        match action {
            Some(AppExit::OpenEditor(path)) => {
                let status = Command::new("vi").arg(path).status()?;
                if !status.success() {
                    return Err(io::Error::other(format!("vi exited with {status}")));
                }
            }
            Some(AppExit::ChangeDirectory(path)) => {
                std::env::set_current_dir(&path)?;
                if let Some(cwd_file) = &cwd_file {
                    fs::write(cwd_file, path.as_os_str().as_encoded_bytes())?;
                }
                return Ok(());
            }
            None => return Ok(()),
        }
    }
}
