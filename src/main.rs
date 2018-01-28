extern crate rand;
extern crate url;
extern crate reqwest;
extern crate regex;
use std::thread;
use std::sync;
use std::fs;
use std::io::Write;
mod murmur;
mod unique;
mod crawl_worker;
mod url_reservoir;


const NUM_THREADS: usize = 100;
const ALLOWED_CHARS: &str = "abcdefghijklmnopqrstuvwxzy0123456789\n\t\r \"'(){}[]+-*/.,:;_@#%$!?=\\<>~^|&`";

fn contains_only_allowed_chars(s: &String) -> bool{
    s
    .chars()
    .all(|c| ALLOWED_CHARS.chars().any(|ca| ca==c))
}

fn main() {
    let (css_sender, css_receiver) = sync::mpsc::channel::<String>();
    let unique=sync::Arc::new(sync::Mutex::new(unique::LargeUnique::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()]))); // , "http://www.rust-lang.org".to_string(), "http://github.com".to_string(), "http://wikipedia.com".to_string()
    let urls_crawled=sync::Arc::new(sync::atomic::AtomicUsize::new(0));

    for _ in 0..NUM_THREADS{
        let css_sender=css_sender.clone();
        let unique=unique.clone();
        let url_reservoir=url_reservoir.clone();
        let urls_crawled=urls_crawled.clone();
        let _ = thread::spawn(move || {
            crawl_worker::worker(css_sender, unique, url_reservoir, urls_crawled);
        });
    }

    let mut unique=unique::LargeUnique::new(vec![0x5a14a940, 0xa87239b4]);
    let re_comments=regex::Regex::new(r"/\*(.|\n)*?\*/").unwrap();
    let re_breaklines=regex::Regex::new(r"\n{3,}").unwrap();
    let newline_char=std::char::from_u32(10).unwrap();

    let mut css_found:usize=0;
    let mut css_written:usize=0;
    for css_content in css_receiver.iter(){
        let mut css_content=css_content;
        css_found+=1;

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

        if unique.contains_add(css_content.as_bytes()){
            println!("Error: {:?}", "css was already gathered");
            continue;
        }

        let mut f=match fs::File::create(format!("css/css{:06}.css", css_written)) {
            Ok(f) => f,
            Err(e) => {println!("Error: {:?}", e);continue;},
        };

        match f.write_all(css_content.as_bytes()) {
            Ok(_) => (),
            Err(e) => {println!("Error: {:?}", e);continue;},
        }

        css_written+=1;
        let reservoir_available_space={
            let mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error: {:?}", e);break;},
            };
            mutex_guard.available_space()
        };

        let mut f=match fs::OpenOptions::new().append(true).create(true).open("csslog.txt") {
            Ok(f) => f,
            Err(e) => {println!("Error: {:?}", e);continue;},
        };

        match f.write_all(format!("[report] Urls urls_crawled: {:?}, Css found: {:?}, Css written: {:?}, Reservoir available space: {:?}\n", urls_crawled.load(sync::atomic::Ordering::Relaxed), css_found, css_written, reservoir_available_space).as_bytes()) {
            Ok(_) => (),
            Err(e) => {println!("Error: {:?}", e);continue;},
        }
        println!("[report] Urls urls_crawled: {:?}, Css found: {:?}, Css written: {:?}, Reservoir available space: {:?}", urls_crawled.load(sync::atomic::Ordering::Relaxed), css_found, css_written, reservoir_available_space);
    }

    println!("Crawler exited (somehow).");
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