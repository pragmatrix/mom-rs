macro_rules! race {
    ($($fut:ident),+ $(,)?) => {
        {
            use futures::future::Future;
            use futures::pin_mut;

            // Pin all the futures
            $(pin_mut!($fut);)+

            // Create the select! block
            futures::select! {
                $($fut_res = $fut => {
                    return $fut_res;
                })+
            }
        }
    };
}
