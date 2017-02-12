lazy_static! {
    pub static ref DEBUG_ENABLED: bool = {
        use std::env;
        env::var("NLL_DEBUG").is_ok()
    };
}

macro_rules! log {
    ($($t:tt)*) => {
        if *::log::DEBUG_ENABLED {
            println!($($t)*)
        }
    }
}
