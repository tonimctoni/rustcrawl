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
    rng: rand::StdRng,
}

impl UrlReservoir {
    /// Creates and returns a new UrlReservoir structure.
    ///
    /// # Arguments
    ///
    /// * `starting_urls` - strings the structure should contain right after creation.
    pub fn new(starting_urls: Vec<String>, rng: rand::StdRng) -> UrlReservoir{
        let mut urls=Vec::with_capacity(RESERVOIR_SIZE);
        urls.extend(starting_urls.into_iter());
        UrlReservoir{urls: urls, rng: rng}
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
    pub fn add_urls(&mut self, urls: Vec<String>){
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
                    self.urls[(self.rng.next_u64()%(len as u64)) as usize]=url;
                }
            }
        }
        assert!(self.urls.capacity()==RESERVOIR_SIZE);
    }

    /// Retrieves a random one of the contained strings, or None if the UrlReservoir
    /// structure is empty.
    pub fn get_url(&mut self) -> Option<String>{
        let len=self.urls.len();
        if len==0{
            None
        } else{
            Some(self.urls.swap_remove((self.rng.next_u64()%(len as u64)) as usize))
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand;

    #[test]
    fn test_url_reservoir() {
        let mut url_reservoir=UrlReservoir::new(vec!["hello".to_string()], rand::StdRng::new().unwrap());
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE-1);
        assert_eq!(url_reservoir.get_url(), Some("hello".to_string()));
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE);

        url_reservoir.add_urls(vec!["1".to_string(), "1".to_string()]);
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE-2);
        assert_eq!(url_reservoir.get_url(), Some("1".to_string()));
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE-1);
        assert_eq!(url_reservoir.get_url(), Some("1".to_string()));
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE);
        assert_eq!(url_reservoir.get_url(), None);
        assert_eq!(url_reservoir.available_space(), RESERVOIR_SIZE);
    }
}