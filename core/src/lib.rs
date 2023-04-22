//! Dependency injection functionality.

pub mod bind;
pub mod construct;
pub mod inject;
pub mod singleton;
pub mod tagged;

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

pub use bind::{BindServices, Bound};
pub use construct::{Construct, ConstructServices, Constructor};
pub use inject::{Inject, InjectServices};
pub use singleton::{Singleton, SingletonServices};
pub use tagged::{Tagged, TaggedServices};

// Snippet: https://github.com/AzureMarker/shaku/blob/0be60063f093b164631853be72eb099813502172/shaku/src/trait_alias.rs
// Adapted from https://stackoverflow.com/a/30293051/3267834
// FIXME: Use real trait aliases when they are stabilized:
//        https://github.com/rust-lang/rust/issues/41517
macro_rules! trait_alias {
    ($(#[$attributes:meta])* $visibility:vis $name:ident = $base1:ident $(+ $base2:ident)*) => {
        $(#[$attributes])*
        $visibility trait $name: $base1 $(+ $base2)* { }
        impl<T: $base1 $(+ $base2)*> $name for T { }
    };
}

trait_alias! {
    pub Interface = Send + Sync
}

pub trait Resolve<S, C = ServiceContainer>: Send + Sync
where
    S: ?Sized,
{
    fn resolve(&self, container: &C) -> Option<Arc<S>>;
}

struct Resolver<S, C = ServiceContainer>
where
    S: ?Sized,
{
    resolve: Arc<dyn Resolve<S, C>>,
}

impl<S, C> Resolver<S, C>
where
    S: ?Sized,
{
    fn new<R>(resolve: R) -> Self
    where
        R: Resolve<S, C> + 'static,
    {
        Self {
            resolve: Arc::new(resolve),
        }
    }

    fn as_inner(&self) -> &dyn Resolve<S, C> {
        self.resolve.as_ref()
    }
}

#[async_trait]
pub trait AsyncResolve<S, C = ServiceContainer>: Send + Sync
where
    S: ?Sized,
{
    async fn async_resolve(&self, container: &C) -> Option<Arc<S>>;
}

pub struct AsyncResolver<S, C = ServiceContainer>
where
    S: ?Sized,
{
    resolve: Arc<dyn AsyncResolve<S, C>>,
}

impl<S, C> AsyncResolver<S, C>
where
    S: ?Sized,
{
    pub fn new<R>(resolve: R) -> Self
    where
        R: AsyncResolve<S, C> + 'static,
    {
        Self {
            resolve: Arc::new(resolve),
        }
    }

    pub fn as_inner(&self) -> &dyn AsyncResolve<S, C> {
        self.resolve.as_ref()
    }
}

pub trait Services: Sized + Send + Sync {
    fn get<S>(&self) -> Option<Arc<S>>
    where
        S: ?Sized + Send + Sync + 'static;

    fn put<S, R>(&mut self, resolver: R)
    where
        S: ?Sized + Send + Sync + 'static,
        R: Resolve<S, Self> + 'static;

    fn replace<S, F>(&mut self, f: F)
    where
        S: Send + Sync + 'static,
        F: FnOnce(Option<&S>) -> S,
    {
        self.put(Singleton::new(f(self.get::<S>().as_deref())));
    }
}

#[async_trait]
pub trait AsyncServices: Sized + Send + Sync {
    async fn get_async<S>(&self) -> Option<Arc<S>>
    where
        S: ?Sized + Send + Sync + 'static;

    fn put_async<S>(&mut self, resolver: AsyncResolver<S, Self>)
    where
        S: ?Sized + Send + Sync + 'static;
}

type ServiceId = TypeId;

#[derive(Debug, Clone, Default)]
pub struct ServiceContainer {
    services: HashMap<ServiceId, Arc<dyn Any + Send + Sync>>,
}

impl Services for ServiceContainer {
    fn get<S>(&self) -> Option<Arc<S>>
    where
        S: ?Sized + Send + Sync + 'static,
    {
        self.services
            .get(&TypeId::of::<S>())
            .and_then(|r| r.downcast_ref::<Resolver<S>>())
            .and_then(|r| r.as_inner().resolve(self))
    }

    fn put<S, R>(&mut self, resolver: R)
    where
        S: ?Sized + Send + Sync + 'static,
        R: Resolve<S, Self> + 'static,
    {
        self.services
            .insert(TypeId::of::<S>(), Arc::new(Resolver::new(resolver)));
    }
}

#[async_trait]
impl AsyncServices for ServiceContainer {
    async fn get_async<S>(&self) -> Option<Arc<S>>
    where
        S: ?Sized + Send + Sync + 'static,
    {
        let resolved = match self
            .services
            .get(&TypeId::of::<S>())
            .and_then(|r| r.downcast_ref::<AsyncResolver<S>>())
        {
            Some(r) => r.as_inner().async_resolve(self).await,
            _ => None,
        };

        match resolved {
            Some(s) => Some(s),
            _ => self.get(),
        }
    }

    fn put_async<S>(&mut self, resolver: AsyncResolver<S>)
    where
        S: ?Sized + Send + Sync + 'static,
    {
        self.services.insert(TypeId::of::<S>(), Arc::new(resolver));
    }
}

#[macro_export]
macro_rules! alias {
    ($int: ty, $act: ty $(,)?) => {
        |c| c.get::<$act>().map(|s| s as ::std::sync::Arc<$int>)
    };
}
