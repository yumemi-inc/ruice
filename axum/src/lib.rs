use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use async_trait::async_trait;
use axum::extract::rejection::ExtensionRejection;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Extension;

use ruice::{AsyncServices, ServiceContainer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Service container is not available in this context: {0}")]
    ServiceContainerNotAvailable(#[from] ExtensionRejection),

    #[error("Could not find the service in the container, or could not resolve the service.")]
    ServiceNotFound,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("{}", self)).into_response()
    }
}

/// Inject enables you to retrieve DI components from the controllers.
///
/// ```
/// use ruice_axum::Inject;
/// use ruice::ServiceContainer;
///
/// # trait Database {}
/// async fn get_foo(db: Inject<dyn Database>) {
///     // do something with db
/// }
/// ```
pub struct Inject<I, C = ServiceContainer>
where
    I: ?Sized,
    C: AsyncServices,
{
    interface: Arc<I>,
    _phantom: PhantomData<fn() -> C>,
}

#[async_trait]
impl<I, C, B> FromRequestParts<B> for Inject<I, C>
where
    I: ?Sized + Send + Sync + 'static,
    C: AsyncServices + 'static,
    B: Send + Sync,
{
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &B) -> Result<Self, Self::Rejection> {
        let Extension(services): Extension<Arc<C>> =
            Extension::from_request_parts(parts, state).await?;

        Ok(Inject {
            interface: services.get_async().await.ok_or(Error::ServiceNotFound)?,
            _phantom: PhantomData,
        })
    }
}

impl<I, C> Deref for Inject<I, C>
where
    I: ?Sized + Send + Sync,
    C: AsyncServices,
{
    type Target = I;

    fn deref(&self) -> &Self::Target {
        self.interface.as_ref()
    }
}
