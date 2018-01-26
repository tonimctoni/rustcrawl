#![allow(dead_code)]

// copied from https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp
pub fn murmur_hash3_x64_128(input: &[u8], seed: u32) -> [u64;2] {
    let nblocks=input.len()/16;
    let mut h1=seed as u64;
    let mut h2=seed as u64;
    let c1=0x87c37b91114253d5u64;
    let c2=0x4cf5ad432745937fu64;

    for i in 0..nblocks{
        // let mut k1=(0..8).fold(0u64, |acc, n| acc|(input[i*16+n] as u64) << n);
        // let mut k2=(0..8).fold(0u64, |acc, n| acc|(input[i*16+n+8] as u64) << n);
        let (mut k1, mut k2):(u64,u64)=unsafe{
            ((0..8).fold(0u64, |acc, n| acc|(*input.get_unchecked(i*16+n) as u64) << n),
            (0..8).fold(0u64, |acc, n| acc|(*input.get_unchecked(i*16+n+8) as u64) << n))
        };

        k1=k1.wrapping_mul(c1);
        k1=k1.rotate_left(31);
        k1=k1.wrapping_mul(c2);
        h1^=k1;

        h1=h1.rotate_left(27);
        h1=h1.wrapping_add(h2);
        h1=(h1.wrapping_mul(5)).wrapping_add(0x52dce729);

        k2=k2.wrapping_mul(c2);
        k2=k2.rotate_left(33);
        k2=k2.wrapping_mul(c1);
        h2^=k2;

        h2=h2.rotate_left(31);
        h2=h2.wrapping_add(h1);
        h2=(h2.wrapping_mul(5)).wrapping_add(0x38495ab5);
    }

    let tail=&input[nblocks*16..];
    // assert!(tail.len()<16);

    let mut k1=0u64;
    let mut k2=0u64;

    if tail.len()>=15{
        k2^=(unsafe{*tail.get_unchecked(14)} as u64)<<48;
    }
    if tail.len()>=14{
        k2^=(unsafe{*tail.get_unchecked(13)} as u64)<<40;
    }
    if tail.len()>=13{
        k2^=(unsafe{*tail.get_unchecked(12)} as u64)<<32;
    }
    if tail.len()>=12{
        k2^=(unsafe{*tail.get_unchecked(11)} as u64)<<24;
    }
    if tail.len()>=11{
        k2^=(unsafe{*tail.get_unchecked(10)} as u64)<<16;
    }
    if tail.len()>=10{
        k2^=(unsafe{*tail.get_unchecked(9)} as u64)<<8;
    }
    if tail.len()>=9{
        k2^=(unsafe{*tail.get_unchecked(8)} as u64)<<0;

        k2=k2.wrapping_mul(c2);
        k2=k2.rotate_left(33);
        k2=k2.wrapping_mul(c1);
        h2^=k2;
    }

    if tail.len()>=8{
        k1^=(unsafe{*tail.get_unchecked(7)} as u64)<<56;
    }
    if tail.len()>=7{
        k1^=(unsafe{*tail.get_unchecked(6)} as u64)<<48;
    }
    if tail.len()>=6{
        k1^=(unsafe{*tail.get_unchecked(5)} as u64)<<40;
    }
    if tail.len()>=5{
        k1^=(unsafe{*tail.get_unchecked(4)} as u64)<<32;
    }
    if tail.len()>=4{
        k1^=(unsafe{*tail.get_unchecked(3)} as u64)<<24;
    }
    if tail.len()>=3{
        k1^=(unsafe{*tail.get_unchecked(2)} as u64)<<16;
    }
    if tail.len()>=2{
        k1^=(unsafe{*tail.get_unchecked(1)} as u64)<<8;
    }
    if tail.len()>=1{
        k1^=(unsafe{*tail.get_unchecked(0)} as u64)<<0;

        k1=k1.wrapping_mul(c1);
        k1=k1.rotate_left(31);
        k1=k1.wrapping_mul(c2);
        h1^=k1;
    }

    h1^=input.len() as u64;
    h2^=input.len() as u64;

    h1=h1.wrapping_add(h2);
    h2=h2.wrapping_add(h1);

    fn fmix64(mut k: u64) -> u64{
        k^=k>>33;
        k=k.wrapping_mul(0xff51afd7ed558ccdu64);
        k^=k>>33;
        k=k.wrapping_mul(0xc4ceb9fe1a85ec53u64);
        k^=k>>33;

        k
    }

    h1=fmix64(h1);
    h2=fmix64(h2);

    h1=h1.wrapping_add(h2);
    h2=h2.wrapping_add(h1);

    [h1,h2]
}

// copied from https://github.com/aappleby/smhasher/blob/master/src/MurmurHash3.cpp
pub fn murmur_hash3_32(input: &[u8], seed: u32) -> u32 {
    let nblocks=input.len()/4;
    let mut h1=seed as u32;
    let c1=0xcc9e2d51u32;
    let c2=0x1b873593u32;

    for chunk in input.chunks(4).rev().skip_while(|x| x.len()<4){ // for i in (0..nblocks).rev(){ let chunk=&input[i*4..i*4+4];
        assert!(chunk.len()==4);
        let mut k1=chunk.iter().enumerate().fold(0u32, |acc, (i, &e)| acc|(e as u32) << i);
        k1=k1.wrapping_mul(c1);
        k1=k1.rotate_left(15);
        k1=k1.wrapping_mul(c2);

        h1^=k1;
        h1=h1.rotate_left(13); 
        h1=(h1.wrapping_mul(5)).wrapping_add(0xe6546b64);
    }

    let tail=&input[nblocks*4..];
    assert!(tail.len()<4);

    let mut k1=0u32;

    if tail.len()>=3{
        k1^=(unsafe{*tail.get_unchecked(2)} as u32)<<16;
    }
    if tail.len()>=2{
        k1^=(unsafe{*tail.get_unchecked(1)} as u32)<<8;
    }
    if tail.len()>=1{
        k1^=(unsafe{*tail.get_unchecked(0)} as u32)<<0;

        k1=k1.wrapping_mul(c1);
        k1=k1.rotate_left(15);
        k1=k1.wrapping_mul(c2);
        h1^=k1;
    }

    h1^=input.len() as u32;

    fn fmix32(mut h:u32) -> u32{
        h^=h>>16;
        h=h.wrapping_mul(0x85ebca6b);
        h^=h>>13;
        h=h.wrapping_mul(0xc2b2ae35);
        h^=h>>16;

        h
    }

    h1=fmix32(h1);

    h1
}
