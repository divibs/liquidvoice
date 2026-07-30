#[cfg(windows)]
pub fn type_text(text: &str) -> Result<(), String> {
    use windows::Win32::Foundation::WPARAM;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_UNICODE, KEYEVENTF_KEYUP,
    };

    const CHUNK_SIZE: usize = 64;

    for chunk in text.encode_utf16().collect::<Vec<u16>>().chunks(CHUNK_SIZE) {
        let mut inputs: Vec<INPUT> = Vec::with_capacity(chunk.len() * 2);

        for &code in chunk {
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: Default::default(),
                        wScan: code,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: WPARAM(0),
                    },
                },
            });
            inputs.push(INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: Default::default(),
                        wScan: code,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: WPARAM(0),
                    },
                },
            });
        }

        unsafe {
            SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
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
