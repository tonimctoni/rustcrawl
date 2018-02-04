#![feature(box_syntax)]

extern crate rand;
extern crate url;
extern crate regex;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
use futures::Future;
use futures::stream::Stream;
use std::thread;
use std::sync;
use std::time;
use std::fs;
use std::io::Write;
mod murmur;
mod bloom_filter;
mod url_reservoir;
mod css_worker;
mod html_worker;
mod url_enqueuer;

const CHANNEL_BUFFER_SIZE: usize = 1024;
const FUTURE_STREAM_BUFFER_SIZE: usize = 100;
const SLEEP_MILLIS_BETWEEN_REPORTS: u64 = 60000;
const GET_TIMEOUT_MILLIS: u64 = 4000;
const REPORT_FILENAME: &str = "report.txt";

// Declare enum needed to distinguish between contenttypes of gotten urls.
#[derive(PartialEq, Copy, Clone)]
enum ContentType {
    Html,
    Css,
    Other,
}

fn get_timeout(handle: &tokio_core::reactor::Handle) -> tokio_core::reactor::Timeout {
    loop {
        match tokio_core::reactor::Timeout::new(time::Duration::from_millis(GET_TIMEOUT_MILLIS), &handle) {
            Ok(timeout) => {
                return timeout
            },
            Err(e) => {
                eprintln!("Error (Timeout.new): {:?}", e);
                continue;
            },
        }
    }
}

fn main() {
    // Define channels for html and css code.
    let (css_sender, css_receiver) = sync::mpsc::channel::<Vec<u8>>();
    let (html_sender, html_receiver) = sync::mpsc::channel::<(String,Vec<u8>)>();

    // Define atomic variables to keep track of some stats.
    let css_written=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let htmls_crawled=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let urls_enqueued=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let urls_gotten=sync::Arc::new(sync::atomic::AtomicUsize::new(0));

    // Define a bloom filter and url reservoir to keep track of used urls and store them respectively.
    let bloom_filter=sync::Arc::new(sync::Mutex::new(bloom_filter::LargeBloomFilter::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()], rand::StdRng::new().unwrap())));

    // Define channel sink/stream pair for uris to be gotten by the hyper::client::Client.
    let (uri_sink, uri_stream)=futures::sync::mpsc::channel::<hyper::Uri>(CHANNEL_BUFFER_SIZE);

    // Run `css_worker` concurrently.
    {
        let css_written=css_written.clone();
        thread::spawn(move || {
            css_worker::css_worker(css_receiver, css_written);
        });
    }

    // Run `html_worker` concurrently.
    {
        let htmls_crawled=htmls_crawled.clone();
        let bloom_filter=bloom_filter.clone();
        let url_reservoir=url_reservoir.clone();
        thread::spawn(move || {
            html_worker::html_worker(html_receiver, htmls_crawled, bloom_filter, url_reservoir);
        });
    }

    // Run `url_enqueuer` concurrently.
    {
        let urls_enqueued=urls_enqueued.clone();
        let url_reservoir=url_reservoir.clone();
        thread::spawn(move || {
            url_enqueuer::url_enqueuer(uri_sink, urls_enqueued, bloom_filter, url_reservoir);
        });
    }

    // Run a reporter that logs data concurrently.
    {
        let urls_gotten=urls_gotten.clone();
        thread::spawn(move || {
            let mut last_gotten=0;
            let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_BETWEEN_REPORTS);
            for i in 0.. {
                thread::sleep(sleep_duration_per_iter);
                let mut f=match fs::OpenOptions::new().append(true).create(true).open(REPORT_FILENAME) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("Error (reporting): {:?}", e);
                        last_gotten=urls_gotten.load(sync::atomic::Ordering::Relaxed);
                        continue;
                    },
                };

                let reservoir_len={
                    let mutex_guard=match url_reservoir.lock() {
                        Ok(mutex_guard) => mutex_guard,
                        Err(e) => {eprintln!("Error (reporting): {:?}", e);break;},
                    };

                    mutex_guard.len()
                };

                let gotten=urls_gotten.load(sync::atomic::Ordering::Relaxed);
                match f.write_all(format!("[report ({})] urls enqueued: {}, urls gotten: {}, htmls crawled: {}, css written: {}, reservoir contains: {}, get requests per second: {:.2}\n",
                    i,
                    urls_enqueued.load(sync::atomic::Ordering::Relaxed),
                    gotten,
                    htmls_crawled.load(sync::atomic::Ordering::Relaxed),
                    css_written.load(sync::atomic::Ordering::Relaxed),
                    reservoir_len,
                    ((gotten-last_gotten) as f64)/((SLEEP_MILLIS_BETWEEN_REPORTS as f64) / 1000.0)
                    ).as_bytes()) {
                    Ok(_) => {},
                    Err(e) => eprintln!("Error (reporting): {:?}", e),
                }
                last_gotten=gotten;
            }
            eprintln!("Reporter terminated.");
        });
    }

    // Define tokio Core and client to be used in/as IO loop.
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::new(&handle);

    // Prepare work for the core.
    let work=uri_stream
    .map(|uri|{
        let c=urls_gotten.fetch_add(1, sync::atomic::Ordering::Relaxed);
        println!("{}, {}", c, uri.host().unwrap_or(""));
        // println!("{}", uri);

        let timeout=get_timeout(&handle);
        let uri_string=uri.to_string();

        client.get(uri)
        .and_then(|res| {
            let content_type=match res.headers().get::<hyper::header::ContentType>(){
                Some(content_type) => {
                    let mimetype=(content_type.type_(), content_type.subtype());
                    if mimetype.0=="text" && mimetype.1=="html"{
                        ContentType::Html
                    } else if mimetype.0=="text" && mimetype.1=="css"{
                        ContentType::Css
                    } else{
                        ContentType::Other
                    }
                },
                None => ContentType::Other,
            };

            res.body().concat2().map(move |res| (res, content_type))
        })
        .select2(timeout)
        .then(|t| {
            match t {
                Ok(futures::future::Either::B((_, _))) => {eprintln!("Error (get timeout ok): {:?}", uri_string);Ok(())},
                Err(futures::future::Either::A((get_error, _))) => {eprintln!("Error (Client.get err): {:?}", get_error);Ok(())},
                Err(futures::future::Either::B((timeout_error, _))) => {eprintln!("Error (get timeout err): {:?}", timeout_error);Ok(())},
                Ok(futures::future::Either::A(((chunks, content_type), _))) => {
                    match content_type {
                        ContentType::Html => {
                            match html_sender.send((uri_string, chunks.to_vec())) {
                                Err(e) => eprintln!("Error (html_sender.send): {:?}", e),
                                _ => {},
                            }
                        },
                        ContentType::Css => {
                            match css_sender.send(chunks.to_vec()) {
                                Err(e) => eprintln!("Error (css_sender.send): {:?}", e),
                                _ => {},
                            }
                        },
                        ContentType::Other => {},
                    }
                    Ok(())
                },
            }
        })
    })
    .buffer_unordered(FUTURE_STREAM_BUFFER_SIZE)
    .for_each(|_| Ok(()));


    // Run work (operations on stream) in tokio core.
    match core.run(work) {
        Ok(o) => eprintln!("Ok (Core.run): {:?}", o),
        Err(e) => eprintln!("Error (Core.run): {:?}", e),
    }

    eprintln!("IO loop terminated.");
}
