extern crate rand;
extern crate url;
extern crate hyper;
extern crate tokio_core;
extern crate futures;
mod murmur;
mod unique;
mod check_stuff;
use std::io::{self, Write};
use futures::Future;
use futures::stream::Stream;
use hyper::Client;

fn main() {
    let mut core = tokio_core::reactor::Core::new().unwrap();
    let handle = core.handle();
    let client=hyper::Client::new(&handle);



    let url="http://www.google.com".parse::<hyper::Uri>().unwrap();
    let work = client.get(url).and_then(|res| {

        println!("Response: {}", res.status());
        println!("Headers: \n{}", res.headers());
        
        res.body().for_each(|chunk| {
            io::stdout().write_all(&chunk).map_err(From::from)
        })
    });


    core.run(work).unwrap();
}

// 20 40 80 120 200
// https://github.com/tonimctoni/rustcrawl/blob/master/src/unique.rs