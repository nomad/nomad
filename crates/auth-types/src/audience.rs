/// TODO: docs.
pub trait Audience {
    /// Sets the audience-related fields on the given
    /// [`jsonwebtoken::Validation`].
    fn set_audience(validation: &mut jsonwebtoken::Validation);
}

/// TODO: docs.
pub struct Client;

impl Audience for Client {
    fn set_audience(validation: &mut jsonwebtoken::Validation) {
        validation.validate_aud = false;
    }
}

/// TODO: docs.
pub struct CollabServer;

impl CollabServer {
    /// TODO: docs.
    pub const AUDIENCE: &'static str = "collab.nomad.foo";
}

impl Audience for CollabServer {
    fn set_audience(validation: &mut jsonwebtoken::Validation) {
        validation.set_audience(&[Self::AUDIENCE]);
    }
}
