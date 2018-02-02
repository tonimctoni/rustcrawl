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
const SLEEP_MILLIS_BETWEEN_REPORTS: u64 = 60000;
const REPORT_FILENAME: &str = "report.txt";

fn main() {
    let (css_sender, css_receiver) = sync::mpsc::channel::<Vec<u8>>();
    let (html_sender, html_receiver) = sync::mpsc::channel::<(String,Vec<u8>)>();
    let css_written=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let htmls_crawled=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let urls_enqueued=sync::Arc::new(sync::atomic::AtomicUsize::new(0));

    let bloom_filter=sync::Arc::new(sync::Mutex::new(bloom_filter::LargeBloomFilter::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string(), "http://www.wikipedia.com".to_string()], rand::StdRng::new().unwrap())));

    let (uri_sink, uri_stream)=futures::sync::mpsc::channel::<hyper::Uri>(CHANNEL_BUFFER_SIZE);

    {
        // let css_written=css_written.clone();
        // thread::spawn(move || {
        //     css_worker::css_worker(css_receiver, css_written);
        // });

        let htmls_crawled=htmls_crawled.clone();
        let bloom_filter_c=bloom_filter.clone();
        let url_reservoir_c=url_reservoir.clone();
        thread::spawn(move || {
            html_worker::html_worker(html_receiver, htmls_crawled, bloom_filter_c, url_reservoir_c);
        });

        let urls_enqueued=urls_enqueued.clone();
        thread::spawn(move || {
            url_enqueuer::url_enqueuer(uri_sink, urls_enqueued, bloom_filter, url_reservoir);
        });
    }

    thread::spawn(move || {
        let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_BETWEEN_REPORTS);
        for i in 0.. {
            thread::sleep(sleep_duration_per_iter);
            let mut f=match fs::OpenOptions::new().append(true).create(true).open(REPORT_FILENAME) {
                Ok(f) => f,
                Err(e) => {println!("Error (reporting): {:?}", e);continue;},
            };

            match f.write_all(format!("[report ({})] urls enqueued: {:?}, htmls crawled: {:?}, css written: {:?}\n", i, urls_enqueued.load(sync::atomic::Ordering::Relaxed), htmls_crawled.load(sync::atomic::Ordering::Relaxed), css_written.load(sync::atomic::Ordering::Relaxed)).as_bytes()) {
                Ok(_) => {},
                Err(e) => println!("Error (reporting): {:?}", e),
            }
        }
    });

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::new(&handle);

    enum ContentType {
        Html,
        Css,
        Other,
    }

    let mut c=0;
    let work=uri_stream.and_then(|uri| {
        let uri_string=uri.to_string();
        // println!("{}, {}", c, uri_string);
        println!("{}, {}", c, uri.host().unwrap_or(""));
        c+=1;
        client.get(uri)
        .map_err(|e| println!("Error (Client.get): {:?}", e))
        .and_then(|res|{
            // let content_type=match res.headers().get::<hyper::header::ContentType>(){
            //     Some(content_type) => {
            //         let mimetype=(content_type.type_(), content_type.subtype());
            //         if mimetype.0=="text" && mimetype.1=="html"{
            //             ContentType::Html
            //         } else if mimetype.0=="text" && mimetype.1=="css"{
            //             ContentType::Css
            //         } else{
            //             ContentType::Other
            //         }
            //     },
            //     None => ContentType::Other,
            // };

            // match content_type {
            //     ContentType::Html => {
            //         res.body().concat2()
            //         .map_err(|e| println!("Error (Response.body().concat2): {:?}", e))
            //         .and_then(|chunks|
            //             html_sender.send((uri_string, chunks.to_vec()))
            //             .map_err(|e| println!("Error (html_sender.send): {:?}", e))
            //             .map(|_| ())
            //         )
            //     },
            //     ContentType::Css => {
            //         res.body().concat2()
            //         .map_err(|e| println!("Error (Response.body().concat2): {:?}", e))
            //         .and_then(|chunks|
            //             css_sender.send(chunks.to_vec())
            //             .map_err(|e| println!("Error (css_sender.send): {:?}", e))
            //             .map(|_| ())
            //         )
            //     },
            //     ContentType::Other => Err(()),
            // }

            res.body().concat2()
            .map_err(|e| println!("Error (Response.body().concat2): {:?}", e))
            .and_then(|chunks|
                html_sender.send((uri_string, chunks.to_vec()))
                .map_err(|e| println!("Error (html_sender.send): {:?}", e))
            )
        })
        .or_else(|_| Ok(()))
    }).for_each(|_| Ok(()));

    // let mut c=0;
    // let work=uri_stream.and_then(|uri| {
    //     let uri_string=uri.to_string();
    //     println!("{}, {}", c, uri.host().unwrap_or(""));
    //     c+=1;
    //     client.get(uri)
    // })
    // .map_err(|e| println!("Error (Client.get): {:?}", e))
    // .and_then(|res|{
    //     res.body().concat2()
    // })
    // .map_err(|e| println!("Error (Response.body().concat2): {:?}", e))
    // .and_then(|chunks|{
    //     html_sender.send((uri_string, chunks.to_vec()))
    // })
    // .map_err(|e| println!("Error (html_sender.send): {:?}", e))
    // .for_each(|_| Ok(()));



    match core.run(work) {
        Ok(o) => println!("Ok: {:?}", o),
        Err(e) => println!("Error: {:?}", e),
    }

    println!("IO loop terminated.");
}

// [report (0)] urls enqueued: 576, htmls crawled: 270, css written: 0
// [report (1)] urls enqueued: 1172, htmls crawled: 425, css written: 0
// [report (2)] urls enqueued: 1768, htmls crawled: 597, css written: 0
// [report (3)] urls enqueued: 2012, htmls crawled: 694, css written: 0
