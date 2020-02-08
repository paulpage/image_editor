use std::process::Command;
use std::str;

pub fn get_clipboard_image() -> Option<Vec<u8>> {
    if let Ok(out) = Command::new("xclip")
        .arg("-o")
        .arg("-selection")
        .arg("clipboard")
        .arg("-t")
        .arg("TARGETS")
        .output()
    {
        if let Ok(s) = str::from_utf8(&out.stdout) {
            for line in s.lines() {
                if line == "image/png" {
                    if let Ok(out) = Command::new("xclip")
                        .arg("-o")
                        .arg("-selection")
                        .arg("clipboard")
                        .arg("-t")
                        .arg("image/png")
                        .output()
                    {
                        return Some(out.stdout);
                    }
                }
            }
        }
    }
    None
}
