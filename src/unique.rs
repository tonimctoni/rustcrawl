#![allow(dead_code)]

use murmur;

const ARRAY_SIZE: usize = 8192;

#[derive(Clone)]
pub struct Unique{
    unique_array: Box<[u8;ARRAY_SIZE]>,
    seeds: Vec<u32>,
}

// Uses bloom filter
impl Unique {
    pub fn new(seeds: Vec<u32>) -> Unique {
        Unique{
            unique_array: Box::new([0u8;ARRAY_SIZE]), // use box syntax as soon as there is an updated compiler (lazy admin is lazy)
            seeds: seeds,
        }
    }

    pub fn add(&mut self, item: &[u8]){
        let seed_pairs=self.seeds.iter().map(|&x| murmur::murmur_hash3_32(item, x)).map(|x| (x>>16,x&0xffff));

        for (s1,s2) in seed_pairs{
            self.unique_array[(s1>>3) as usize]|=1<<((s1&7) as u8);
            self.unique_array[(s2>>3) as usize]|=1<<((s2&7) as u8);
        }
    }

    pub fn contains(&self, item: &[u8]) -> bool{
        let mut seed_pairs=self.seeds.iter().map(|&x| murmur::murmur_hash3_32(item, x)).map(|x| (x>>16,x&0xffff));

        seed_pairs.all(|(s1,s2)| self.unique_array[(s1>>3) as usize]&((1<<(s1&7)) as u8)!=0 && self.unique_array[(s2>>3) as usize]&((1<<(s2&7)) as u8)!=0)
    }
}

const LARGE_ARRAY_SIZE: usize = 536870912;

#[derive(Clone)]
pub struct LargeUnique{
    unique_array: Vec<u8>, // use box of array as soon as box syntax is here (lazy adming is lazy, again)
    seeds: Vec<u32>,
}

// Uses bloom filter
impl LargeUnique {
    pub fn new(seeds: Vec<u32>) -> LargeUnique {
        let mut vector=Vec::new();
        vector.resize(LARGE_ARRAY_SIZE, 0);
        LargeUnique{
            unique_array: vector,
            seeds: seeds,
        }
    }

    pub fn add(&mut self, item: &[u8]){
        let fourwise_hashes=self.seeds.iter().map(|&x| murmur::murmur_hash3_x64_128(item, x)).map(|(x1,x2)| (x1>>32, x1&0xffffffff, x2>>32, x2&0xffffffff));

        for (s1,s2,s3,s4) in fourwise_hashes{
            self.unique_array[(s1>>3) as usize]|=1<<((s1&7) as u8);
            self.unique_array[(s2>>3) as usize]|=1<<((s2&7) as u8);
            self.unique_array[(s3>>3) as usize]|=1<<((s3&7) as u8);
            self.unique_array[(s4>>3) as usize]|=1<<((s4&7) as u8);
        }
    }

    pub fn contains(&self, item: &[u8]) -> bool{
        let mut fourwise_hashes=self.seeds.iter().map(|&x| murmur::murmur_hash3_x64_128(item, x)).map(|(x1,x2)| (x1>>32, x1&0xffffffff, x2>>32, x2&0xffffffff));

        fourwise_hashes.all(|(s1,s2,s3,s4)| 
            self.unique_array[(s1>>3) as usize]&((1<<(s1&7)) as u8)!=0 &&
            self.unique_array[(s2>>3) as usize]&((1<<(s2&7)) as u8)!=0 &&
            self.unique_array[(s3>>3) as usize]&((1<<(s3&7)) as u8)!=0 &&
            self.unique_array[(s4>>3) as usize]&((1<<(s4&7)) as u8)!=0
            )
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unique() {
        let mut unique=Unique::new(vec![0xa4a759a4, 0xe5f20661, 0x85684b56, 0xba444a10]);

        let to_add=[b"hello" as &[u8], b"NaN" as &[u8], b"" as &[u8], b"ohle" as &[u8], b"some rather long string just because. And just in case, lets make it even longer :D" as &[u8], b"1" as &[u8]];
        let not_to_add=[b"bye" as &[u8], b"ehlo" as &[u8], b"lello" as &[u8], b"_" as &[u8], b"another rather long string just because. And just in case, lets make it even longer :D" as &[u8], b"0" as &[u8]];

        for s in to_add.iter(){
            unique.add(s);
        }

        for s in to_add.iter(){
            assert!(unique.contains(s), format!("Unique says a word that was added is not contained. ({:?})", s));
        }

        for s in not_to_add.iter(){
            assert!(!unique.contains(s), format!("Unique says a word that was not added is contained. ({:?})", s));
        }
    }

    #[test]
    fn test_large_unique() {
        let mut unique=LargeUnique::new(vec![0xa4a759a4, 0xe5f20661]);

        let to_add=[b"hello" as &[u8], b"NaN" as &[u8], b"" as &[u8], b"ohle" as &[u8], b"some rather long string just because. And just in case, lets make it even longer :D" as &[u8], b"1" as &[u8]];
        let not_to_add=[b"bye" as &[u8], b"ehlo" as &[u8], b"lello" as &[u8], b"_" as &[u8], b"another rather long string just because. And just in case, lets make it even longer :D" as &[u8], b"0" as &[u8]];

        for s in to_add.iter(){
            unique.add(s);
        }

        for s in to_add.iter(){
            assert!(unique.contains(s), format!("Unique says a word that was added is not contained. ({:?})", s));
        }

        for s in not_to_add.iter(){
            assert!(!unique.contains(s), format!("Unique says a word that was not added is contained. ({:?})", s));
        }
    }
}