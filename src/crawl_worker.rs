#![allow(dead_code)]
use url;
use reqwest;
use regex;
use std::sync;
use bloom_filter;
use url_reservoir;
use thread;
use std::time;

const MAX_URLS_PER_SITE: usize = 100;
const SLEEP_SECONDS_PER_LOOP: u64 = 1;

/// Composite error structure for easier error handling.
#[derive(Debug)]
enum CompositeError {
    ReqwestError(reqwest::Error),
    StringSendError(sync::mpsc::SendError<String>),
    OtherError,
}

impl From<reqwest::Error> for CompositeError {
    fn from(err: reqwest::Error) -> CompositeError {
        CompositeError::ReqwestError(err)
    }
}

impl From<sync::mpsc::SendError<String>> for CompositeError {
    fn from(err: sync::mpsc::SendError<String>) -> CompositeError {
        CompositeError::StringSendError(err)
    }
}

impl From<()> for CompositeError {
    fn from(_: ()) -> CompositeError {
        CompositeError::OtherError
    }
}

/// Opens and reads `url`. If it contains html, it searches the html text for links
/// and returns the found links. If it contains css, it sends the css text via the
/// `css_sender` channel and returns an empty vector. If it contains something else,
/// an empty vector is also returned.
///
/// # Arguments
///
/// * `url` - Url to request.
/// * `client` - Client to requests urls with.
/// * `re` - Regular expression to find links in html text.
/// * `css_sender` - Sender channel to send css text through if found.
fn get_urls_send_css(url: url::Url, client: &reqwest::Client, re: &regex::Regex, css_sender: &mut sync::mpsc::Sender<String>) -> Result<Vec<String>, CompositeError>{
    enum ContentType{
        Html,
        Css,
        Other
    }

    let mut response=try!(client.get(url.clone()).send());
    let content_type={
        let content_type=response.headers().get::<reqwest::header::ContentType>();
        let content_type=try!(content_type.ok_or(()));
        let mimetype=(content_type.type_(), content_type.subtype());

        if mimetype.0=="text" && mimetype.1=="html"{
            ContentType::Html
        } else if mimetype.0=="text" && mimetype.1=="css"{
            ContentType::Css
        } else{
            ContentType::Other
        }
    };

    match content_type {
        ContentType::Html =>{
            let response_text=try!(response.text());
            // Ok(re.captures_iter(response_text.as_str())
            //     .map(|cap| cap.get(1))
            //     .filter(|&cap| cap.is_some())
            //     .map(|cap| url.join(cap.unwrap().as_str()))
            //     .filter(|url| url.is_ok())
            //     .map(|url| url.unwrap().into_string())
            //     .take(MAX_URLS_PER_SITE)
            //     .collect())
            let mut urls=Vec::with_capacity(MAX_URLS_PER_SITE);
            for cap in re.captures_iter(response_text.as_str()).take(MAX_URLS_PER_SITE){
                let cap=match cap.get(1) {
                    Some(expr) => expr,
                    None => {eprintln!("Error: {:?}", "cannot get regex cap 1");continue;},
                };

                let url=match url.join(cap.as_str()) {
                    Ok(expr) => expr.into_string(),
                    Err(e) => {eprintln!("Error: {:?}", e);continue;},
                };

                urls.push(url);
            }

            Ok(urls)
        },
        ContentType::Css =>{
            try!(css_sender.send(try!(response.text())));
            Ok(vec![])
        },
        ContentType::Other =>{
            Ok(vec![])
        },
    }
}

/// Within an endless loop, it obtains an url from the `url_reservoir` and crawls
/// it with `get_urls_send_css`. Found urls are added to `url_reservoir` and `bloom_filter`
/// if not contained in `bloom_filter`. If css text is found, it is sent via the
/// `css_sender` channel. After successfully crawling the `url`, `urls_crawled` is
/// incremented.
///
/// # Arguments
///
/// * `css_sender` - Sender channel to send css text through when found.
/// * `bloom_filter` - LargeBloomFilter structure that helps avoiding crawling the same url several times.
/// * `url_reservoir` - UrlReservoir structure to get urls from and add urls to.
/// * `urls_crawled` - Atomic counter that keeps track of the ammount of urls crawled.
pub fn worker(mut css_sender: sync::mpsc::Sender<String>, bloom_filter: sync::Arc<sync::Mutex<bloom_filter::LargeBloomFilter>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>, urls_crawled: sync::Arc<sync::atomic::AtomicUsize>){
    let client=reqwest::Client::new();
    let re=regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();

    let sleep_duration_per_loop=time::Duration::from_secs(SLEEP_SECONDS_PER_LOOP);
    loop {
        thread::sleep(sleep_duration_per_loop);

        // Get url; continue if reservoir is impty, break if lock is broken.
        let url={
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error: {:?}", e);break;},
            };

            match mutex_guard.get_url() {
                Some(url) => url,
                None => {eprintln!("Error: {:?}", "reservoir is empty");continue;},
            }
        };

        // Check if url has been crawled already. If not, add to bloom_filter and go on. If yes, continue.
        let url_has_been_used={
            let mut mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error: {:?}", e);break;},
            };

            mutex_guard.contains_add(url.as_bytes())
        };
        if url_has_been_used{
            eprintln!("Error: {:?}", "url_has_been_used");
            continue;
        }

        // Transform url from string to url format. Should not fail, since the string was gotten from a url. If it fails, continue.
        let url=match url::Url::parse(url.as_str()) {
            Ok(url) => url,
            Err(e) => {eprintln!("Error: {:?}", e);continue;},
        };

        // Crawl the url. On error continue.
        let mut urls=match get_urls_send_css(url, &client, &re, &mut css_sender) {
            Ok(urls) => urls,
            Err(e) => {eprintln!("Error: {:?}", e);continue;},
        };

        // Since url was crawled, increase counter.
        urls_crawled.fetch_add(1, sync::atomic::Ordering::Relaxed);

        // Deduplicate urls.
        urls.sort_unstable();
        urls.dedup();

        // Filter out urls that have already been crawled.
        let urls={
            let mutex_guard=match bloom_filter.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error: {:?}", e);break;},
            };

            urls.into_iter().filter(|u| !mutex_guard.contains(u.as_bytes())).collect::<Vec<_>>()
        };

        // Add obtained urls to reservoir.
        {
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(e) => {eprintln!("Error: {:?}", e);break;},
            };

            mutex_guard.add_urls(urls);
        }
    }

    eprintln!("Worker terminated.");
}