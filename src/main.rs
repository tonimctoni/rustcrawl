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
mod url_reservoir;
mod css_worker;
mod html_worker;
mod url_enqueuer;

const CHANNEL_BUFFER_SIZE: usize = 1024*10;
const SLEEP_MILLIS_BETWEEN_REPORTS: u64 = 6000;
const GET_TIMEOUT_MILLIS: u64 = 5000;
const REPORT_FILENAME: &str = "report.txt";

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

                match f.write_all(format!("[report ({})] urls enqueued: {}, urls gotten: {}, htmls crawled: {}, css written: {}, reservoir contains: {}\n",
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

    // Define tokio Core and client to be used in/as IO loop.
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::new(&handle);

    // Declare enum needed to distinguish between contenttypes of gotten urls.
    #[derive(PartialEq)]
    enum ContentType {
        Html,
        Css,
        Other,
    }

    // Define operations to be run on stram of uris.
    let work=uri_stream
    // Get uri with client.
    .and_then(|uri| {
        let uri_string=uri.to_string();
        // Keep track of number of uris received at stream.
        let c=urls_gotten.fetch_add(1, sync::atomic::Ordering::Relaxed);
        println!("{}, {}", c, uri.host().unwrap_or(""));

        // Define a timeout future. On error, try again in a loop.
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

        // Actually get the uri with client or get a timeout, whichever happens first.
        // On timeout, print and give error.
        client.get(uri)
        .select2(timeout)
        .then(|res|{
            match res {
                Ok(futures::future::Either::A((got, _))) => Ok(got),
                Ok(futures::future::Either::B((timeout, _))) => {println!("Error (get timeout ok): {:?}", timeout);Err(())},
                Err(futures::future::Either::A((get_error, _))) => {println!("Error (Client.get err): {:?}", get_error);Err(())},
                Err(futures::future::Either::B((timeout_error, _))) => {println!("Error (get timeout err): {:?}", timeout_error);Err(())},
            }
        })
        .map(move |r| (r, uri_string))
    })
    // Get the content type and concatenate Chunks of content body.
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

        // Define a timeout future. On error, try again in a loop.
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

        // Concatenate chunks if body or get a timeout, whichever happens first.
        // On timeout, print and give error.
        res.body().concat2()
        .select2(timeout)
        .then(|res|{
            match res {
                Ok(futures::future::Either::A((chunks, _))) => Ok(chunks),
                Ok(futures::future::Either::B((timeout, _))) => {println!("Error (concat timeout ok): {:?}", timeout);Err(())},
                Err(futures::future::Either::A((chunks_error, _))) => {println!("Error (Response.body().concat2 err): {:?}", chunks_error);Err(())},
                Err(futures::future::Either::B((timeout_error, _))) => {println!("Error (concat timeout err): {:?}", timeout_error);Err(())},
            }
        })
        .map(move |r| (r, uri_string, content_type))
    })
    // Filter out content that is not html or css.
    .filter(|t| (*t).2!=ContentType::Other)
    // Send content through the appropiate channel.
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
    // Make sure no error reaches the `for_each`. The else is there so rust knows the error type.
    .or_else(|_| {
        if true{
            Ok(())
        } else {
            Err(())
        }
    })
    // Finally, run all operations on stream.
    .for_each(|_| Ok(()));


    // Run work (operations on stream) in tokio core.
    match core.run(work) {
        Ok(o) => println!("Ok (Core.run): {:?}", o),
        Err(e) => println!("Error (Core.run): {:?}", e),
    }

    println!("IO loop terminated.");
}
