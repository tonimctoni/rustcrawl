use bloom_filter;
use url_reservoir;
use futures;
use hyper;
use std::thread;
use std::sync;
use std::time;

const SLEEP_MILLIS_PER_ITER: u64 = 100;
const SLEEP_MILLIS_ON_EMPTY_RESERVOIR: u64 = 2000;
const SLEEP_MILLIS_ON_FULL_CHANNEL: u64 = 10000;
const SLEEP_MILLIS_ON_PEEK_FULL_CHANNEL: u64 = 0;

pub fn url_enqueuer(mut uri_sink: futures::sync::mpsc::Sender<hyper::Uri>, urls_enqueued: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_PER_ITER);
    let sleep_duration_on_empty_reservoir=time::Duration::from_millis(SLEEP_MILLIS_ON_EMPTY_RESERVOIR);
    let sleep_duration_on_full_channel=time::Duration::from_millis(SLEEP_MILLIS_ON_FULL_CHANNEL);
    let sleep_duration_on_peek_full_channel=time::Duration::from_millis(SLEEP_MILLIS_ON_PEEK_FULL_CHANNEL);

    loop{
        // let url={
        //     let mut guarded_bloom_filter=match bloom_filter.lock() {
        //         Ok(guarded_bloom_filter) => guarded_bloom_filter,
        //         Err(e) => {println!("Error (url_enqueuer): {:?}", e);break;},
        //     };

        //     let mut guarded_url_reservoir=match url_reservoir.lock() {
        //         Ok(guarded_url_reservoir) => guarded_url_reservoir,
        //         Err(e) => {println!("Error (url_enqueuer): {:?}", e);break;},
        //     };

        //     let mut url=match guarded_url_reservoir.get_url() {
        //         Some(url) => url,
        //         None => {println!("Error (url_enqueuer): {:?}", "reservoir is empty");continue;},
        //     };

        //     while !guarded_bloom_filter.contains_add(url.as_bytes()){
        //         url=match guarded_url_reservoir.get_url() {
        //             Some(url) => url,
        //             None => {println!("Error (url_enqueuer): {:?}", "reservoir is empty");continue;},
        //         };
        //     }

        //     url
        // };

        if SLEEP_MILLIS_ON_PEEK_FULL_CHANNEL!=0{
            match uri_sink.poll_ready() {
                Ok(_) => {},
                Err(e) => {println!("Error (url_enqueuer): {:?}", e);thread::sleep(sleep_duration_on_peek_full_channel);continue;},
            }
        }

        let maybe_url={
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error (url_enqueuer): {:?}", e);break;},
            };

            mutex_guard.get_url()
        };

        let url=match maybe_url {
            Some(url) => url,
            None => {println!("Error (url_enqueuer): {:?}", "reservoir is empty");thread::sleep(sleep_duration_on_empty_reservoir);continue;},
        };

        let url_has_been_used={
            let mut mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error (url_enqueuer): {:?}", e);break;},
            };

            mutex_guard.contains_add(url.as_bytes())
        };
        if url_has_been_used{
            println!("Error (url_enqueuer): {:?}", "url_has_been_used");
            continue;
        }

        let uri=match url.parse::<hyper::Uri>() {
            Ok(uri) => uri,
            Err(e) => {println!("Error (url_enqueuer): {:?}", e);continue;},
        };

        match uri_sink.try_send(uri) {
            Ok(_) => {},
            Err(e) => {println!("Error (url_enqueuer): {:?}", e);thread::sleep(sleep_duration_on_full_channel);continue;},
        }

        urls_enqueued.fetch_add(1, sync::atomic::Ordering::Relaxed);
        if SLEEP_MILLIS_PER_ITER!=0{
            thread::sleep(sleep_duration_per_iter);
        }
    }
    println!("Url enqueuer terminated.");
}