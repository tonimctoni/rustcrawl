#![allow(dead_code)]
use url;
use reqwest;
use regex;
use std::sync;
use unique;
use url_reservoir;
use rand;

const MAX_URLS_PER_SITE: usize = 40;

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
            Ok(re.captures_iter(response_text.as_str())
                .map(|cap| cap.get(1))
                .filter(|&cap| cap.is_some())
                .map(|cap| url.join(cap.unwrap().as_str()))
                .filter(|url| url.is_ok())
                .map(|url| url.unwrap().into_string())
                .take(MAX_URLS_PER_SITE)
                .collect())
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


pub fn worker(mut css_sender: sync::mpsc::Sender<String>, unique: sync::Arc<sync::Mutex<unique::Unique>>, url_reservoir: sync::Arc<sync::Mutex<url_reservoir::UrlReservoir>>){
    let client=reqwest::Client::new();
    let re=regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();
    let mut rng=rand::thread_rng();

    loop {
        let url={
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(_) => break,
            };

            match mutex_guard.get_url(&mut rng) {
                Some(url) => url,
                None => continue,
            }
        };

        println!("{:?}", url);


        let url_has_been_used={
            let mut mutex_guard=match unique.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(_) => break,
            };

            if mutex_guard.contains(url.as_bytes()){
                true
            } else {
                mutex_guard.add(url.as_bytes());
                false
            }
        };

        if url_has_been_used{
            continue;
        }

        let url=match url::Url::parse(url.as_str()) {
            Ok(url) => url,
            Err(_) => continue,
        };

        let urls=match get_urls_send_css(url, &client, &re, &mut css_sender) {
            Ok(urls) => urls,
            Err(_) => continue,
        };

        {
            let mut mutex_guard=match url_reservoir.lock() {
                Ok(mutex_guard) => mutex_guard,
                Err(_) => break,
            };

            mutex_guard.add_urls(urls, &mut rng);
        }
    }
}