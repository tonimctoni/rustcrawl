// #![feature(drain_filter)]
#![allow(dead_code)]
use bloom_filter;
use url_reservoir;
use regex;
use url;
use std::sync;

const MAX_URLS_PER_SITE: usize = 1000;
const MAX_HOST_SHARING_URLS_PER_SITE: usize = 5;


pub fn html_worker(html_receiver: sync::mpsc::Receiver<(String,Vec<u8>)>, htmls_crawled: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let re=regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();

    let mut urls:Vec<String>=Vec::with_capacity(MAX_URLS_PER_SITE);
    let mut hosts_nums:Vec<(String, usize)>=Vec::with_capacity(MAX_URLS_PER_SITE);
    for (url,html_content) in html_receiver.iter(){
        let url=match url::Url::parse(url.as_str()) {
            Ok(url) => url,
            Err(e) => {println!("Error (html_worker): {:?}", e);continue;},
        };

        let html_content=match String::from_utf8(html_content) {
            Ok(html_content) => html_content,
            Err(e) => {println!("Error (html_worker): {:?}", e.utf8_error());continue;},
        };

        hosts_nums.clear();
        for cap in re.captures_iter(html_content.as_str()).take(MAX_URLS_PER_SITE){
            let cap=match cap.get(1) {
                Some(cap) => cap,
                None => {println!("Error (html_worker): {:?}", "cannot get regex cap 1");continue;},
            };

            let url=match url.join(cap.as_str()) {
                Ok(url) => url,
                Err(e) => {println!("Error (html_worker): {:?}", e);continue;},
            };

            let surpassed_host_num_limit={
                let host=match url.host_str() {
                    Some(host) => host,
                    None => {println!("Error (html_worker): {:?}", "no host");continue;},
                };

                (||{
                    for host_num in hosts_nums.iter_mut(){
                        if (*host_num).0==host{
                            if (*host_num).1<MAX_HOST_SHARING_URLS_PER_SITE{
                                (*host_num).1+=1;
                                return false;
                            } else {
                                return true;
                            }
                        }
                    }
                    hosts_nums.push((host.into(), 0));
                    return false;
                })()
            };
            if surpassed_host_num_limit{
                continue;
            }

            urls.push(url.into_string());
        }

        // Deduplicate urls.
        urls.sort_unstable();
        urls.dedup();

        // Filter out urls that have already been crawled.
        if !urls.is_empty(){
            let mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error (html_worker): {:?}", e);break;},
            };

            urls.retain(|u| !mutex_guard.contains(u.as_bytes()));
        };

        // Add obtained urls to reservoir.
        if !urls.is_empty(){
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error (html_worker): {:?}", e);break;},
            };

            mutex_guard.add_urls_popping(&mut urls);

        }

        htmls_crawled.fetch_add(1, sync::atomic::Ordering::Relaxed);
    }
    println!("Html worker terminated.");
}