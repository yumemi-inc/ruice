use std::marker::PhantomData;
use std::sync::Arc;

use crate::{Resolve, Resolver, ServiceContainer, Services};

pub trait Construct<S = Self, C = ServiceContainer>: Send + Sync {
    fn construct(container: &C) -> Option<S>;
}

pub struct Constructor<S> {
    _phantom: PhantomData<fn() -> S>,
}

impl<S> Constructor<S> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for Constructor<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S, C> Resolve<S, C> for Constructor<S>
where
    S: Construct<S, C>,
{
    fn resolve(&self, container: &C) -> Option<Arc<S>> {
        Some(Arc::new(S::construct(container)?))
    }
}

pub trait ConstructServices: Services {
    fn construct<S>(&mut self)
    where
        S: Construct<S, Self> + 'static,
    {
        self.put(Resolver::<S, Self>::new(Constructor::<S>::new()));
    }
}

impl<C> ConstructServices for C where C: Services {}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::construct::{Construct, ConstructServices};
    use crate::singleton::SingletonServices;
    use crate::{ServiceContainer, Services};

    struct Foo {
        name: String,
    }

    struct Bar {
        foo: Arc<Foo>,
    }

    impl Bar {
        fn greet(&self) -> String {
            format!("Hello, {}!", self.foo.name)
        }
    }

    impl<C> Construct<Self, C> for Bar
    where
        C: Services,
    {
        fn construct(container: &C) -> Option<Self> {
            Some(Self {
                foo: container.get()?,
            })
        }
    }

    #[test]
    fn construct() {
        let mut container = ServiceContainer::default();

        // Bar is constructed from services in the container.
        // The instance will be initiated lazily, so we can set this earlier than Foo.
        container.construct::<Bar>();

        // Foo is shared as a singleton service.
        container.singleton(Foo {
            name: "Taro".to_string(),
        });

        let bar = container.get::<Bar>().unwrap();
        assert_eq!("Hello, Taro!".to_string(), bar.greet());
    }
}
