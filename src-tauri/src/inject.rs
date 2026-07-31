#[cfg(windows)]
pub fn type_text(text: &str) -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    };

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
pub fn type_text(text: &str) -> Result<(), String> {
    eprintln!("[dev] Would type: {text}");
    Ok(())
}
