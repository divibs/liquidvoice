/// Foreground window at the moment a take starts; used to refuse injection if
/// the user switches apps while transcription is in flight.
#[cfg(windows)]
pub fn foreground_window() -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let hwnd = unsafe { GetForegroundWindow() };
    (!hwnd.0.is_null()).then_some(hwnd.0 as isize)
}

#[cfg(not(windows))]
pub fn foreground_window() -> Option<isize> {
    None
}

#[cfg(windows)]
fn foreground_is(hwnd: isize) -> bool {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    unsafe { GetForegroundWindow().0 as isize == hwnd }
}

#[cfg(not(windows))]
#[allow(dead_code)]
fn foreground_is(_hwnd: isize) -> bool {
    true
}

#[cfg(windows)]
pub fn type_text(text: &str, target: Option<isize>) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };

    if let Some(hwnd) = target {
        if !foreground_is(hwnd) {
            return Err("Focus changed during transcription — text not typed".into());
        }
    }

    // Keep bursts small so target apps (especially slow ones) keep up.
    const CHUNK_SIZE: usize = 64;

    let codes: Vec<u16> = text.encode_utf16().collect();
    for chunk in codes.chunks(CHUNK_SIZE) {
        let mut inputs: Vec<INPUT> = Vec::with_capacity(chunk.len() * 2);

        for &code in chunk {
            let key = |flags| INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: Default::default(),
                        wScan: code,
                        dwFlags: flags,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            inputs.push(key(KEYEVENTF_UNICODE));
            inputs.push(key(KEYEVENTF_UNICODE | KEYEVENTF_KEYUP));
        }

        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err(format!(
                "SendInput injected {sent}/{} events (focus may be elevated or blocked)",
                inputs.len()
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }

    Ok(())
}

#[cfg(not(windows))]
pub fn type_text(text: &str, _target: Option<isize>) -> Result<(), String> {
    eprintln!("[dev] Would type: {text}");
    Ok(())
}
