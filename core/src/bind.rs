use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;

use crate::{
    AsyncResolve, AsyncResolver, AsyncServices, Resolve, Resolver, ServiceContainer, Services,
};

pub struct Bound<Interface>
where
    Interface: ?Sized,
{
    service: Arc<Interface>,
}

impl<Interface> From<Arc<Interface>> for Bound<Interface>
where
    Interface: ?Sized,
{
    fn from(value: Arc<Interface>) -> Self {
        Self { service: value }
    }
}

impl<Interface, C> Resolve<Interface, C> for Bound<Interface>
where
    Interface: ?Sized + Send + Sync,
{
    fn resolve(&self, _container: &C) -> Option<Arc<Interface>> {
        Some(Arc::clone(&self.service))
    }
}

pub struct BindBy<Interface, C = ServiceContainer>
where
    Interface: ?Sized + Send + Sync,
{
    #[allow(clippy::type_complexity)]
    f: Arc<dyn Fn(&C) -> Option<Arc<Interface>> + Send + Sync>,
}

impl<Interface, C, F> From<F> for BindBy<Interface, C>
where
    Interface: ?Sized + Send + Sync,
    F: (Fn(&C) -> Option<Arc<Interface>>) + Send + Sync + 'static,
{
    fn from(value: F) -> Self {
        Self { f: Arc::new(value) }
    }
}

impl<Interface, C> Resolve<Interface, C> for BindBy<Interface, C>
where
    Interface: ?Sized + Send + Sync,
{
    fn resolve(&self, container: &C) -> Option<Arc<Interface>> {
        (self.f)(container)
    }
}

pub struct AsyncBindBy<Interface, C = ServiceContainer>
where
    Interface: ?Sized + Send + Sync,
{
    #[allow(clippy::type_complexity)]
    f: Arc<
        dyn Fn(&C) -> Pin<Box<dyn Future<Output = Option<Arc<Interface>>> + Send>> + Send + Sync,
    >,
}

impl<Interface, C, F, Fut> From<F> for AsyncBindBy<Interface, C>
where
    Interface: ?Sized + Send + Sync,
    F: (Fn(&C) -> Fut) + Send + Sync + 'static,
    Fut: Future<Output = Option<Arc<Interface>>> + Send + 'static,
    C: Send + Sync,
{
    fn from(value: F) -> Self {
        Self {
            f: Arc::new(move |c| Box::pin(value(c))),
        }
    }
}

#[async_trait]
impl<Interface, C> AsyncResolve<Interface, C> for AsyncBindBy<Interface, C>
where
    Interface: ?Sized + Send + Sync,
    C: Send + Sync,
{
    async fn async_resolve(&self, container: &C) -> Option<Arc<Interface>> {
        (self.f)(container).await
    }
}

pub trait BindServices: Services {
    fn bind<Interface>(&mut self, service: Arc<Interface>)
    where
        Interface: ?Sized + Send + Sync + 'static,
    {
        self.put(Resolver::new(Bound::from(service)));
    }

    fn bind_by<Interface, F>(&mut self, f: F)
    where
        Interface: ?Sized + Send + Sync + 'static,
        F: (Fn(&Self) -> Option<Arc<Interface>>) + Send + Sync + 'static,
        Self: 'static,
    {
        self.put(Resolver::new(BindBy::from(f)))
    }
}

impl<C> BindServices for C where C: Services {}

pub trait AsyncBindServices: AsyncServices {
    fn bind_by_async<Interface, F, Fut>(&mut self, f: F)
    where
        Interface: ?Sized + Send + Sync + 'static,
        F: (Fn(&Self) -> Fut) + Send + Sync + 'static,
        Fut: Future<Output = Option<Arc<Interface>>> + Send + 'static,
        Self: Send + Sync + 'static,
    {
        self.put_async(AsyncResolver::new(AsyncBindBy::from(f)))
    }
}

impl<C> AsyncBindServices for C where C: AsyncServices {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceContainer;

    trait Greet: Send + Sync {
        fn greet(&self) -> String;
    }

    struct Greeter {
        name: String,
    }

    impl Greet for Greeter {
        fn greet(&self) -> String {
            format!("Hello, {}!", self.name)
        }
    }

    #[test]
    fn bind() {
        let mut container = ServiceContainer::default();

        // We can bind a service onto an interface.
        container.bind::<dyn Greet>(Arc::new(Greeter {
            name: "Taro".to_string(),
        }));

        // Now we can get the service by their interface instead of the actual type.
        let name_getter = container.get::<dyn Greet>().unwrap();
        assert_eq!("Hello, Taro!".to_string(), name_getter.greet());
    }

    #[test]
    fn bind_by() {
        let mut container = ServiceContainer::default();

        // We can bind a service onto an interface lazily.
        container.bind_by(|_| -> Option<Arc<dyn Greet>> {
            Some(Arc::new(Greeter {
                name: "Taro".to_string(),
            }))
        });

        // Now we can get the service by their interface instead of the actual type.
        let name_getter = container.get::<dyn Greet>().unwrap();
        assert_eq!("Hello, Taro!".to_string(), name_getter.greet());
    }

    #[tokio::test]
    async fn bind_by_async() {
        let mut container = ServiceContainer::default();

        // We can bind a service onto an interface lazily in an async context.
        container.bind_by_async(|_| async {
            Some(Arc::new(Greeter {
                name: "Taro".to_string(),
            }) as Arc<dyn Greet>)
        });

        // We can not the service in a non-async context.
        assert!(container.get::<dyn Greet>().is_none());

        // Now we can get the service by their interface instead of the actual type.
        let name_getter = container.get_async::<dyn Greet>().await.unwrap();
        assert_eq!("Hello, Taro!".to_string(), name_getter.greet());
    }
}
