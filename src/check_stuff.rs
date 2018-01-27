#![allow(dead_code)]

use rand;
use rand::Rng;
use unique;


pub fn check_colissions_large_unique(){
    let mut random_bytes=[0u8;65];
    let mut rng=rand::thread_rng();

    let mut unique=unique::LargeUnique::new(vec![0xa4a759a4, 0xe5f20661]);

    let mut collisions=0;
    for i in 0..{
        // println!("{:?}, {:?}", i, collisions);
        rng.fill_bytes(&mut random_bytes);

        if unique.contains(&random_bytes){
            collisions+=1;
            println!("{:?}, {:?}, {:?}", i-collisions, collisions, ((i-collisions) as f64)/(i as f64 + collisions as f64));
            if collisions>1000{
                break
            }
        }

        unique.add(&random_bytes);
    }
}

pub fn check_colissions_unique(){
    let mut random_bytes=[0u8;65];
    let mut rng=rand::thread_rng();

    let mut unique=unique::Unique::new(vec![0xa4a759a4, 0xe5f20661, 0x85684b56, 0xba444a10]);

    let mut collisions=0;
    for i in 0..{
        // println!("{:?}, {:?}", i, collisions);
        rng.fill_bytes(&mut random_bytes);

        if unique.contains(&random_bytes){
            collisions+=1;
            println!("{:?}, {:?}, {:?}", i-collisions, collisions, ((i-collisions) as f64)/(i as f64 + collisions as f64));
            if collisions>1000{
                break
            }
        }

        unique.add(&random_bytes);
    }
}