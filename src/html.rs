extern crate google_classroom1 as classroom1;
use classroom1::api::{ListAnnouncementsResponse, ListCoursesResponse, ListCourseWorkResponse, ListCourseWorkMaterialResponse, ListTeachersResponse, ListTopicResponse, self};
use classroom1::{Classroom, hyper, hyper_rustls};
use hyper::Body;
use hyper::Response;
use serde_json::Value;
use tera::Tera;
use tera::Context;
use tokio::task;
use std::{fs::File, io::Write, collections::HashMap};
use std::sync::{Arc, Mutex};
use tokio::runtime::Builder;

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
    let _scopes = vec![
        classroom1::api::Scope::AnnouncementReadonly,
        classroom1::api::Scope::CourseReadonly,
        classroom1::api::Scope::CourseworkMeReadonly,
        classroom1::api::Scope::CourseworkmaterialReadonly,
        classroom1::api::Scope::ProfileEmail,
        classroom1::api::Scope::ProfilePhoto,
        classroom1::api::Scope::RosterReadonly,
        classroom1::api::Scope::RosterReadonly,
        classroom1::api::Scope::StudentSubmissionStudentReadonly,
        classroom1::api::Scope::TopicReadonly,
    ];
    match auth.token(&_scopes).await {
        Ok(_token) => (),
        Err(e) => println!("error: {:?}", e),
    }

    let hub = Classroom::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build()), auth);
    let courses = hub.courses();
    let response: (Response<Body>, ListCoursesResponse) = courses.list().page_size(100).doit().await.unwrap();

    let mut tera = Tera::new("../templates/**/*.html").unwrap();
    let mut context = Context::new();
    let mut buffer = Vec::new();
    let course_list = response.1.courses.clone().unwrap();
    tera.add_template_file("templates/courses.html", Some("course_list")).unwrap();
    context.insert("courses", &course_list);
    tera.render_to("course_list", &context, &mut buffer).unwrap();
    let mut file = File::create("html/courses.html").expect("Failed to create file");

    let hub_arc = Arc::new(Mutex::new(hub.clone()));

    tera.register_function("getusername", move |args: &HashMap<String, Value>| {
        if let Some(id) = args.get("id").and_then(|v| v.as_str()) {
            let hub_mutex = hub_arc.lock().unwrap(); // Acquire the lock to access hub
    
            let user_profile = task::block_in_place(|| {
                let runtime = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async move {
                    hub_mutex.user_profiles().get(id).doit().await
                })
            });
            match user_profile {
                Ok(profile) => {
                    let name = profile.1.name.unwrap().full_name.unwrap_or_else(|| "None".to_string());
                    let name_value: Value = Value::String(name);
                    Ok(name_value)
                }
                Err(_) => {
                    let name = "None".to_string();
                    let name_value: Value = Value::String(name);
                    Ok(name_value)
                },
            }
        } else {
            Err(tera::Error::msg("No 'id' argument provided"))
        }
    });
    
    
    file.write_all(&buffer).expect("Failed to write to file");
    buffer = Vec::new();
    for course in response.1.courses.unwrap() {
        let the_id = course.clone().id.unwrap();
        println!("Course: {}, {}", course.name.clone().unwrap(), the_id);
        tera.add_template_file("templates/course.html", Some("course")).unwrap();
        context.insert("course", &course);
        let course_announcements: (Response<Body>, ListAnnouncementsResponse) = courses.announcements_list(&the_id).doit().await.unwrap();
        if course_announcements.1.announcements.is_some() {
            context.insert("course_announcements", &course_announcements.1.announcements.clone().unwrap());
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
            context.insert("teachers", &teachers.1.teachers.clone().unwrap());
        }
        let topics: (Response<Body>, ListTopicResponse) = courses.topics_list(&the_id).doit().await.unwrap();
        if topics.1.topic.is_some() {
            for topic in topics.1.topic.unwrap() {
                if topic.topic_id.is_some() {
                    println!("Topic Id: {}", topic.topic_id.unwrap());
                }
                if topic.name.is_some() {
                    println!("{}", topic.name.unwrap());
                }
                if topic.update_time.is_some() {
                    println!("last updated: {:#?}", topic.update_time);
                }
                println!("");
            }
        }
        //let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = courses.course_work_student_submissions_list(course_id: &the_id).doit().await.unwrap();
        println!("{:#?}", &context);
        tera.render_to("course", &context, &mut buffer).unwrap();
        let mut file = File::create(format!("html/courses/{}.html", the_id)).expect("Failed to create file");
        file.write_all(&buffer).expect("Failed to write to file");
        buffer = Vec::new();
    }
}

//str - Stack allocated, not mutable (usually). have to know size at compile time.

//String - Heap allocated, can be mutable (with 1 referance only OR Rust mem lock like RWlock), can grow or shrink size.

//char - single character, including unicode, can be mutable