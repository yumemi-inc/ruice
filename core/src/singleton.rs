use std::sync::Arc;

use crate::{Resolve, Services};

pub struct Singleton<S> {
    service: Arc<S>,
}

impl<S> Singleton<S> {
    pub fn new(service: S) -> Self {
        Self {
            service: Arc::new(service),
        }
    }
}

impl<S, C> Resolve<S, C> for Singleton<S>
where
    S: Send + Sync,
{
    fn resolve(&self, _container: &C) -> Option<Arc<S>> {
        Some(Arc::clone(&self.service))
    }
}

pub trait SingletonServices: Services {
    fn singleton<S>(&mut self, service: S)
    where
        S: Send + Sync + 'static,
    {
        self.put(Singleton::new(service));
    }
}

impl<C> SingletonServices for C where C: Services {}
