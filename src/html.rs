mod classroom_v1_types;
use classroom_v1_types as cr;
use env_logger;
mod drive_v3_types;
use drive_v3_types as drive;
use async_google_apis_common as common;
use crate::classroom_v1_types::ListCoursesResponse;

use std::path::Path;
use std::sync::Arc;

/// Create a new HTTPS client.
fn https_client() -> common::TlsClient {
    let conn = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build();
    let cl = hyper::Client::builder().build(conn);
    cl
}
async fn list_courses(cl: &cr::CoursesService) -> std::result::Result<ListCoursesResponse, async_google_apis_common::Error> {
    let mut params = cr::CoursesListParams {
        page_size: Some(50),
        ..cr::CoursesListParams::default()
    };
    cl.list(&params).await
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let https = https_client();
    let sec = common::yup_oauth2::read_application_secret("credentials.json")
        .await
        .expect("client secret couldn't be read.");
    let auth = common::yup_oauth2::InstalledFlowAuthenticator::builder(
        sec,
        common::yup_oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokens.json")
    .hyper_client(https.clone())
    .build()
    .await
    .expect("InstalledFlowAuthenticator failed to build");

    let scopes = vec![
        cr::ClassroomScopes::ClassroomCourses,
        cr::ClassroomScopes::ClassroomAnnouncementsReadonly,
        cr::ClassroomScopes::ClassroomStudentSubmissionsStudentsReadonly,
    ];
    let mut cl = cr::CoursesService::new(https, Arc::new(auth));
    let result = list_courses(&cl).await;
    println!("{:#?}", result);

}