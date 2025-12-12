#![cfg(windows)]

// #![allow(dead_code, unused_imports)]

use std::{
    ffi::c_void,
    io::{Error, Result},
    sync::Mutex,
    time::Duration
};

use windows_sys::{w, Win32::{
    Foundation::{
        HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT
    },
    System::{
        Console::{
            GetConsoleMode,
            GetConsoleScreenBufferInfo,
            GetNumberOfConsoleInputEvents,
            GetStdHandle, SetConsoleMode,
            WriteConsoleA,
            CONSOLE_SCREEN_BUFFER_INFO,
            ENABLE_ECHO_INPUT,
            ENABLE_LINE_INPUT,
            ENABLE_PROCESSED_INPUT,
            ENABLE_PROCESSED_OUTPUT,
            ENABLE_VIRTUAL_TERMINAL_INPUT,
            ENABLE_VIRTUAL_TERMINAL_PROCESSING,
            ENABLE_WRAP_AT_EOL_OUTPUT,
            FOCUS_EVENT,
            INPUT_RECORD,
            KEY_EVENT,
            STD_INPUT_HANDLE,
            STD_OUTPUT_HANDLE
        },
        LibraryLoader::{
            GetModuleHandleW,
            GetProcAddress
        },
        Threading::WaitForSingleObject
    }
}};

use crate::types::Size;

const NULL: *const c_void = std::ptr::null() as _;

type ReadConsoleInputExA = fn(
    h_console_input: HANDLE,
    lp_buffer: *mut INPUT_RECORD,
    n_length: u32,
    lp_number_of_events_read: *mut u32,
    w_flags: u16,
) -> i32;

const CONSOLE_READ_NOWAIT: u16 = 0x002;

pub struct State {
    default_stdin_flags: Option<u32>,
    default_stdout_flags: Option<u32>,
    inject_resize: bool,
    read_console_input: Option<ReadConsoleInputExA>
}

static STATE: Mutex<State> = Mutex::new(State {
    default_stdin_flags: None,
    default_stdout_flags: None,
    inject_resize: true,
    read_console_input: None
});

fn get_stdin() -> Result<HANDLE> {
    let stdin = unsafe { GetStdHandle(STD_INPUT_HANDLE) };

    if std::ptr::eq(stdin, NULL) {
        return Err(Error::last_os_error());
    }

    Ok(stdin)
}

fn get_stdout() -> Result<HANDLE> {
    let stdout = unsafe { GetStdHandle(STD_OUTPUT_HANDLE) };

    if std::ptr::eq(stdout, NULL) {
        return Err(Error::last_os_error());
    }

    Ok(stdout)
}

fn check_int_return(code: i32) -> Result<()> {
    if code != 0 {
        Ok(())
    }
    else {
        Err(Error::last_os_error())
    }
}

fn get_read_function() -> Result<ReadConsoleInputExA> {
    unsafe {
        let module_handle = GetModuleHandleW(w!("kernel32.dll"));

        if std::ptr::eq(module_handle, NULL) {
            return Err(Error::last_os_error());
        }

        let proc = GetProcAddress(module_handle, c"ReadConsoleInputExA".as_ptr() as _);

        if let Some(f) = proc {
            Ok(std::mem::transmute::<unsafe extern "system" fn() -> isize, ReadConsoleInputExA>(f))
        }
        else {
            Err(Error::last_os_error())
        }
    }
}

pub fn setup() -> Result<()> {
    let mut state = STATE.lock().unwrap();
    
    unsafe {
        let stdin = get_stdin()?;

        let mut stdin_flags: u32 = 0;

        check_int_return(
            GetConsoleMode(stdin, &mut stdin_flags as _)
        )?;

        state.default_stdin_flags = Some(stdin_flags);

        let stdout = get_stdout()?;

        let mut stdout_flags: u32 = 0;

        check_int_return(
            GetConsoleMode(stdout, &mut stdout_flags as _)
        )?;

        state.default_stdout_flags = Some(stdout_flags);

        state.read_console_input = Some(get_read_function()?);

        Ok(())
    }
}

pub fn enable_raw_mode() -> Result<()> {
    unsafe {
        let stdin = get_stdin()?;

        // ENABLE_WINDOW_INPUT might be helpful here, or something similar
        let mut stdin_flags = 0;
        
        check_int_return(
            GetConsoleMode(stdin, &mut stdin_flags)
        )?;

        // Send VT sequences to stdin
        stdin_flags |= ENABLE_VIRTUAL_TERMINAL_INPUT;
        stdin_flags &= !(
            // Don't render typed characters.
            ENABLE_ECHO_INPUT
            // Return as soon as a character is typed.
          | ENABLE_LINE_INPUT
            // Treat Ctrl+C and other control characters literally
          | ENABLE_PROCESSED_INPUT
        );

        check_int_return(
            SetConsoleMode(stdin, stdin_flags)
        )?;

        let stdout = get_stdout()?;

        let mut stdout_flags = 0;

        check_int_return(
            GetConsoleMode(stdout, &mut stdout_flags)
        )?;

        // Use VT sequences in stdout
        stdout_flags |= ENABLE_VIRTUAL_TERMINAL_PROCESSING;
        
        // Disable line wrap
        stdout_flags |= ENABLE_PROCESSED_OUTPUT;
        
        // Use certain ASCII control characters in stdout
        stdout_flags &= !ENABLE_WRAP_AT_EOL_OUTPUT;

        check_int_return(
            SetConsoleMode(stdout, stdout_flags)
        )?;

        Ok(())
    }
}

pub fn disable_raw_mode() -> Result<()> {
    let state = STATE.lock().unwrap();
    
    unsafe {
        if let Some(flags) = state.default_stdin_flags {
            check_int_return(
                SetConsoleMode(get_stdin()?, flags)
            )?;
        }
        else {
            return Err(Error::other("Could not retrieve initial stdin console mode. Was sys initialised?"));
        }

        if let Some(flags) = state.default_stdout_flags {
            check_int_return(
                SetConsoleMode(get_stdout()?, flags)
            )?;
        }
        else {
            return Err(Error::other("Could not retrieve initial stdout console mode. Was sys initialised?"));
        }

        Ok(())
    }
}

pub fn terminal_size() -> Result<Size> {
    unsafe {
        let mut info: CONSOLE_SCREEN_BUFFER_INFO = std::mem::zeroed();

        check_int_return(
            GetConsoleScreenBufferInfo(
                GetStdHandle(STD_OUTPUT_HANDLE) as _,
                &mut info as _
            )
        )?;

        let w = info.srWindow;

        Ok(Size {
            width: (w.Right - w.Left + 1).max(0) as u16,
            height: (w.Top - w.Bottom + 1).max(0) as u16
        })
    }
}

pub fn read_stdin(timeout: Duration) -> Result<Option<Vec<u8>>> {
    let mut state = STATE.lock().unwrap();

    if state.inject_resize {
        state.inject_resize = false;

        let size = terminal_size()?;

        return Ok(Some(format!("\x1b[8;{};{}t", size.height, size.width).into_bytes()));
    }

    let stdin = get_stdin()?;

    let wait_event = unsafe {
        WaitForSingleObject(stdin, timeout.as_millis() as u32)
    };

    match wait_event {
        // Ready to read
        WAIT_OBJECT_0 => {},
        
        // Polling timed out
        WAIT_TIMEOUT => return Ok(None),

        // Something went wrong.
        _ => return Err(Error::last_os_error())
    }

    let mut inputs: Vec<INPUT_RECORD> = Vec::with_capacity(4096);
    let mut inputs_read: u32 = 0;

    let mut buf: Vec<u8> = Vec::with_capacity(4096);

    unsafe {
        loop {
            let mut events_left: u32 = 0;

            GetNumberOfConsoleInputEvents(stdin, &mut events_left as _);

            if events_left == 0 {
                break;
            }

            inputs.reserve(events_left as usize);
            
            check_int_return(
                (state.read_console_input.unwrap())(
                    stdin,
                    inputs.as_mut_ptr(),
                    inputs.capacity() as u32,
                    &mut inputs_read,
                    CONSOLE_READ_NOWAIT
                )
            )?;

            inputs.set_len(inputs_read as usize);

            for input in &inputs[.. inputs_read as usize] {
                match input.EventType as u32 {
                    FOCUS_EVENT => {
                        let c = if input.Event.FocusEvent.bSetFocus == 1 { 'I' } else { 'O' };

                        buf.extend(format!("\x1b[{c}").as_bytes());
                    }

                    // Essentially just generic text inputs
                    KEY_EVENT => {
                        let event = input.Event.KeyEvent;

                        if event.bKeyDown != 1 {
                            continue;
                        }

                        buf.push(event.uChar.AsciiChar as u8);
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(Some(buf))
}

pub fn write_stdout(data: &[u8]) -> Result<()> {
    let stdout = get_stdout()?;
    
    let mut bytes_so_far: usize = 0;
    let mut written_now: u32 = 0;

    loop {
        if bytes_so_far == data.len() {
            break;
        }

        check_int_return(
            unsafe {
                WriteConsoleA(
                    stdout,
                    data.as_ptr() as _,
                    data.len() as u32,
                    &mut written_now as _,
                    NULL
                )
            }
        )?;

        bytes_so_far += written_now as usize;
    }
    
    Ok(())
}