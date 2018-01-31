#![allow(dead_code)]
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



pub fn css_worker(css_receiver: sync::mpsc::Receiver<String>, css_written: sync::Arc<sync::atomic::AtomicUsize>){
    let mut bloom_filter=bloom_filter::LargeBloomFilter::new(vec![0x41be6a18, 0xb8261088]);
    let re_comments=regex::Regex::new(r"/\*(.|\n)*?\*/").unwrap();
    let re_breaklines=regex::Regex::new(r"\n{3,}").unwrap();
    let newline_char=char::from_u32(10).unwrap();

    let mut c=0;
    for mut css_content in css_receiver.iter(){
        css_content=css_content.to_lowercase();
        if !contains_only_allowed_chars(&css_content){
            continue;
        }

        css_content=String::from(re_comments.replace_all(css_content.as_str(), ""));
        css_content=String::from(re_breaklines.replace_all(css_content.as_str(), "\n\n"));
        css_content=css_content.trim().to_string();
        if css_content.len()<=50{
            continue;
        }

        if css_content.chars().filter(|&c| c==newline_char).count()<5{
            continue;
        }

        if bloom_filter.contains_add(css_content.as_bytes()){
            println!("Error: {:?}", "css was already gathered");
            continue;
        }

        c+=1;
        let mut f=match fs::File::create(format!("css/css{:06}.css", c)) {
            Ok(f) => f,
            Err(e) => {println!("Error: {:?}", e);continue;},
        };

        match f.write_all(css_content.as_bytes()) {
            Ok(_) => (),
            Err(e) => {println!("Error: {:?}", e);continue;},
        }

        css_written.fetch_add(1, sync::atomic::Ordering::Relaxed);
    }

    println!("Css worker terminated.");
}