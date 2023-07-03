extern crate google_classroom1 as classroom1;
use classroom1::{api::{ListAnnouncementsResponse, ListStudentSubmissionsResponse, ListCourseWorkResponse, ListCourseWorkMaterialResponse, ListTeachersResponse, ListTopicResponse, Material}};
use classroom1::{Classroom, hyper, hyper_rustls};
use google_classroom1::api::ListCoursesResponse;
use hyper::Body;
use hyper::Response;

#[tokio::main]
async fn main() {
    let secret = classroom1::oauth2::read_application_secret("credentials.json")
        .await
        .expect("client secret couldn't be read.");
    let auth = classroom1::oauth2::InstalledFlowAuthenticator::builder(
        secret,
        classroom1::oauth2::InstalledFlowReturnMethod::HTTPRedirect,
    )
    .persist_tokens_to_disk("tokens.json")
    .build()
    .await
    .expect("InstalledFlowAuthenticator failed to build");

    let hub = Classroom::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build()), auth);
    let courses = hub.courses();
    let response: (Response<Body>, ListCoursesResponse) = courses.list().page_size(100).doit().await.unwrap();

    for course in response.1.courses.unwrap() {
        let the_id = course.id.unwrap().clone();
        println!("Course: {}, {}", course.name.unwrap(), the_id);
        let course_announcements: (Response<Body>, ListAnnouncementsResponse) = courses.announcements_list(&the_id).doit().await.unwrap();
        if course_announcements.1.announcements.is_some() {
            for announcement in course_announcements.1.announcements.unwrap() {
                println!("announcement: {}", match announcement.text {
                    Some(text) => text,
                    None => "No text".to_string()
                });
                println!("time made: {:#?}", announcement.creation_time.unwrap());
                println!("Author id: {}", match announcement.creator_user_id {
                    Some(creator_user_id) => creator_user_id,
                    None => "Unknown Author id".to_string()
                });
                println!("id: {}", match announcement.id {
                    Some(id) => id,
                    None => "Unknown id".to_string()
                });
                match announcement.materials {
                    Some(materials) => {
                        for material in materials {
                            match material.form {
                                Some(forms) => {
                                    if forms.form_url.is_some() {
                                        println!("form: {}", forms.form_url.unwrap());
                                    }
                                }
                                None => ()
                            }
                            match material.drive_file {
                                Some(drive_file) => {
                                    match drive_file.drive_file {
                                        Some(drive_file) => {
                                            match drive_file.alternate_link {
                                                Some(alternate_link) => println!("{}", alternate_link),
                                                None => (),
                                            };
                                            match drive_file.title {
                                                Some(title) => println!("{}", title),
                                                None => (),
                                            };
                                        }
                                        None => ()
                                    }
                                }
                                None => ()
                            }
                        }
                    }
                    None => println!("No mats")
                }
                println!("last updated: {:#?}", match announcement.update_time {
                    Some(update_time) => update_time,
                    None => announcement.creation_time.unwrap()
                });
                println!("");
            }
        } else {
            println!("No announcements\n")
        }
        /*let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = courses.course_work_student_submissions_list(course_id: &the_id).doit().await.unwrap();
        let course_work: (Response<Body>, ListCourseWorkResponse) = courses.course_work_list(course_id: &the_id).doit().await.unwrap();
        let course_materials: (Response<Body>, ListCourseWorkMaterialResponse) = courses.course_work_materials_list(course_id: &the_id).doit().await.unwrap();
        let teachers: (Response<Body>, ListTeachersResponse) = courses.teachers_list(course_id: &the_id).doit().await.unwrap();
        let topics: (Response<Body>, ListTopicResponse) = courses.topics_list(course_id: &the_id).doit().await*/
        //println!("Courses: {:#?}", course_announcements);
    }
}

//str - Stack allocated, not mutable (usually). have to know size at compile time.

//String - Heap allocated, can be mutable (with 1 referance only OR Rust mem lock like RWlock), can grow or shrink size.

//char - single character, including unicode, can be mutable