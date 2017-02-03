macro_rules! log {
    ($($t:tt)*) => { if true { println!($($t)*) } }
}
