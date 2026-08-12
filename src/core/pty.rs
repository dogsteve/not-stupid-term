use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::thread;
use tempfile::TempDir;

#[cfg(unix)]
use libc;

pub enum PtyEvent {
    CommandStart,
    CommandEnd { exit_code: i32 },
    Output(Vec<u8>),
    ClearScreen,
    EnterAltScreen,
    LeaveAltScreen,
}

pub struct PtySession {
    pub process_id: Option<u32>,
    pub receiver: std::sync::mpsc::Receiver<PtyEvent>,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    last_size: (u16, u16),
    _temp_dir: Option<TempDir>,
}

struct OscInterceptor {
    sender: std::sync::mpsc::Sender<PtyEvent>,
    buffer: Vec<u8>,
}

impl OscInterceptor {
    fn flush(&mut self) {
        if !self.buffer.is_empty() {
            let _ = self.sender.send(PtyEvent::Output(self.buffer.clone()));
            self.buffer.clear();
        }
    }
}

impl vte::Perform for OscInterceptor {
    fn print(&mut self, c: char) {
        let mut b = [0; 4];
        self.buffer.extend_from_slice(c.encode_utf8(&mut b).as_bytes());
    }
    fn execute(&mut self, byte: u8) {
        self.buffer.push(byte);
    }
    fn hook(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {
        self.buffer.push(0x1b);
        self.buffer.push(b'P');
        for p in params.iter() {
            for _b in p.iter() {
            }
        }
    }
    fn put(&mut self, byte: u8) {
        self.buffer.push(byte);
    }
    fn unhook(&mut self) {
        self.buffer.push(0x1b);
        self.buffer.push(b'\\');
    }
    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        let mut handled = false;
        if !params.is_empty() && params[0] == b"1337" {
            if params.len() > 1 {
                if params[1] == b"SmartTermCmdStart" {
                    eprintln!("[DEBUG PTY OSC] Received SmartTermCmdStart");
                    self.flush();
                    let _ = self.sender.send(PtyEvent::CommandStart);
                    handled = true;
                } else if params[1].starts_with(b"SmartTermCmdEnd") {
                    eprintln!("[DEBUG PTY OSC] Received SmartTermCmdEnd: {:?}", std::str::from_utf8(params[1]));
                    self.flush();
                    let parts = params[1].split(|&b| b == b';').collect::<Vec<_>>();
                    let exit_code = if parts.len() > 1 {
                        std::str::from_utf8(parts[1]).unwrap_or("0").parse().unwrap_or(0)
                    } else {
                        0
                    };
                    let _ = self.sender.send(PtyEvent::CommandEnd { exit_code });
                    handled = true;
                }
            }
        }
        
        if !handled {
            self.buffer.push(0x1b);
            self.buffer.push(b']');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    self.buffer.push(b';');
                }
                self.buffer.extend_from_slice(p);
            }
            if bell_terminated {
                self.buffer.push(0x07);
            } else {
                self.buffer.push(0x1b);
                self.buffer.push(b'\\');
            }
        }
    }
    fn csi_dispatch(&mut self, params: &vte::Params, intermediates: &[u8], _ignore: bool, action: char) {
        // Detect Clear Screen (CSI 2 J or CSI 3 J)
        if action == 'J' {
            for p in params.iter() {
                for &b in p.iter() {
                    if b == 2 || b == 3 {
                        self.flush();
                        let _ = self.sender.send(PtyEvent::ClearScreen);
                    }
                }
            }
        }

        // Detect Alternate Screen Buffer mode (CSI ? 1049 h / 47 h / 1047 h or l)
        if (action == 'h' || action == 'l') && intermediates == [b'?'] {
            for p in params.iter() {
                for &b in p.iter() {
                    if b == 1049 || b == 47 || b == 1047 {
                        self.flush();
                        if action == 'h' {
                            let _ = self.sender.send(PtyEvent::EnterAltScreen);
                        } else {
                            let _ = self.sender.send(PtyEvent::LeaveAltScreen);
                        }
                    }
                }
            }
        }
        
        self.buffer.push(0x1b);
        self.buffer.push(b'[');
        for i in intermediates {
            self.buffer.push(*i);
        }
        for (i, p) in params.iter().enumerate() {
            if i > 0 {
                self.buffer.push(b';');
            }
            for b in p.iter() {
                if *b != 0 {
                    let s = format!("{}", b);
                    self.buffer.extend_from_slice(s.as_bytes());
                }
            }
        }
        self.buffer.push(action as u8);
    }
    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.buffer.push(0x1b);
        for i in intermediates {
            self.buffer.push(*i);
        }
        self.buffer.push(byte);
    }
}

impl PtySession {
    pub fn new(ctx: eframe::egui::Context) -> Result<Self, anyhow::Error> {
        let shell = if cfg!(target_os = "windows") {
            "powershell.exe".to_string()
        } else if let Ok(s) = std::env::var("SHELL") {
            if !s.is_empty() && std::path::Path::new(&s).exists() {
                s
            } else if std::path::Path::new("/bin/bash").exists() {
                "/bin/bash".to_string()
            } else {
                "/bin/sh".to_string()
            }
        } else if std::path::Path::new("/bin/bash").exists() {
            "/bin/bash".to_string()
        } else if std::path::Path::new("/bin/zsh").exists() {
            "/bin/zsh".to_string()
        } else {
            "/bin/sh".to_string()
        };
        Self::new_with_shell(&shell, ctx)
    }

    pub fn new_with_shell(shell_prog: &str, ctx: eframe::egui::Context) -> Result<Self, anyhow::Error> {
        let pty_system = NativePtySystem::default();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let is_zsh = shell_prog.ends_with("zsh");
        let is_bash = shell_prog.ends_with("bash");

        let mut cmd = CommandBuilder::new(shell_prog);
        let mut temp_dir = None;

        if is_zsh {
            cmd.arg("-i");
            if let Ok(td) = tempfile::tempdir() {
                let zshrc_path = td.path().join(".zshrc");
                let hook_script = r#"
typeset -g POWERLEVEL9K_INSTANT_PROMPT=off
if [ -f "$HOME/.zshrc" ]; then
    source "$HOME/.zshrc"
fi
if [ -t 1 ]; then
    stty -tabs 2>/dev/null || stty oxtabs 2>/dev/null
fi
export PROMPT=""
export RPROMPT=""
export PS1=""

smart_term_preexec() {
    print -Pn "\e]1337;SmartTermCmdStart\a"
}
smart_term_precmd() {
    local exit_code=$?
    PROMPT=""
    RPROMPT=""
    PS1=""
    print -Pn "\e]1337;SmartTermCmdEnd;$exit_code\a"
}
autoload -Uz add-zsh-hook
add-zsh-hook preexec smart_term_preexec
add-zsh-hook precmd smart_term_precmd
"#;
                if std::fs::write(&zshrc_path, hook_script).is_ok() {
                    cmd.env("ZDOTDIR", td.path());
                    temp_dir = Some(td);
                }
            }
        } else if is_bash {
            cmd.arg("-i");
            if let Ok(td) = tempfile::tempdir() {
                let bashrc_path = td.path().join(".bashrc");
                let hook_script = r#"
if [ -f "$HOME/.bashrc" ]; then
    source "$HOME/.bashrc"
fi
export PS1=""
PROMPT_COMMAND='echo -ne "\033]1337;SmartTermCmdEnd;$?\007"'
"#;
                if std::fs::write(&bashrc_path, hook_script).is_ok() {
                    cmd.arg("--rcfile");
                    cmd.arg(bashrc_path.to_str().unwrap_or(".bashrc"));
                    temp_dir = Some(td);
                }
            }
        } else if !cfg!(target_os = "windows") {
            cmd.arg("-i");
        }

        cmd.env("TERM", "xterm-256color");
        cmd.env("LANG", "en_US.UTF-8");

        let mut child = pair.slave.spawn_command(cmd)?;
        let process_id = child.process_id();

        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let master = pair.master;

        let (sender, receiver) = std::sync::mpsc::channel();
        
        thread::spawn(move || {
            let mut vte_parser = vte::Parser::new();
            let mut interceptor = OscInterceptor {
                sender: sender.clone(),
                buffer: Vec::new(),
            };
            
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        vte_parser.advance(&mut interceptor, &buf[..n]);
                        interceptor.flush();
                        ctx.request_repaint();
                    }
                    Err(_) => break,
                }
            }
            let _ = child.wait();
        });

        Ok(Self {
            process_id,
            receiver,
            master,
            writer,
            last_size: (24, 80),
            _temp_dir: temp_dir,
        })
    }

    pub fn write(&mut self, input: &[u8]) {
        let _ = self.writer.write_all(input);
        let _ = self.writer.flush();
    }

    /// Send Ctrl+C (ETX byte 0x03) to interrupt the currently running command.
    pub fn send_interrupt(&mut self) {
        self.write(&[0x03]);
    }

    /// Forcefully kill the shell process (and its children) with SIGKILL.
    /// Used when closing a terminal window while a command is still running.
    pub fn kill_process(&self) {
        #[cfg(unix)]
        if let Some(pid) = self.process_id {
            if pid > 0 {
                // Kill the entire process group so child processes are also terminated.
                unsafe {
                    libc::killpg(pid as libc::pid_t, libc::SIGKILL);
                    // Fallback: kill the process directly if it's not a group leader.
                    libc::kill(pid as libc::pid_t, libc::SIGKILL);
                }
            }
        }
        #[cfg(windows)]
        if let Some(pid) = self.process_id {
            let _ = std::process::Command::new("taskkill")
                .args(["/F", "/T", "/PID", &pid.to_string()])
                .output();
        }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if rows == 0 || cols == 0 || (rows, cols) == self.last_size {
            return;
        }
        self.last_size = (rows, cols);

        let _ = self.master.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        });
    }
}
