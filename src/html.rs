extern crate google_classroom1 as classroom1;
use classroom1::{api::{ListAnnouncementsResponse, ListStudentSubmissionsResponse, ListCourseWorkResponse, ListCourseWorkMaterialResponse, ListTeachersResponse, ListTopicResponse, Material, Teacher}};
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
                if announcement.alternate_link.is_some() {
                    println!("Link to announcement {}", announcement.alternate_link.unwrap());
                };
                println!("announcement: {}", match announcement.text {
                    Some(text) => text,
                    None => "No text".to_string()
                });
                println!("time made: {:#?}", announcement.creation_time.unwrap());
                if announcement.scheduled_time.is_some() {
                    println!("time published: {}", announcement.scheduled_time.unwrap());
                }
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
                            if material.form.is_some() && material.form.clone().unwrap().form_url.is_some() {
                                let form_link = material.form.unwrap().form_url.unwrap();
                                println!("form: {}", form_link);
                            };
                            if material.drive_file.clone().is_some() && material.drive_file.clone().unwrap().drive_file.is_some() {
                                if material.drive_file.clone().unwrap().drive_file.unwrap().title.is_some() {
                                    println!("{}", material.drive_file.clone().unwrap().drive_file.unwrap().title.unwrap());
                                };
                                if material.drive_file.clone().unwrap().drive_file.unwrap().alternate_link.is_some() {
                                    println!("{}", material.drive_file.clone().unwrap().drive_file.unwrap().alternate_link.unwrap());
                                };
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
            println!("No announcements\n");
        }
        let course_work: (Response<Body>, ListCourseWorkResponse) = courses.course_work_list(&the_id).doit().await.unwrap();
        if course_work.1.course_work.is_some() {
            let course_work_list = course_work.1.course_work.unwrap();
            for course in course_work_list {
                if course.assignee_mode.is_none() {
                    if course.alternate_link.is_some() {
                        println!("Link to work {}", course.alternate_link.unwrap());
                    } else {
                        println!("No link");
                    }
                    println!("time made: {:#?}", course.creation_time.unwrap());
                    if course.scheduled_time.is_some() {
                        println!("time published: {}", course.scheduled_time.unwrap());
                    }
                    println!("Author id: {}", match course.creator_user_id {
                        Some(creator_user_id) => creator_user_id,
                        None => "Unknown Author id".to_string()
                    });
                    println!("last updated: {:#?}", match course.update_time {
                        Some(update_time) => update_time,
                        None => course.creation_time.unwrap()
                    });
                    if course.assignment.is_some() && course.assignment.clone().unwrap().student_work_folder.is_some() {
                        if course.assignment.clone().unwrap().student_work_folder.unwrap().alternate_link.is_some() {
                            println!("Work folder: {}", course.assignment.clone().unwrap().student_work_folder.unwrap().alternate_link.is_some());
                        };
                        if course.assignment.clone().unwrap().student_work_folder.unwrap().title.is_some() {
                            println!("Name of folder: {}", course.assignment.clone().unwrap().student_work_folder.unwrap().title.is_some());
                        };
                    }
                }
                println!("assignment: {}", match course.description {
                    Some(description) => description,
                    None => "None".to_string()
                });
                let mut date_due: String = "".to_string();
                if course.due_date.is_some() {
                    match course.due_date.clone().unwrap().month {
                        Some(month) => date_due.push_str(&month.to_string()),
                        None => ()
                    }
                    date_due.push_str("-");
                    match course.due_date.clone().unwrap().day {
                        Some(day) => date_due.push_str(&day.to_string()),
                        None => ()
                    }
                    date_due.push_str("-");
                    match course.due_date.clone().unwrap().year {
                        Some(year) => date_due.push_str(&year.to_string()),
                        None => ()
                    }
                }
                if course.due_time.is_some() {
                    date_due.push_str(" ");
                    match course.due_time.clone().unwrap().hours {
                        Some(hours) => date_due.push_str(&hours.to_string()),
                        None => ()
                    }
                    date_due.push_str(":");
                    match course.due_time.clone().unwrap().minutes {
                        Some(minutes) => date_due.push_str(&minutes.to_string()),
                        None => ()
                    }
                }
                println!("Due at {}", date_due);
                if course.grade_category.is_some() {
                    println!("{:#?}", course.grade_category.unwrap_or_default());
                }
                println!("{}", course.id.unwrap_or_default());
                match course.materials {
                    Some(materials) => {
                        for material in materials {
                            if material.form.is_some() && material.form.clone().unwrap().form_url.is_some() {
                                let form_link = material.form.unwrap().form_url.unwrap();
                                println!("form: {}", form_link);
                            };
                            if material.drive_file.clone().is_some() && material.drive_file.clone().unwrap().drive_file.is_some() {
                                if material.drive_file.clone().unwrap().drive_file.unwrap().title.is_some() {
                                    println!("{}", material.drive_file.clone().unwrap().drive_file.unwrap().title.unwrap());
                                };
                                if material.drive_file.clone().unwrap().drive_file.unwrap().alternate_link.is_some() {
                                    println!("{}", material.drive_file.clone().unwrap().drive_file.unwrap().alternate_link.unwrap());
                                };
                            }
                        }
                    }
                    None => println!("No mats")
                }
                if course.max_points.is_some() {
                    println!("Points: {}", course.max_points.unwrap().to_string());
                } else {
                    println!("Not graded");
                }
                if course.multiple_choice_question.is_some() {
                    for choice in course.multiple_choice_question.unwrap().choices.unwrap() {
                        println!("{}", choice);
                    }
                }
                if course.title.is_some() {
                    println!("{}", course.title.unwrap());
                }
                if course.topic_id.is_some() {
                    println!("Topic: {}", course.topic_id.unwrap());
                }
                if course.work_type.is_some() {
                    println!("work_type: {}", course.work_type.unwrap());
                }
                println!("last updated: {:#?}", match course.update_time {
                    Some(update_time) => update_time,
                    None => course.creation_time.unwrap()
                });
                println!(" ");
            }
        }
        let course_materials: (Response<Body>, ListCourseWorkMaterialResponse) = courses.course_work_materials_list(&the_id).doit().await.unwrap();
        if course_materials.1.course_work_material.is_some() {
            for course_mats in course_materials.1.course_work_material.unwrap() {
                if course_mats.assignee_mode.is_none() {
                    if course_mats.alternate_link.is_some() {
                        println!("Link to work {}", course_mats.alternate_link.unwrap());
                    } else {
                        println!("No link");
                    }
                    println!("time made: {:#?}", course_mats.creation_time.unwrap());
                    if course_mats.scheduled_time.is_some() {
                        println!("time published: {}", course_mats.scheduled_time.unwrap());
                    }
                    println!("Author id: {}", match course_mats.creator_user_id {
                        Some(creator_user_id) => creator_user_id,
                        None => "Unknown Author id".to_string()
                    });
                    println!("Material: {}", match course_mats.description {
                        Some(description) => description,
                        None => "None".to_string()
                    });
                    if course_mats.id.is_some() {
                        println!("Id: {}", course_mats.id.unwrap());
                    }
                    if course_mats.topic_id.is_some() {
                        println!("In Topic Id: {}", course_mats.topic_id.unwrap());
                    }
                    println!("last updated: {:#?}", match course_mats.update_time {
                        Some(update_time) => update_time,
                        None => course_mats.creation_time.unwrap()
                    });
                    println!(" ");
                }
            }
        }
        let teachers: (Response<Body>, ListTeachersResponse) = courses.teachers_list(&the_id).doit().await.unwrap();
        if teachers.1.teachers.is_some() {
            for teacher in teachers.1.teachers.unwrap() {
                if teacher.user_id.is_some() {
                    println!("User Id: {}", teacher.user_id.unwrap());
                }
                if teacher.profile.is_some() {
                    teacher_profile = "".to_string();
                    if teacher.profile.clone().unwrap().email_address.is_some() {
                        
                    }
                }
            }
        }
        //let topics: (Response<Body>, ListTopicResponse) = courses.topics_list(course_id: &the_id).doit().await*/
        //let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = courses.course_work_student_submissions_list(course_id: &the_id).doit().await.unwrap();

    }
}

//str - Stack allocated, not mutable (usually). have to know size at compile time.

//String - Heap allocated, can be mutable (with 1 referance only OR Rust mem lock like RWlock), can grow or shrink size.

//char - single character, including unicode, can be mutable