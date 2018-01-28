// extern crate rand;
extern crate url;
extern crate reqwest;
extern crate regex;
// use std::thread;
use std::sync; // ::mpsc::channel
mod murmur;
mod unique;
// mod check_stuff;

const MAX_URLS_PER_SITE: usize = 40;

#[derive(Debug)]
enum CompositeError {
    ReqwestError(reqwest::Error),
    OtherError,
}

impl From<reqwest::Error> for CompositeError {
    fn from(err: reqwest::Error) -> CompositeError {
        CompositeError::ReqwestError(err)
    }
}

impl From<()> for CompositeError {
    fn from(err: ()) -> CompositeError {
        CompositeError::OtherError
    }
}


fn get_urls_send_css(url: url::Url, client: &reqwest::Client, re: &regex::Regex, css_sender: &mut sync::mpsc::Sender<String>) -> Result<Vec<url::Url>, CompositeError>{
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
                .map(|url| url.unwrap())
                .take(MAX_URLS_PER_SITE)
                .collect())
        },
        ContentType::Css =>{
            css_sender.send(try!(response.text()));
            Ok(vec![])
        },
        ContentType::Other =>{
            Ok(vec![])
        },
    }
}


fn worker(mut css_sender: sync::mpsc::Sender<String>){
    let client=reqwest::Client::new();
    let re = regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();

    let url=url::Url::parse("http://cssdb.co").unwrap();
    let urls=get_urls_send_css(url, &client, &re, &mut css_sender);
    println!("{:?}", urls);
}


fn main() {
    let (css_sender, _) = sync::mpsc::channel::<String>();
    worker(css_sender.clone());
    // let client=reqwest::Client::new();

    // let url=url::Url::parse("http://cssdb.co").unwrap();
    // let mut response=client.get(url).send().unwrap();
    // println!("{:?}", response.headers().get::<reqwest::header::ContentType>().unwrap().0.subtype());
    // let text=response.text().unwrap();

    // // // println!("{:?}", text);
    // let re = regex::Regex::new("(?:href=|src=|url=)[\"']?([^\"' <>]*)").unwrap();

    // let url=url::Url::parse("http://cssdb.co").unwrap();
    // // for cap in re.captures_iter(text.as_str()){
    // //     println!("{:?}, {:?}", &cap[1], url.join(&cap[1]));
    // // }
    // let cap:Vec<url::Url>=re.captures_iter(text.as_str())
    //     .map(|cap| cap.get(1))
    //     .filter(|&cap| cap.is_some())
    //     .map(|cap| url.join(cap.unwrap().as_str()))
    //     .filter(|url| url.is_ok())
    //     .map(|url| url.unwrap())
    //     .take(MAX_URLS_PER_SITE)
    //     .collect();
    // println!("{:?}", cap);
}

// 20 40 80 120 200
// https://github.com/tonimctoni/rustcrawl/blob/master/src/unique.rs
// https://docs.rs/mime/0.3.5/mime/struct.Mime.html