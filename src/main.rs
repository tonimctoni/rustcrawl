extern crate rand;
extern crate url;
extern crate reqwest;
extern crate regex;
// use std::thread;
use std::sync;
mod murmur;
mod unique;
mod crawl_worker;
mod url_reservoir;




fn main() {
    let (css_sender, _) = sync::mpsc::channel::<String>();
    let unique=sync::Arc::new(sync::Mutex::new(unique::Unique::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()])));

    crawl_worker::worker(css_sender.clone(), unique.clone(), url_reservoir.clone());
}

// 20 40 80 120 200
// https://github.com/tonimctoni/rustcrawl/blob/master/src/unique.rs
// https://docs.rs/mime/0.3.5/mime/struct.Mime.html