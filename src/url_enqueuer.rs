use bloom_filter;
use url_reservoir;
use futures;
use hyper;
use std::thread;
use std::sync;
use std::time;

const SLEEP_MILLIS_PER_ITER: u64 = 10;
const SLEEP_MILLIS_ON_EMPTY_RESERVOIR: u64 = 2000;
const SLEEP_MILLIS_ON_FULL_CHANNEL: u64 = 40000;


/// Within an endless loop, it obtains an url from the `url_reservoir` and sends
/// it via `uri_sink` to be processed. It makes use of `bloom_filter` to not send
/// the same url twice.
///
/// # Arguments
///
/// * `uri_sink` - Channel sink where suitable urls are sent through.
/// * `urls_enqueued` - Atomic counter that counts the urls sent through `uri_sink`
/// * `bloom_filter` - BloomFilter that keeps track of already sent urls.
/// * `url_reservoir` - Large structure containing urls that could be sent.
pub fn url_enqueuer(mut uri_sink: futures::sync::mpsc::Sender<hyper::Uri>, urls_enqueued: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_PER_ITER);
    let sleep_duration_on_empty_reservoir=time::Duration::from_millis(SLEEP_MILLIS_ON_EMPTY_RESERVOIR);
    let sleep_duration_on_full_channel=time::Duration::from_millis(SLEEP_MILLIS_ON_FULL_CHANNEL);

    loop{
        // Gets a url from the url reservoir if not empty.
        let maybe_url={
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
            };

            mutex_guard.get_url()
        };

        // If url was not gotten, because the url reservoir was empty, sleep and continue.
        let url=match maybe_url {
            Some(url) => url,
            None => {eprintln!("Error (url_enqueuer): {:?}", "reservoir is empty");thread::sleep(sleep_duration_on_empty_reservoir);continue;},
        };

        // Uses bloom filter to make sure url was not sent before already. If so, continue.
        let url_has_been_used={
            let mut mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
            };

            mutex_guard.contains_add(url.as_bytes())
        };
        if url_has_been_used{
            eprintln!("Error (url_enqueuer): {:?}", "url_has_been_used");
            continue;
        }

        // Make a uri to use with Client.get out of url string. On error, continue.
        let uri=match url.parse::<hyper::Uri>() {
            Ok(uri) => uri,
            Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);continue;},
        };

        // Try to send uri via uri channel sink. If not possible (probably because it is full), sleep and continue.
        match uri_sink.try_send(uri) {
            Ok(_) => {},
            Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);thread::sleep(sleep_duration_on_full_channel);continue;},
        }

        // Keep track of number of uris sent with atomic counter `urls_enqueued`.
        urls_enqueued.fetch_add(1, sync::atomic::Ordering::Relaxed);

        // If SLEEP_MILLIS_PER_ITER is not zero, sleep here after having sent an uri.
        if SLEEP_MILLIS_PER_ITER!=0{
            thread::sleep(sleep_duration_per_iter);
        }
    }

    eprintln!("Url enqueuer terminated.");
}



// const MAX_URLS_PER_ITER: usize = 20;
// pub fn url_enqueuer(mut uri_sink: futures::sync::mpsc::Sender<hyper::Uri>, urls_enqueued: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
//     let sleep_duration_per_iter=time::Duration::from_millis(SLEEP_MILLIS_PER_ITER);
//     let sleep_duration_on_empty_reservoir=time::Duration::from_millis(SLEEP_MILLIS_ON_EMPTY_RESERVOIR);
//     let sleep_duration_on_full_channel=time::Duration::from_millis(SLEEP_MILLIS_ON_FULL_CHANNEL);

//     let mut urls=Vec::with_capacity(MAX_URLS_PER_ITER);
//     loop {
//         {
//             let mut mutex_guard=match url_reservoir.lock() {
//                 Ok(mutex_guard) => mutex_guard,
//                 Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
//             };

//             urls.clear();
//             for _ in 0..MAX_URLS_PER_ITER{
//                 match mutex_guard.get_url(){
//                     Some(url) => urls.push(url),
//                     None => break,
//                 }
//             }
//         };

//         if urls.is_empty(){
//             eprintln!("Error (url_enqueuer): {:?}", "reservoir is empty");
//             thread::sleep(sleep_duration_on_empty_reservoir);
//             continue;
//         }


//         {
//             let mut mutex_guard=match bloom_filter.lock() {
//                 Ok(mutex_guard) => mutex_guard,
//                 Err(e) => {eprintln!("Error (url_enqueuer): {:?}", e);break;},
//             };

//             urls.retain(|u| !mutex_guard.contains_add(u.as_bytes()));
//         }

//         for url in urls.iter(){
//             let uri=match (*url).parse::<hyper::Uri>() {
//                 Ok(uri) => uri,
//                 Err(e) => {
//                     eprintln!("Error (url_enqueuer): {:?}", e);
//                     continue;
//                 },
//             };

//             match uri_sink.try_send(uri) {
//                 Ok(_) => {},
//                 Err(e) => {
//                     eprintln!("Error (url_enqueuer): {:?}", e);
//                     thread::sleep(sleep_duration_on_full_channel);
//                     continue;
//                 },
//             }
//         }

//         urls_enqueued.fetch_add(urls.len(), sync::atomic::Ordering::Relaxed);

//         if SLEEP_MILLIS_PER_ITER!=0{
//             thread::sleep(sleep_duration_per_iter);
//         }
//     }

//     eprintln!("Url enqueuer terminated.");
// }