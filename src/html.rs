extern crate google_classroom1 as classroom1;
use classroom1::{Result, Error};
use std::default::Default;
use std::fs;
use crate::oauth2::InstalledFlowAuthenticator;
use classroom1::{Classroom, oauth2, hyper, hyper_rustls, chrono, FieldMask};
use google_classroom1::api::ListCoursesResponse;
use hyper::Body;
use hyper::Response;
use futures::future::join_all;
use google_classroom1::api::Course;

#[tokio::main]
async fn main() {
    let secret = classroom1::oauth2::read_application_secret("credentials.json")
        .await
        .expect("client secret couldn't be read.");
    let auth = classroom1::oauth2::InstalledFlowAuthenticator::builder(
        sec,
        classroom1::oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokens.json")
    .build()
    .await
    .expect("InstalledFlowAuthenticator failed to build");

    let mut hub = Classroom::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build()), auth);
    let courses = hub.courses();
    let (response, ListCoursesResponse { course_list: Some(courses), .. }): (Response<Body>, ListCoursesResponse) = (match courses.list().page_size().doit().await {
        Ok(response) => response,
        Err(error) => {
            eprintln!("Error: {}", error);
        }
    }) else {  };

    for course in course_list {
        println!("Courses: {}", (course.id.unwrap()));
        let (response, ListAnnouncementsResponse { announcements_list: Some(announcements), .. }): (Response<Body>, ListAnnouncementsResponse) = match ccourses.announcements_list(course.id.unwrap()).doit().await {
            Ok(response) => response,
            Err(error) => {
                eprintln!("Error: {}", error);
            }
        }
        let (response, ListAnnouncementsResponse { course_work_student: Some(courses), .. }): (Response<Body>, ListAnnouncementsResponse) = match courses.announcements_list(course.id.unwrap()).doit().await {
            Ok(response) => response,
            Err(error) => {
                eprintln!("Error: {}", error);
            }
        }
    }
    //let r = hub.courses().course_work_student_submissions_list(course.id.unwrap()).doit().await
    //let r = hub.courses().course_work_list(course.id.unwrap()).doit().await
    //let r = hub.courses().course_work_materials_list(course.id.unwrap()).doit().await
    //let r = hub.courses().teachers_list(course.id.unwrap()).doit().await
    //let r = hub.courses().topics_list(course.id.unwrap()).doit().await
}