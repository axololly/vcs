#[macro_export]
macro_rules! unwrap {
    ($result:expr, $message:expr) => {{
        #[allow(unused_imports)]
        use eyre::{Context, ContextCompat};
        
        $result.wrap_err_with(|| format!($message))?
    }};

    ($result:expr, $message:expr, $($arg:tt)*) => {{
        #[allow(unused_imports)]
        use eyre::{Context, ContextCompat};
        
        $result.wrap_err_with(|| format!($message, $($arg)*))?
    }};
}