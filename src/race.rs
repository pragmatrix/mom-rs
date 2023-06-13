#[macro_export]
macro_rules! race {
    ($($fut:ident),+ $(,)?) => {{
        let mut out = None;
        use futures::pin_mut;

        // Pin all the futures
        $(pin_mut!($fut);)+

        // Create the select! block
        tokio::select! {
            $(r = $fut => {
                out = Some(r);
            })+
        }
        out.unwrap()
    }};
}
