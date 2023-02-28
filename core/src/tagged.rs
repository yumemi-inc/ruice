use std::sync::Arc;

use crate::Services;

#[derive(Debug)]
pub struct Tagged<Tag>
where
    Tag: ?Sized,
{
    services: Vec<Arc<Tag>>,
}

impl<Tag> Clone for Tagged<Tag>
where
    Tag: ?Sized,
{
    fn clone(&self) -> Self {
        Self {
            services: self.services.to_vec(),
        }
    }
}

impl<Tag> Default for Tagged<Tag>
where
    Tag: ?Sized,
{
    fn default() -> Self {
        Self { services: vec![] }
    }
}

unsafe impl<Tag> Send for Tagged<Tag> where Tag: ?Sized {}
unsafe impl<Tag> Sync for Tagged<Tag> where Tag: ?Sized {}

pub trait TaggedServices: Services {
    fn get_tagged<Tag>(&self) -> Vec<Arc<Tag>>
    where
        Tag: ?Sized + 'static,
    {
        self.get::<Tagged<Tag>>()
            .map(|t| t.services.iter().map(Arc::clone).collect())
            .unwrap_or_default()
    }

    fn put_tagged<Tag>(&mut self, service: Arc<Tag>)
    where
        Tag: ?Sized + 'static,
    {
        self.replace::<Tagged<Tag>, _>(|tagged| {
            let mut tagged = tagged.map(Tagged::<_>::clone).unwrap_or_default();
            tagged.services.push(service);
            tagged
        });
    }
}

impl<C> TaggedServices for C where C: Services {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ServiceContainer;

    trait Greet: Send + Sync {
        fn greet(&self) -> String;
    }

    type GreeterTag = dyn Greet;

    struct FooGreeter;

    impl Greet for FooGreeter {
        fn greet(&self) -> String {
            "Hello from Foo!".to_string()
        }
    }

    struct BarGreeter;

    impl Greet for BarGreeter {
        fn greet(&self) -> String {
            "Hello from Bar!".to_string()
        }
    }

    #[test]
    fn tag() {
        let mut container = ServiceContainer::default();

        container.put_tagged::<GreeterTag>(Arc::new(FooGreeter));
        container.put_tagged::<GreeterTag>(Arc::new(BarGreeter));

        let greetings = container
            .get_tagged::<GreeterTag>()
            .into_iter()
            .map(|g| g.greet())
            .collect::<Vec<_>>();

        assert_eq!(
            vec!["Hello from Foo!".to_string(), "Hello from Bar!".to_string()],
            greetings,
        )
    }
}
