use crate::Services;

pub trait Inject<C> {
    fn inject(&self, container: &mut C);
}

pub trait InjectServices: Services {
    fn inject<I>(&mut self, injector: I)
    where
        I: Inject<Self>,
    {
        injector.inject(self);
    }
}

impl<C> InjectServices for C where C: Services {}
