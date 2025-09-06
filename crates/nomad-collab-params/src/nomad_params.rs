/// The [`Params`](collab_server::Params) used by the Collab server deployed at
/// `collab.nomad.foo`.
pub struct NomadParams;

impl collab_server::Params for NomadParams {
    const MAX_FRAME_LEN: u32 = 2048;

    type AuthenticateInfos = auth_types::AccessToken;
    type AuthenticateError = crate::AuthError;
    type SessionId = ulid::Ulid;
}
