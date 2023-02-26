use std::{marker::PhantomData, num::NonZeroU64};

mod thread;
use nugc_derive::{Trace, generate_write_barriers};



// #[derive(Trace)]
// #[generate_write_barriers]
// struct Epicswag {
//     v: GcPtr<()>
// }




// fn epic(mut v: Epicswag, a: GcPtr<Epicswag>, b: GcPtr<()>) {
//     v.set_v(&(),a, b);
// }