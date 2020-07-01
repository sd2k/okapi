use super::OpenApiResponder;
use crate::{gen::OpenApiGenerator, util::*};
use okapi::openapi3::Responses;
use rocket::response::Responder;
use rocket_contrib::json::{Json, JsonValue}; // TODO json feature flag
use schemars::JsonSchema;
use serde::Serialize;
use std::fmt::Debug;
use std::result::Result as StdResult;

type Result = crate::Result<Responses>;

impl<'r, T: JsonSchema + Serialize + Send> OpenApiResponder<'r, 'static> for Json<T> {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        let mut responses = Responses::default();
        let schema = gen.json_schema::<T>();
        add_schema_response(&mut responses, 200, "application/json", schema)?;
        Ok(responses)
    }
}

impl<'r> OpenApiResponder<'r, 'static> for JsonValue {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        let mut responses = Responses::default();
        let schema = gen.schema_generator().schema_for_any();
        add_schema_response(&mut responses, 200, "application/json", schema.into())?;
        Ok(responses)
    }
}

impl<'r> OpenApiResponder<'r, 'static> for String {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        let mut responses = Responses::default();
        let schema = gen.json_schema::<String>();
        add_schema_response(&mut responses, 200, "text/plain", schema)?;
        Ok(responses)
    }
}

impl<'r, 'o: 'r> OpenApiResponder<'r, 'o> for &'o str {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        <String>::responses(gen)
    }
}

impl<'r> OpenApiResponder<'r, 'static> for Vec<u8> {
    fn responses(_: &mut OpenApiGenerator) -> Result {
        let mut responses = Responses::default();
        add_content_response(
            &mut responses,
            200,
            "application/octet-stream",
            Default::default(),
        )?;
        Ok(responses)
    }
}

impl<'r, 'o: 'r> OpenApiResponder<'r, 'o> for &'o [u8] {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        <Vec<u8>>::responses(gen)
    }
}

impl<'r> OpenApiResponder<'r, 'static> for () {
    fn responses(_: &mut OpenApiGenerator) -> Result {
        let mut responses = Responses::default();
        ensure_status_code_exists(&mut responses, 200);
        Ok(responses)
    }
}

impl<'r, T: OpenApiResponder<'r, 'static>> OpenApiResponder<'r, 'static> for Option<T> {
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        let mut responses = T::responses(gen)?;
        ensure_status_code_exists(&mut responses, 404);
        Ok(responses)
    }
}

macro_rules! status_responder {
    ($responder: ident, $status: literal) => {
        impl<'r, 'o: 'r, T: OpenApiResponder<'r, 'o> + 'r> OpenApiResponder<'r, 'o>
            for rocket::response::status::$responder<T>
        {
            fn responses(gen: &mut OpenApiGenerator) -> Result {
                let mut responses = T::responses(gen)?;
                set_status_code(&mut responses, $status)?;
                Ok(responses)
            }
        }
    };
}

status_responder!(Accepted, 202);
status_responder!(Created, 201);
status_responder!(BadRequest, 400);
status_responder!(NotFound, 404);
// These aren't present in Rocket's async branch for now.
// status_responder!(Unauthorized, 401);
// status_responder!(Forbidden, 403);
// status_responder!(Conflict, 409);

// impl OpenApiResponder<'_> for rocket::response::status::NoContent {
//     fn responses(_: &mut OpenApiGenerator) -> Result {
//         let mut responses = Responses::default();
//         set_status_code(&mut responses, 204)?;
//         Ok(responses)
//     }
// }

macro_rules! response_content_wrapper {
    ($responder: ident, $mime: literal) => {
        impl<'r, 'o: 'r, T: OpenApiResponder<'r, 'o> + 'r> OpenApiResponder<'r, 'o>
            for rocket::response::content::$responder<T>
        {
            fn responses(gen: &mut OpenApiGenerator) -> Result {
                let mut responses = T::responses(gen)?;
                set_content_type(&mut responses, $mime)?;
                Ok(responses)
            }
        }
    };
}

response_content_wrapper!(Css, "text/css");
response_content_wrapper!(Html, "text/html");
response_content_wrapper!(JavaScript, "application/javascript");
response_content_wrapper!(Json, "application/json");
response_content_wrapper!(MsgPack, "application/msgpack");
response_content_wrapper!(Plain, "text/plain");
response_content_wrapper!(Xml, "text/xml");

impl<'r, 'o: 'r, T: OpenApiResponder<'r, 'o>, E: Responder<'r, 'o> + Send> OpenApiResponder<'r, 'o>
    for StdResult<T, E>
{
    default fn responses(gen: &mut OpenApiGenerator) -> Result {
        let mut responses = T::responses(gen)?;
        ensure_status_code_exists(&mut responses, 500);
        Ok(responses)
    }
}

impl<'r, 'o: 'r, T: OpenApiResponder<'r, 'o>, E: OpenApiResponder<'r, 'o> + Debug>
    OpenApiResponder<'r, 'o> for StdResult<T, E>
{
    fn responses(gen: &mut OpenApiGenerator) -> Result {
        let ok_responses = T::responses(gen)?;
        let err_responses = E::responses(gen)?;
        produce_any_responses(ok_responses, err_responses)
    }
}
