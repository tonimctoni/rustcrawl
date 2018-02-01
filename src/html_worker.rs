// #![feature(drain_filter)]
#![allow(dead_code)]
use bloom_filter;
use url_reservoir;
use regex;
use url;
use std::sync;

const MAX_URLS_PER_SITE: usize = 100;


pub fn html_worker(html_receiver: sync::mpsc::Receiver<(String,String)>, htmls_crawled: sync::Arc<sync::atomic::AtomicUsize>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let re=regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();

    let mut urls=Vec::with_capacity(MAX_URLS_PER_SITE);
    for (url,html_content) in html_receiver.iter(){
        let url=match url::Url::parse(url.as_str()) {
            Ok(url) => url,
            Err(e) => {println!("Error: {:?}", e);continue;},
        };

        for cap in re.captures_iter(html_content.as_str()).take(MAX_URLS_PER_SITE){
            let cap=match cap.get(1) {
                Some(expr) => expr,
                None => {println!("Error: {:?}", "cannot get regex cap 1");continue;},
            };

            let url=match url.join(cap.as_str()) {
                Ok(expr) => expr.into_string(),
                Err(e) => {println!("Error: {:?}", e);continue;},
            };

            urls.push(url);
        }

        // Deduplicate urls.
        urls.sort_unstable();
        urls.dedup();

        // Filter out urls that have already been crawled.
        if !urls.is_empty(){
            let mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error: {:?}", e);break;},
            };

            urls.retain(|u| !mutex_guard.contains(u.as_bytes()));
        };

        // Add obtained urls to reservoir.
        if !urls.is_empty(){
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {println!("Error: {:?}", e);break;},
            };

            mutex_guard.add_urls_popping(&mut urls);

        }

        htmls_crawled.fetch_add(1, sync::atomic::Ordering::Relaxed);
    }
    println!("Html worker terminated.");
}