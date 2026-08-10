use portable_pty::{CommandBuilder, MasterPty, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use tempfile::TempDir;

pub enum PtyEvent {
    CommandStart,
    CommandEnd { exit_code: i32 },
    Output(Vec<u8>),
    ClearScreen,
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
                    self.flush();
                    let _ = self.sender.send(PtyEvent::CommandStart);
                    handled = true;
                } else if params[1].starts_with(b"SmartTermCmdEnd;") {
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
        let pty_system = NativePtySystem::default();

        let pair = pty_system.openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })?;

        let mut cmd = CommandBuilder::new("zsh");
        
        let temp_dir = tempfile::tempdir()?;
        let zshrc_path = temp_dir.path().join(".zshrc");
        
        let hook_script = r#"
# Disable Powerlevel10k instant prompt to prevent initialization warnings and jumps
typeset -g POWERLEVEL9K_INSTANT_PROMPT=off

if [ -f "$HOME/.zshrc" ]; then
    source "$HOME/.zshrc"
fi

# Expand tabs to spaces so vt100 parser renders columns correctly
if [ -t 1 ]; then
    stty -tabs 2>/dev/null || stty oxtabs 2>/dev/null
fi

# Override PROMPT strictly so no theme prints it
export PROMPT=""
export RPROMPT=""
export PS1=""

smart_term_preexec() {
    print -Pn "\e]1337;SmartTermCmdStart\a"
}

smart_term_precmd() {
    local exit_code=$?
    # Ensure it's cleared again just in case a theme hook sets it during precmd
    PROMPT=""
    RPROMPT=""
    PS1=""
    print -Pn "\e]1337;SmartTermCmdEnd;$exit_code\a"
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec smart_term_preexec
add-zsh-hook precmd smart_term_precmd
"#;
        std::fs::write(&zshrc_path, hook_script)?;
        cmd.env("ZDOTDIR", temp_dir.path());
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
            _temp_dir: Some(temp_dir),
        })
    }

    pub fn write(&mut self, input: &[u8]) {
        let _ = self.writer.write_all(input);
        let _ = self.writer.flush();
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
