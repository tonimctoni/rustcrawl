#![allow(dead_code)]
use rand;
use rand::Rng;


/// Data structure designed to hold a large but finite amount of strings. Adding
/// strings beyond capacity replaces random strings. The strings are intended to
/// represent urls, hence the name.
///
/// Retrieving data happens in random order.
const RESERVOIR_SIZE: usize = 1024*1024;
pub struct UrlReservoir {
    urls: Vec<String>,
}

impl UrlReservoir {
    /// Creates and returns a new UrlReservoir structure.
    ///
    /// # Arguments
    ///
    /// * `starting_urls` - strings the structure should contain right after creation.
    pub fn new(starting_urls: Vec<String>) -> UrlReservoir{
        let mut urls=Vec::with_capacity(RESERVOIR_SIZE);
        urls.extend(starting_urls.into_iter());
        UrlReservoir{urls: urls}
    }

    /// Returns the ammount of strings that could be added to the UrlReservoir
    /// structure before the strings it already contains start having to be removed.
    pub fn available_space(&self) -> usize{
        self.urls.capacity()-self.urls.len()
    }

    /// Adds strings to the UrlReservoir structure, removing already contained
    /// strings if it is already full.
    ///
    /// # Arguments
    ///
    /// * `urls` - vector of strings to add to the UrlReservoir structure.
    /// * `rng` - random number generator, needed to decide which strings to replace.
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

    /// Retrieves a random one of the contained strings, or None if the UrlReservoir
    /// structure is empty.
    ///
    /// # Arguments
    ///
    /// * `rng` - random number generator, needed to decide which strings to retrieve.
    pub fn get_url(&mut self, rng: &mut rand::ThreadRng) -> Option<String>{
        let len=self.urls.len();
        if len==0{
            None
        } else{
            Some(self.urls.swap_remove((rng.next_u64()%(len as u64)) as usize))
        }
    }
}