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
// use std::fs;
// use std::io::Write;
mod murmur;
mod bloom_filter;
// mod crawl_worker;
mod url_reservoir;
mod css_worker;
mod html_worker;

// https://tokio.rs/docs/getting-started/streams-and-sinks/
fn main() {
    let (css_sender, css_receiver) = sync::mpsc::channel::<String>();
    let (html_sender, html_receiver) = sync::mpsc::channel::<(String,String)>();
    let css_written=sync::Arc::new(sync::atomic::AtomicUsize::new(0));
    let htmls_crawled=sync::Arc::new(sync::atomic::AtomicUsize::new(0));

    let bloom_filter=sync::Arc::new(sync::Mutex::new(bloom_filter::LargeBloomFilter::new(vec![0xb77c92ec, 0x660208ac])));
    let url_reservoir=sync::Arc::new(sync::Mutex::new(url_reservoir::UrlReservoir::new(vec!["http://cssdb.co".to_string()], rand::StdRng::new().unwrap())));

    let (uri_sink, uri_stream)=futures::sync::mpsc::channel::<hyper::Uri>(1024);

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

        thread::spawn(move || {
            let mut uri_sink=uri_sink;
            let sleep_duration_per_loop=time::Duration::from_secs(2);
            loop{
                let url={
                    let mut mutex_guard=match url_reservoir.lock() {
                        Ok(mutex_guard) => mutex_guard,
                        Err(e) => {println!("Error: {:?}", e);break;},
                    };

                    match mutex_guard.get_url() {
                        Some(url) => url,
                        None => {println!("Error: {:?}", "reservoir is empty");continue;},
                    }
                };

                let url_has_been_used={
                    let mut mutex_guard=match bloom_filter.lock() {
                        Ok(mutex_guard) => mutex_guard,
                        Err(e) => {println!("Error: {:?}", e);break;},
                    };

                    mutex_guard.contains_add(url.as_bytes())
                };
                if url_has_been_used{
                    println!("Error: {:?}", "url_has_been_used");
                    continue;
                }

                let uri=match url.parse::<hyper::Uri>() {
                    Ok(uri) => uri,
                    Err(e) => {println!("Error: {:?}", e);continue;},
                };

                match uri_sink.try_send(uri) {
                    Ok(_) => {},
                    Err(e) => {println!("Error: {:?}", e);continue;},
                }

                thread::sleep(sleep_duration_per_loop);
            }
        });
    }

    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client = hyper::Client::new(&handle);

    enum ContentType {
        Html,
        Css,
        Other,
    }

    let work=uri_stream.and_then(|uri| {
        let uri_string=uri.to_string();
        client.get(uri)
        .map_err(|e| println!("Error: {:?}", e))
        .and_then(|res|{
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
            .map_err(|e| println!("Error: {:?}", e))
            .and_then(|chunks| {
                match String::from_utf8(chunks.to_vec()) {
                    Err(e) => println!("Error: {:?}", e),
                    Ok(content) => {
                        match html_sender.send((uri_string, content)) {
                            Err(e) => println!("Error: {:?}", e),
                            _ => {},
                        }
                    },
                }

                Ok(())
            })
        })
    }).for_each(|_|{
        Ok(())
    });

    core.run(work).unwrap();

    // match res.headers().get::<hyper::header::ContentType>() {
    //     Some(expr) => expr,
    //     None => expr,
    // }

    // let mut urls=Vec::with_capacity(MAX_URLS_PER_BATCH);
    // // let mut works=Vec::with_capacity(MAX_URLS_PER_BATCH);
    // loop {
    //     urls.clear();
    //     // works.clear();
    //     {
    //         let mut guarded_url_reservoir=match url_reservoir.lock() {
    //             Ok(guarded_url_reservoir) => guarded_url_reservoir,
    //             Err(e) => {println!("Error: {:?}", e);break;},
    //         };

    //         let mut guarded_bloom_filter=match bloom_filter.lock() {
    //             Ok(guarded_bloom_filter) => guarded_bloom_filter,
    //             Err(e) => {println!("Error: {:?}", e);break;},
    //         };

    //         while urls.len()<MAX_URLS_PER_BATCH{
    //             let url=match guarded_url_reservoir.get_url() {
    //                 Some(url) => url,
    //                 None => break,
    //             };

    //             if !guarded_bloom_filter.contains_add(url.as_bytes()){
    //                 urls.push(url);
    //             }
    //         }
    //     }

    //     let url=match urls.pop() {
    //         Some(expr) => expr,
    //         None => {println!("Error: {:?}", "reservoir is empty");continue;},
    //     };

    //     let uri=match url.parse::<hyper::Uri>() {
    //         Ok(expr) => expr,
    //         Err(e) => {println!("Error: {:?}", e);continue;}, //Not ideal, but a hassle to do right. Max urls lost is MAX_URLS_PER_BATCH-1, so not a problem. Goto would actually be nice and elegant here.
    //     };

    //     let work=client.get(uri).and_then(|res|{
    //         res.body().concat2()
    //     });

    // }








    println!("IO loop terminated.");





    // let url="http://httpbin.org/ip".to_string();

    // let work = client.get(url.parse::<hyper::Uri>().unwrap()).and_then(|res| {
    //     println!("Response: {}", url);
    //     println!("Response: {}", res.status());
    //     println!("Headers: \n{}", res.headers());

    //     res.body().concat2().map(|a| println!("{:?}", a))
    // });

    // let a=core.run(work).unwrap();
    // println!("{:?}", std::str::from_utf8(a.as_ref()));
}

