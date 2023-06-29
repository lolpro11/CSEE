mod classroom_v1_types;
use classroom_v1_types as cr;
use env_logger;
//mod drive_v3_types;
//use drive_v3_types as drive;
use async_google_apis_common as common;
use crate::classroom_v1_types::ListCoursesResponse;
use crate::classroom_v1_types::ListAnnouncementsResponse;
use crate::classroom_v1_types::ListCourseWorkResponse;
use crate::classroom_v1_types::ListCourseWorkMaterialResponse;
use crate::classroom_v1_types::ListTopicResponse;
use crate::classroom_v1_types::ListTeachersResponse;

//use std::path::Path;
use std::sync::Arc;

/// Create a new HTTPS client.
fn https_client() -> common::TlsClient {
    let conn = hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build();
    let cl = hyper::Client::builder().build(conn);
    cl
}
async fn list_courses(cl: &cr::CoursesService) -> std::result::Result<ListCoursesResponse, async_google_apis_common::Error> {
    let params = cr::CoursesListParams {
        page_size: Some(1),
        ..cr::CoursesListParams::default()
    };
    cl.list(&params).await
}

async fn announcements(cl: &cr::CoursesAnnouncementsService, id: &str) -> std::result::Result<ListAnnouncementsResponse, async_google_apis_common::Error> {
    let params = cr::CoursesAnnouncementsListParams {
        course_id: id.to_owned(),
        ..cr::CoursesAnnouncementsListParams::default()
    };
    cl.list(&params).await
}

async fn assignments(cl: &cr::CoursesCourseWorkService, id: &str) -> std::result::Result<ListCourseWorkResponse, async_google_apis_common::Error> {
    let params = cr::CoursesCourseWorkListParams {
        course_id: id.to_owned(),
        ..cr::CoursesCourseWorkListParams::default()
    };
    cl.list(&params).await
}

async fn materials(cl: &cr::CoursesCourseWorkMaterialsService, id: &str) -> std::result::Result<ListCourseWorkMaterialResponse, async_google_apis_common::Error> {
    let params = cr::CoursesCourseWorkMaterialsListParams {
        course_id: id.to_owned(),
        ..cr::CoursesCourseWorkMaterialsListParams::default()
    };
    cl.list(&params).await
}

async fn topics(cl: &cr::CoursesTopicsService, id: &str) -> std::result::Result<ListTopicResponse, async_google_apis_common::Error> {
    let params = cr::CoursesTopicsListParams {
        course_id: id.to_owned(),
        ..cr::CoursesTopicsListParams::default()
    };
    cl.list(&params).await
}

async fn teachers(cl: &cr::CoursesTeachersService, id: &str) -> std::result::Result<ListTeachersResponse, async_google_apis_common::Error> {
    let params = cr::CoursesTeachersListParams {
        course_id: id.to_owned(),
        ..cr::CoursesTeachersListParams::default()
    };
    cl.list(&params).await
}

fn process_result(result: Result<ListCoursesResponse, async_google_apis_common::Error>) -> Vec<(Option<String>, Option<String>)>{
    let mut course_info: Vec<(Option<String>, Option<String>)> = Vec::new();
    match result {
        Ok(response) => {
            match response.courses {
                Some(courses) => {
                    for course in courses {
                        let id = course.id;
                        let name = format!("{}-{}", course.name.expect("Oof").to_string(), course.section.expect("Oof").to_string());
                        
                        course_info.push((id, Some(name)));
                    }
                }
                None => {
                    println!("No courses available.");
                }
            }
        }
        Err(error) => {
            println!("Error: {:?}", error);
        }
    }
    course_info
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

    let _scopes = vec![
        cr::ClassroomScopes::ClassroomAnnouncementsReadonly,
        cr::ClassroomScopes::ClassroomCoursesReadonly,
        cr::ClassroomScopes::ClassroomCourseworkMeReadonly,
        cr::ClassroomScopes::ClassroomCourseworkStudentsReadonly,
        cr::ClassroomScopes::ClassroomCourseworkmaterialsReadonly,
        //cr::ClassroomScopes::ClassroomGuardianlinksMeReadonly,
        //cr::ClassroomScopes::ClassroomGuardianlinksStudentsReadonly,
        cr::ClassroomScopes::ClassroomRostersReadonly,
        cr::ClassroomScopes::ClassroomStudentSubmissionsMeReadonly,
        cr::ClassroomScopes::ClassroomStudentSubmissionsStudentsReadonly,
        cr::ClassroomScopes::ClassroomTopicsReadonly,
    ];
    match auth.token(&_scopes).await {
        Ok(_token) => (),
        Err(e) => println!("error: {:?}", e),
    }
    let shared_auth = Arc::new(auth);
    let cl = cr::CoursesService::new(https.clone(), shared_auth.clone());
    let result = process_result(list_courses(&cl).await);
    for (id, name) in result {
        match (id, name) {
            (Some(course_id), Some(course_name)) => {
                    let cl = cr::CoursesAnnouncementsService::new(https.clone(), shared_auth.clone());
                    let class_announcements = announcements(&cl, &course_id).await;

                    let cl = cr::CoursesCourseWorkMaterialsService::new(https.clone(), shared_auth.clone());
                    let class_materials = materials(&cl, &course_id).await;

                    let cl = cr::CoursesCourseWorkService::new(https.clone(), shared_auth.clone());
                    let class_assignments = assignments(&cl, &course_id).await;

                    let cl = cr::CoursesTopicsService::new(https.clone(), shared_auth.clone());
                    let class_topics = topics(&cl, &course_id).await;

                    let cl = cr::CoursesTeachersService::new(https.clone(), shared_auth.clone());
                    let class_teachers = teachers(&cl, &course_id).await;
                    
                    println!("{}: {:#?} {:#?} {:#?} {:#?} {:#?}", course_name, class_announcements, class_materials, class_assignments, class_topics, class_teachers);
                }
            (Some(_course_id), None) => (),
            (None, Some(_course_name)) => (),
            (None, None) => (),
        }
    }
}