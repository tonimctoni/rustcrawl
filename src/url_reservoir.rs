#![allow(dead_code)]
use rand;
use rand::Rng;



const RESERVOIR_SIZE: usize = 1024*1024;
pub struct UrlReservoir {
    urls: Vec<String>,
}

impl UrlReservoir {
    pub fn new(starting_urls: Vec<String>) -> UrlReservoir{
        let mut urls=Vec::with_capacity(RESERVOIR_SIZE);
        urls.extend(starting_urls.into_iter());
        UrlReservoir{urls: urls}
    }

    pub fn available_space(&self) -> usize{
        self.urls.capacity()-self.urls.len()
    }

    pub fn add_urls(&mut self, urls: Vec<String>, rng: &mut rand::ThreadRng){
        let mut available_space=self.available_space();
        if available_space>=urls.len(){
            self.urls.extend(urls.into_iter());
        } else {
            for url in urls.into_iter(){
                if available_space>0{
                    self.urls.push(url);
                    available_space-=1;
                } else {
                    let len=self.urls.len();
                    self.urls[(rng.next_u64()%(len as u64)) as usize]=url;
                }
            }
        }
        assert!(self.urls.capacity()==RESERVOIR_SIZE);
    }

    pub fn get_url(&mut self, rng: &mut rand::ThreadRng) -> Option<String>{
        let len=self.urls.len();
        if len==0{
            None
        } else{
            Some(self.urls.swap_remove((rng.next_u64()%(len as u64)) as usize))
        }
    }
}