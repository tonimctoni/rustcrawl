use bloom_filter;
use url_reservoir;
use futures;
use hyper;
use std::thread;
use std::sync;
use std::time;

const SLEEP_MILLIS_AFTER_SEND: u64 = 12;
const SLEEP_MILLIS_ON_EMPTY_RESERVOIR: u64 = 2000;
const SLEEP_MILLIS_ON_FULL_CHANNEL: u64 = 8000;
const MAX_URLS_PER_ITER: usize = 100;



/// Within an endless loop, it obtains urls from the `url_reservoir` and sends them
/// via `uri_sink` to be processed. It makes use of `bloom_filter` to not send the
/// same url twice.
///
/// # Arguments
///
/// * `uri_sink` - Channel sink where suitable urls are sent through.
/// * `urls_enqueued` - Atomic counter that counts the urls sent through `uri_sink`
/// * `bloom_filter` - BloomFilter that keeps track of already sent urls.
/// * `url_reservoir` - Large structure containing urls that could be sent.
pub fn url_enqueuer(mut uri_sink: futures::sync::mpsc::Sender<hyper::Uri>, urls_enqueued: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let sleep_duration_after_send=time::Duration::from_millis(SLEEP_MILLIS_AFTER_SEND);
    let sleep_duration_on_empty_reservoir=time::Duration::from_millis(SLEEP_MILLIS_ON_EMPTY_RESERVOIR);
    let sleep_duration_on_full_channel=time::Duration::from_millis(SLEEP_MILLIS_ON_FULL_CHANNEL);

    let mut urls=Vec::with_capacity(MAX_URLS_PER_ITER);
    loop {
        // Grab up to MAX_URLS_PER_ITER urls from the reservoir.
        {
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
            };

            urls.clear();
            for _ in 0..MAX_URLS_PER_ITER{
                match mutex_guard.get_url(){
                    Some(url) => urls.push(url),
                    None => break,
                }
            }
        };

        // If no url was grabbed, continue.
        if urls.is_empty(){
            eprintln!("Error (url_enqueuer): {:?}", "reservoir is empty");
            thread::sleep(sleep_duration_on_empty_reservoir);
            continue;
        }

        // Discard urls that have already been sent.
        {
            let mut mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
            };

            urls.retain(|u| !mutex_guard.contains_add(u.as_bytes()));
        }

        // Send urls through the sink.
        for url in urls.iter(){
            let uri=match (*url).parse::<hyper::Uri>() {
                Ok(uri) => uri,
                Err(e) => {
                    eprintln!("Error (url_enqueuer): {:?}", e);
                    continue;
                },
            };

            match uri_sink.try_send(uri) {
                Ok(_) => {
                    urls_enqueued.fetch_add(1, sync::atomic::Ordering::Relaxed);

                    if SLEEP_MILLIS_AFTER_SEND!=0{
                        thread::sleep(sleep_duration_after_send);
                    }
                },
                Err(e) => {
                    eprintln!("Error (url_enqueuer): {:?}", e);
                    thread::sleep(sleep_duration_on_full_channel);
                    continue;
                },
            }
        }
    }

    eprintln!("Url enqueuer terminated.");
}