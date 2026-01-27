
mod num_utils;
mod database;







fn main() {
    println!("Hello, world!");
    let a = 1855454745u128.to_le_bytes();
    let b = 1u32.to_le_bytes();
    //let c = u32::from_le_bytes(a[0..4].try_into().unwrap());
    println!("{:?}", a);
    println!("{:?}", b);

    let (c,r) = a.as_chunks::<4>();
    println!("{:?}", c);
    println!("{:?}", r);
    let a1 = 128u32;
    let e = a1.min(u32::MAX);
    println!("e = {}",e);


}


// Так. Нам нужно база данныъ с дипазонами всей фигни. И сервить её через tls или что-то типо того.



