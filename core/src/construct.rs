use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::{AsyncResolve, AsyncResolver, AsyncServices, Resolve, ServiceContainer, Services};

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

#[async_trait]
pub trait AsyncConstruct<S = Self, C = ServiceContainer>: Send + Sync {
    async fn construct_async(container: &C) -> Option<S>;
}

pub struct AsyncConstructor<S> {
    _phantom: PhantomData<fn() -> S>,
}

impl<S> AsyncConstructor<S> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<S> Default for AsyncConstructor<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl<S, C> AsyncResolve<S, C> for AsyncConstructor<S>
where
    S: AsyncConstruct<S, C>,
    C: Sync,
{
    async fn async_resolve(&self, container: &C) -> Option<Arc<S>> {
        Some(Arc::new(S::construct_async(container).await?))
    }
}

pub trait ConstructServices: Services {
    fn construct<S>(&mut self)
    where
        S: Construct<S, Self> + 'static,
    {
        self.put(Constructor::<S>::new());
    }
}

impl<C> ConstructServices for C where C: Services {}

#[async_trait]
pub trait AsyncConstructServices: AsyncServices {
    async fn construct_async<S>(&mut self)
    where
        S: AsyncConstruct<S, Self> + 'static,
    {
        self.put_async(AsyncResolver::new(AsyncConstructor::<S>::new()));
    }
}

#[async_trait]
impl<C> AsyncConstructServices for C where C: AsyncServices {}

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
