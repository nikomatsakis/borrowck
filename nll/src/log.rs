macro_rules! log {
    ($($t:tt)*) => { if false { println!($($t)*) } }
}
