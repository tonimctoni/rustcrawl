extern crate rand;
extern crate url;
extern crate reqwest;
extern crate regex;
use std::thread;
use std::sync;
mod murmur;
mod unique;
mod crawl_worker;
mod url_reservoir;


const NUM_THREADS: usize = 4;

fn main() {
    let (css_sender, css_receiver) = sync::mpsc::channel::<String>();
    let unique=sync::Arc::new(sync::Mutex::new(unique::Unique::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()])));

    for _ in 0..NUM_THREADS{
        let css_sender=css_sender.clone();
        let unique=unique.clone();
        let url_reservoir=url_reservoir.clone();
        let _ = thread::spawn(move || {
            crawl_worker::worker(css_sender, unique, url_reservoir);
        });
    }

    for css_content in css_receiver.iter(){
        println!("{:?}", css_content.len());
    }

    println!("Crawler exited (somehow).");
}

// 20 40 80 120 200
// https://github.com/tonimctoni/rustcrawl/blob/master/src/unique.rs
// https://docs.rs/mime/0.3.5/mime/struct.Mime.html