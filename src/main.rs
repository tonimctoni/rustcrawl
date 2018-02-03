// #![feature(drain_filter)]
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
// mod crawl_worker;
mod url_reservoir;
mod css_worker;
mod html_worker;
mod url_enqueuer;

const CHANNEL_BUFFER_SIZE: usize = 1024;
const SLEEP_MILLIS_BETWEEN_REPORTS: u64 = 6000;
const GET_TIMEOUT_MILLIS: u64 = 5000;
const REPORT_FILENAME: &str = "report.txt";

fn main() {
    let (css_sender, css_receiver) = sync::mpsc::channel::<Vec<u8>>();
    let (html_sender, html_receiver) = sync::mpsc::channel::<(String,Vec<u8>)>();
    let css_written=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let htmls_crawled=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let urls_enqueued=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let urls_gotten=sync::Arc::new(sync::atomic::AtomicUsize::new(0));

    let bloom_filter=sync::Arc::new(sync::Mutex::new(bloom_filter::LargeBloomFilter::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()], rand::StdRng::new().unwrap())));

    let (uri_sink, uri_stream)=futures::sync::mpsc::channel::<hyper::Uri>(CHANNEL_BUFFER_SIZE);

    // {
    //     let css_written=css_written.clone();
    //     thread::spawn(move || {
    //         css_worker::css_worker(css_receiver, css_written);
    //     });
    // }

    {
        let htmls_crawled=htmls_crawled.clone();
        let bloom_filter=bloom_filter.clone();
        let url_reservoir=url_reservoir.clone();
        thread::spawn(move || {
            html_worker::html_worker(html_receiver, htmls_crawled, bloom_filter, url_reservoir);
        });
    }

    {
        let urls_enqueued=urls_enqueued.clone();
        let url_reservoir=url_reservoir.clone();
        thread::spawn(move || {
            url_enqueuer::url_enqueuer(uri_sink, urls_enqueued, bloom_filter, url_reservoir);
        });
    }

    {
        let urls_gotten=htmls_crawled.clone();
        thread::spawn(move || {
            let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_BETWEEN_REPORTS);
            for i in 0.. {
                thread::sleep(sleep_duration_per_iter);
                let mut f=match fs::OpenOptions::new().append(true).create(true).open(REPORT_FILENAME) {
                    Ok(f) => f,
                    Err(e) => {println!("Error (reporting): {:?}", e);continue;},
                };

                let reservoir_len={
                    let mutex_guard=match url_reservoir.lock() {
                        Ok(mutex_guard) => mutex_guard,
                        Err(e) => {println!("Error (reporting): {:?}", e);break;},
                    };

                    mutex_guard.len()
                };

                match f.write_all(format!("[report ({})] urls enqueued: {:?}, urls gotten: {:?}, htmls crawled: {:?}, css written: {:?}, reservoir contains: {:?}\n",
                    i,
                    urls_enqueued.load(sync::atomic::Ordering::Relaxed),
                    urls_gotten.load(sync::atomic::Ordering::Relaxed),
                    htmls_crawled.load(sync::atomic::Ordering::Relaxed),
                    css_written.load(sync::atomic::Ordering::Relaxed),
                    reservoir_len
                    ).as_bytes()) {
                    Ok(_) => {},
                    Err(e) => println!("Error (reporting): {:?}", e),
                }
            }
            println!("Reporter terminated.");
        });
    }

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::new(&handle);

    #[derive(PartialEq)]
    enum ContentType {
        Html,
        Css,
        Other,
    }

    let work=uri_stream.and_then(|uri| {
        let uri_string=uri.to_string();
        let c=urls_gotten.fetch_add(1, sync::atomic::Ordering::Relaxed);
        println!("{}, {}", c, uri.host().unwrap_or(""));

        let timeout=(||{
                    loop {
                        match tokio_core::reactor::Timeout::new(time::Duration::from_millis(GET_TIMEOUT_MILLIS), &handle) {
                            Ok(timeout) => {
                                return timeout
                            },
                            Err(e) => {
                                println!("Error (Timeout.new): {:?}", e);
                                continue
                            },
                        }
                    }
                })();

        client.get(uri)
        .select2(timeout)
        .then(|res|{
            match res {
                Ok(futures::future::Either::A((got, _))) => Ok(got),
                Ok(futures::future::Either::B((timeout, _))) => {println!("Error (timeout ok): {:?}", timeout);Err(())},
                Err(futures::future::Either::A((get_error, _))) => {println!("Error (Client.get err): {:?}", get_error);Err(())},
                Err(futures::future::Either::B((timeout_error, _))) => {println!("Error (timeout err): {:?}", timeout_error);Err(())},
                // Err(e) => {println!("Error (Client.get/Timeout): {:?}", e);Err(())},
            }
        })
        .map(move |r| (r, uri_string))
        // .map_err(|e| println!("Error (Client.get): {:?}", e))
    })
    .and_then(|(res, uri_string)|{
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
        res.body().concat2()
        .map(move |r| (r, uri_string, content_type))
        .map_err(|e| println!("Error (Response.body().concat2): {:?}", e))
    })
    .filter(|t| (*t).2!=ContentType::Other)
    .and_then(|(chunks, uri_string, content_type)|{
        match content_type {
            ContentType::Html => {
                html_sender.send((uri_string, chunks.to_vec()))
                .map_err(|e| println!("Error (html_sender.send): {:?}", e))
            },
            ContentType::Css => {
                css_sender.send(chunks.to_vec())
                .map_err(|e| println!("Error (css_sender.send): {:?}", e))
            },
            _ => panic!("ContentType::Other should have been filtered out"),
        }
    })
    .or_else(|_| {
        if true{
            Ok(())
        } else {
            Err(())
        }
    })
    .for_each(|_| Ok(()));



    match core.run(work) {
        Ok(o) => println!("Ok (Core.run): {:?}", o),
        Err(e) => println!("Error (Core.run): {:?}", e),
    }

    println!("IO loop terminated.");
}
