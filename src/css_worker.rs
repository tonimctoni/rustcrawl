use bloom_filter;
use regex;
use std::sync;
use std::fs;
use std::io::Write;
use std::char;

const ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxzy0123456789\n\t\r \"'(){}[]+-*/.,:;_@#%$!?=\\<>~^|&`";


/// Checks wether the input string `s` contains only allowed characters.
///
/// # Arguments
///
/// * `s` - String to be checked.
///
fn contains_only_allowed_chars(s: &String) -> bool{
    s
    .chars()
    .all(|c| ALLOWED_CHARS.chars().any(|ca| ca==c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_only_allowed_chars() {
        assert!(contains_only_allowed_chars(&"hello".to_string()));
        assert!(contains_only_allowed_chars(&"hello123".to_string()));
        assert!(contains_only_allowed_chars(&"hello world !".to_string()));
        assert!(contains_only_allowed_chars(&"fn a() -> bool {return true;}".to_string()));
        assert!(contains_only_allowed_chars(&"0123456789,.-;:_[]@#! ?\"\n\t\r".to_string()));

        assert!(!contains_only_allowed_chars(&"Hello".to_string()));
        assert!(!contains_only_allowed_chars(&"helloª".to_string()));
        assert!(!contains_only_allowed_chars(&"hello¨".to_string()));
        assert!(!contains_only_allowed_chars(&"helloÇ".to_string()));
        assert!(!contains_only_allowed_chars(&"ASD".to_string()));
    }
}


/// Within an endless loop, it obtains css code via the channel `css_receiver` and
/// if it fulfills certain parameters (in an attempt to get nice looking ones only)
/// and has not been obtained before, the css code gets saved into a file.
///
/// # Arguments
///
/// * `css_receiver` - Channel receiver that receives css code.
/// * `css_written` - Atomic counter that counts the amount of css files saved.
pub fn css_worker(css_receiver: sync::mpsc::Receiver<Vec<u8>>, css_written: sync::Arc<sync::atomic::AtomicUsize>){
    let mut bloom_filter=bloom_filter::LargeBloomFilter::new(vec![0x41be6a18, 0xb8261088]);
    let re_comments=regex::Regex::new(r"/\*(.|\n)*?\*/").unwrap();
    let re_breaklines=regex::Regex::new(r"\n{3,}").unwrap();
    let newline_char=char::from_u32(10).unwrap();

    let mut c=0;
    // For every css code received.
    for css_content in css_receiver.iter(){
        // Make sure it contains valide utf8 only and turn into a String.
        let mut css_content=match String::from_utf8(css_content) {
            Ok(css_content) => css_content,
            Err(e) => {eprintln!("Error (css_worker): {:?}", e.utf8_error());continue;},
        };

        // Transform to lower case.
        css_content=css_content.to_lowercase();
        if !contains_only_allowed_chars(&css_content){
            eprintln!("Error (css_worker): {:?}", "css contains disallowed chars");
            continue;
        }

        // Remove all comments, parts where there are too many newlines next to each other, and trim (strip in python).
        css_content=String::from(re_comments.replace_all(css_content.as_str(), ""));
        css_content=String::from(re_breaklines.replace_all(css_content.as_str(), "\n\n"));
        css_content=css_content.trim().to_string();
        // If the code is too small, discard and continue.
        if css_content.len()<=50{
            eprintln!("Error (css_worker): {:?}", "css len less than 50");
            continue;
        }

        // If code has too few newlines, discard and continue.
        if css_content.chars().filter(|&c| c==newline_char).count()<5{
            eprintln!("Error (css_worker): {:?}", "css has fewer than 5 newline chars");
            continue;
        }

        // If code was saved into a file before, discard and continue.
        if bloom_filter.contains_add(css_content.as_bytes()){
            eprintln!("Error (css_worker): {:?}", "css was already gathered");
            continue;
        }

        // Open file to save code to.
        c+=1;
        let mut f=match fs::File::create(format!("css/css{:06}.css", c)) {
            Ok(f) => f,
            Err(e) => {eprintln!("Error (css_worker): {:?}", e);continue;},
        };

        // Save code to file.
        match f.write_all(css_content.as_bytes()) {
            Ok(_) => (),
            Err(e) => {eprintln!("Error (css_worker): {:?}", e);continue;},
        }

        // Keep track of number css files created with atomic counter `css_written`.
        css_written.fetch_add(1, sync::atomic::Ordering::Relaxed);
    }

    eprintln!("Css worker terminated.");
}