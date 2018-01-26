#![allow(dead_code)]

use murmur;


#[derive(Clone)]
pub struct Unique{
    unique_array: Box<[u8;8192]>,
    seeds: Vec<u32>,
}


impl Unique {
    pub fn new(seeds: Vec<u32>) -> Unique {
        Unique{
            unique_array: Box::new([0u8;8192]), // use box syntax as soon as there is an updated compiler (lazy admin is lazy)
            seeds: seeds,
        }
    }

    // fn set_bit(&mut self, i: usize){
    //     self.unique_array[i>>3]|=((i&7) as u8)<<1;
    // }

    // fn get_bit(&self, i: usize) -> bool{
    //     self.unique_array[i>>3]&(((i&7) as u8)<<1)!=0
    // }

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
}