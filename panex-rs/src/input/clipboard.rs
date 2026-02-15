use std::io::Write;

/// Copy text to system clipboard, with OSC 52 fallback for remote sessions
pub fn copy_to_clipboard(text: &str) -> bool {
    // Try system clipboard first
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if clipboard.set_text(text).is_ok() {
            return true;
        }
    }

    // Fallback: OSC 52 escape sequence (works over SSH)
    osc52_copy(text)
}

fn osc52_copy(text: &str) -> bool {
    let encoded = base64_encode(text.as_bytes());
    let seq = format!("\x1b]52;c;{}\x07", encoded);
    let mut stdout = std::io::stdout();
    stdout.write_all(seq.as_bytes()).is_ok() && stdout.flush().is_ok()
}

/// Minimal base64 encode (avoids extra dependency)
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((data.len() + 2) / 3 * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            result.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        if chunk.len() > 2 {
            result.push(CHARS[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    result
}
