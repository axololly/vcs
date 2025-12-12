#![cfg(unix)]
#![allow(dead_code)]

use std::{
    io::{Error, Result},
    mem::MaybeUninit,
    sync::Mutex,
    time::Duration,
};

use crate::{println, types::Size};

const STDIN_BUFFER_SIZE_BYTES: usize = 4096;

struct State {
    original_termios: Option<libc::termios>,
    inject_resize: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    original_termios: None,
    inject_resize: true,
});

fn get_termios() -> Result<libc::termios> {
    unsafe {
        let mut termios = MaybeUninit::<libc::termios>::uninit();

        // Get the current attributes of the terminal
        let result = libc::tcgetattr(libc::STDIN_FILENO, termios.as_mut_ptr());

        if result == -1 {
            Err(Error::last_os_error())
        } else {
            Ok(termios.assume_init())
        }
    }
}

fn set_termios(term: &libc::termios) -> Result<()> {
    let result = unsafe { libc::tcsetattr(libc::STDIN_FILENO, libc::TCSAFLUSH, term) };

    if result == -1 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn enable_raw_mode() -> Result<()> {
    let mut term = get_termios()?;

    term.c_iflag &= !(
        // Treat Ctrl-S and Ctrl-Q literally.
        libc::IXON

        // Don't translate carriage returns into newlines.
      | libc::ICRNL
    );

    term.c_oflag &= !(
        // Don't precede printed newlines with carriage returns.
        libc::OPOST
    );

    term.c_lflag &= !(
        // Disable rendering typed characters.
        libc::ECHO

        // Turn off canonical (cooked) mode.
      | libc::ICANON

        // Treat Ctrl-C and Ctrl-Z literally.
      | libc::ISIG

        // Treat Ctrl-V literally.
      | libc::IEXTEN
    );

    set_termios(&term)?;

    Ok(())
}

pub fn disable_raw_mode() -> Result<()> {
    if let Some(orig) = STATE.lock().unwrap().original_termios {
        set_termios(&orig)?;
    }

    Ok(())
}

fn check_int_return(code: i32) -> Result<()> {
    if code == -1 {
        Err(Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn terminal_size() -> Result<Size> {
    let mut window_size = MaybeUninit::<libc::winsize>::uninit();

    unsafe {
        check_int_return(libc::ioctl(
            libc::STDOUT_FILENO,
            libc::TIOCGWINSZ,
            window_size.as_mut_ptr(),
        ))?
    }

    let ws = unsafe { window_size.assume_init() };

    Ok(Size {
        width: ws.ws_col,
        height: ws.ws_row,
    })
}

extern "C" fn listen_for_sigwinch(_: libc::c_int) {
    STATE.lock().unwrap().inject_resize = true;
}

/// Taken from `microsoft/edit`.
///
/// If enabled, `libc::read()` returns immediately, even if no content is sent. If
/// disabled, `libc::read()` will block until input is ready.
fn set_tty_nonblocking(blocking: bool) -> Result<()> {
    unsafe {
        let mut flags = libc::fcntl(libc::STDIN_FILENO, libc::F_GETFL, 0);

        if flags == -1 {
            return Err(Error::last_os_error());
        }

        let is_nonblock = flags & libc::O_NONBLOCK == libc::O_NONBLOCK;

        if is_nonblock != blocking {
            flags ^= libc::O_NONBLOCK;
        }

        check_int_return(libc::fcntl(libc::STDIN_FILENO, libc::F_SETFL, flags))?;
    }

    Ok(())
}

/// Write directly to stdout.
///
/// This does not perform any locking, buffering or flushing as `println!` would,
/// and instead directly calls [`libc::write`].
pub fn write_stdout(data: &[u8]) -> Result<()> {
    let mut written = 0;

    loop {
        let result = unsafe {
            libc::write(
                libc::STDOUT_FILENO,
                data[written..].as_ptr().cast(),
                data.len(),
            )
        };

        if result == -1 {
            let e = Error::last_os_error();

            // Failed so try again.
            if let Some(libc::EINTR) = e.raw_os_error() {
                continue;
            }

            return Err(e);
        }

        written += result as usize;

        if written == data.len() {
            break;
        }
    }

    Ok(())
}

// #[deprecated]
fn bytes_left_in_stdin() -> Result<usize> {
    let mut bytes_left: i32 = 0;

    loop {
        let result = unsafe {
            libc::ioctl(
                libc::STDIN_FILENO,
                libc::FIONREAD,
                &mut bytes_left as *mut libc::c_int,
            )
        };

        if result == -1 {
            let e = Error::last_os_error();

            // Failed so try again.
            // Not sure if this is valid or not - will still leave it.
            if let Some(libc::EINTR) = e.raw_os_error() {
                continue;
            }

            break Err(e);
        }

        println!("\x1b[38;5;210mBytes left: {bytes_left}\x1b[0m\r");

        break Ok(bytes_left as usize);
    }
}

/// Wait for stdin for `timeout` and return.
///
/// The `io::Result<T>` comes from terminal I/O operations,
/// and the `Option<Vec<u8>>` is either `Some(Vec<u8>)` with input,
/// or `None` if timed out.
///
/// This can be called manually for basic non-blocking reads from stdin, but
/// is best left alone in favour of the [`manyterm::event::read`] function.
pub fn read_stdin(timeout: Duration) -> Result<Option<Vec<u8>>> {
    let mut state = STATE.lock().unwrap();

    if state.inject_resize {
        state.inject_resize = false;

        let size = terminal_size()?;

        return Ok(Some(
            format!("\x1b[8;{};{}t", size.height, size.width)
                .as_bytes()
                .to_vec(),
        ));
    }

    let mut buf: Vec<u8> = Vec::with_capacity(STDIN_BUFFER_SIZE_BYTES);

    set_tty_nonblocking(true)?;

    let mut poll_fd = libc::pollfd {
        fd: libc::STDIN_FILENO,
        events: libc::POLLIN,
        revents: 0,
    };

    let result = unsafe { libc::poll(&mut poll_fd, 1, timeout.as_millis() as i32) };

    if result < 0 {
        let e = Error::last_os_error();

        if let Some(libc::EINTR) = e.raw_os_error() {
            return Ok(None);
        }

        return Err(e);
    }

    // Timed out or polling failed somehow
    if poll_fd.revents & libc::POLLIN != libc::POLLIN {
        return Ok(None);
    }

    loop {
        buf.reserve(STDIN_BUFFER_SIZE_BYTES);

        let buf_to_write_to = buf.spare_capacity_mut();

        let bytes_read = unsafe {
            libc::read(
                libc::STDIN_FILENO,
                buf_to_write_to.as_mut_ptr() as _,
                STDIN_BUFFER_SIZE_BYTES,
            )
        };

        if bytes_read < 0 {
            let e = Error::last_os_error();

            match e.raw_os_error().unwrap() {
                // This is most likely to arise from SIGWINCH firing,
                // so we can ignore it and process the current input,
                // then on the next `read()` call, we'll have the new
                // terminal size.
                libc::EINTR => continue,

                // We requested non-blocking I/O, but this means that what we
                // requested would have been blocking, ie. waiting for new input.
                // We can only wait for new input if we've finished processing all
                // available input so far, thus we have an EOF metric! Huzzah!
                libc::EAGAIN => break,

                _ => return Err(e),
            }
        }

        if bytes_read == 0 {
            break;
        }

        unsafe {
            buf.set_len(buf.len() + bytes_read as usize);
        }
    }

    set_tty_nonblocking(false)?;

    buf.shrink_to_fit();

    Ok(Some(buf))
}

/// Setup the library.
///
/// For Unix, this attaches a listener to `SIGWINCH` and stores the original state of the terminal.
pub fn setup() -> Result<()> {
    unsafe {
        STATE.lock().unwrap().original_termios = Some(get_termios()?);

        let mut action: libc::sigaction = std::mem::zeroed();

        action.sa_sigaction = listen_for_sigwinch as usize;

        check_int_return(libc::sigaction(
            libc::SIGWINCH,
            &action,
            std::ptr::null_mut(),
        ))?;
    }

    Ok(())
}
