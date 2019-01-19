use hyper::Body;

pub trait Representation {
    fn body(self: Box<Self>) -> Body;
}

impl<B: Into<Body>> Representation for B {
    fn body(self: Box<Self>) -> Body {
        (*self).into()
    }
}
