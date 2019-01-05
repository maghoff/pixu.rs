use super::media_type::MediaType;
use super::representation::Representation;

pub trait Resource {
    fn representations(self: Box<Self>) ->
        Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>;
}

impl<T: FnOnce() -> Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>> Resource for T {
    fn representations(self: Box<Self>)
        -> Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>
    {
        (*self)()
    }
}

impl Resource for Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)> {
    fn representations(self: Box<Self>)
        -> Vec<(MediaType, Box<dyn FnOnce() -> Box<dyn Representation>>)>
    {
        *self
    }
}
