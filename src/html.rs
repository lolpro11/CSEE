extern crate google_classroom1 as classroom1;
use classroom1::api::{ListAnnouncementsResponse, ListCoursesResponse, ListCourseWorkResponse, ListCourseWorkMaterialResponse, ListTeachersResponse, ListTopicResponse};
use classroom1::{Classroom, hyper, hyper_rustls};
use hyper::Body;
use hyper::Response;
use oauth2::Scope;
use serde_json::Value;
use tera::Tera;
use tera::Context;
use tokio::task;
use std::time::{Instant, Duration};
use std::{fs::File, io::Write, collections::HashMap};
use std::sync::{Arc, Mutex, mpsc};
use tokio::runtime::{Builder, Runtime};

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
        Scope::new("https://www.googleapis.com/auth/drive.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.announcements.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.courses.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.coursework.students.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.coursework.me.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.courseworkmaterials.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.rosters.readonly".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.profile.emails".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.profile.photos".to_string()),
        Scope::new("https://www.googleapis.com/auth/classroom.topics.readonly".to_string()),
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
            let hub_mutex = &hub_arc.lock().unwrap(); // Acquire the lock to access hub
    
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

    async fn check_url(url: String) -> bool {
        match reqwest::get(&url).await {
            Ok(response) => response.status() == reqwest::StatusCode::OK,
            Err(_) => false,
        }
    }
    
    // Register the Tera filter function
    tera.register_function("url_ok", move |args: &HashMap<String, Value>| {
        if let Some(url) = args.get("url").and_then(|v| v.as_str()) {
            let url = url.to_string(); // Clone the URL for async closure
    
            // Create a channel for communication between threads
            let (sender, receiver) = mpsc::channel();
    
            // Spawn a new thread to run the async block
            std::thread::spawn(move || {
                let runtime = Runtime::new().unwrap();
                let result = runtime.block_on(async move {
                    check_url(url).await
                });
                let _ = sender.send(result);
            });
    
            // Wait for the result from the spawned thread
            let result = receiver.recv().unwrap();
    
            Ok(Value::Bool(result))
        } else {
            Err(tera::Error::msg("No 'url' argument provided"))
        }
    });

    /*tera.register_function("to_utc", move |args: &HashMap<String, Value>| {
        if let Some(date) = args.get("date") {
            let utc_date = "".to_string();
            match date.year {
                Some(year) => date_due.push_str(&year.to_string()),
                None => Err(tera::Error::msg("No 'date.month' argument"))
            }
            date_due.push_str("-");
            match date.month {
                Some(month) => date_due.push_str(&month.to_string()),
                None => Err(tera::Error::msg("No 'date.month' argument"))
            }
            date_due.push_str("-");
            match date.day {
                Some(day) => date_due.push_str(&day.to_string()),
                None => Err(tera::Error::msg("No 'date.day' argument"))
            }
    
            Ok(Value::Bool(result))
        } else {
            Err(tera::Error::msg("No 'date or time' argument provided"))
        }
    });*/

    file.write_all(&buffer).expect("Failed to write to file");
    buffer = Vec::new();
    let mut total_duration = Duration::new(0, 0);
    let num_iterations = response.1.courses.clone().unwrap().len();
    for course in response.1.courses.unwrap() {
        let start_time = Instant::now();
        context = Context::new();
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
            context.insert("coursework", &course_work.1.course_work.clone().unwrap());
        }
        let course_materials: (Response<Body>, ListCourseWorkMaterialResponse) = courses.course_work_materials_list(&the_id).doit().await.unwrap();
        if course_materials.1.course_work_material.is_some() {
            context.insert("course_materials", &course_materials.1.course_work_material.clone().unwrap());
        }
        let teachers: (Response<Body>, ListTeachersResponse) = courses.teachers_list(&the_id).doit().await.unwrap();
        if teachers.1.teachers.is_some() {
            context.insert("teachers", &teachers.1.teachers.clone().unwrap());
        }
        let topics: (Response<Body>, ListTopicResponse) = courses.topics_list(&the_id).doit().await.unwrap();
        if topics.1.topic.is_some() {
            context.insert("topics", &topics.1.topic.clone().unwrap());
        }
        //let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = courses.course_work_student_submissions_list(course_id: &the_id).doit().await.unwrap();
        //println!("{:#?}", &context);
        tera.render_to("course", &context, &mut buffer).unwrap();
        let mut file = File::create(format!("html/courses/{}.html", the_id)).expect("Failed to create file");
        file.write_all(&buffer).expect("Failed to write to file");
        buffer = Vec::new();
        let end_time = Instant::now();
        let iteration_duration = end_time - start_time;
        println!("Render Time: {:?}", iteration_duration);
        total_duration += iteration_duration;
    }
    let average_duration = total_duration / num_iterations as u32;
    println!("Average time per iteration: {:?}", average_duration);
}

//str - Stack allocated, not mutable (usually). have to know size at compile time.

//String - Heap allocated, can be mutable (with 1 referance only OR Rust mem lock like RWlock), can grow or shrink size.

//char - single character, including unicode, can be mutable