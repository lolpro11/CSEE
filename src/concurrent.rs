use std::time::{Duration, Instant};
extern crate google_classroom1 as classroom1;
use classroom1::api::ListCoursesResponse;
use classroom1::hyper_rustls::HttpsConnector;
use classroom1::{Classroom, hyper, hyper_rustls};
use futures::StreamExt;
use hyper::Body;
use hyper::Response;
use hyper::client::HttpConnector;
use oauth2::Scope;
use serde_json::Value;
use tera::Tera;
use tera::Context;
use tokio::task;
use std::{fs::File, io::Write, collections::HashMap};
use std::sync::{Arc, Mutex, mpsc};
use tokio::runtime::{Builder, Runtime};

#[derive(Clone)]
struct Args {
    id: Option<String>,
    name: Option<String>,
    hub: Option<Classroom<HttpsConnector<HttpConnector>>>,
    tera: Option<Tera>,
}

#[tokio::main]
async fn main() {

    let threadcount = 25;
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
    tera.add_template_file("templates/course.html", Some("course")).unwrap();
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
    
    file.write_all(&buffer).expect("Failed to write to file");
    let mut total_duration = Duration::new(0, 0);

    let mut reqquery_vec: Vec<Args> = Vec::new();
    for course in response.1.courses.unwrap() {
        let course_content = Args {
            id: Some(course.clone().id.unwrap()),
            name: Some(course.name.clone().unwrap_or_default()),
            hub: Some(hub.clone()),
            tera: Some(tera.clone()),
        };
        reqquery_vec.push(course_content);
    }

    
    loop {
        let lastloop = Instant::now();
        let fetches = futures::stream::iter(reqquery_vec.clone().into_iter().map(|course| {
            async move {
                let start_time = Instant::now();
                let mut buffer = Vec::new();
                let mut context = Context::new();
                let string_id = course.clone().id.clone().unwrap();
                let id = string_id.as_str();
                println!("Pulling Data From {}", course.name.clone().unwrap());
                let course_announcements = Some(course.hub.clone().unwrap().courses().announcements_list(&id).doit().await.unwrap().1.announcements.clone().unwrap_or_default());
                let course_work = Some(course.hub.clone().unwrap().courses().course_work_list(&id).doit().await.unwrap().1.course_work.clone().unwrap_or_default());
                let course_materials = Some(course.hub.clone().unwrap().courses().course_work_materials_list(&id).doit().await.unwrap().1.course_work_material.clone().unwrap_or_default());
                let name = Some(course.name.clone().unwrap_or_default());
                let teachers = Some(course.hub.clone().unwrap().courses().teachers_list(&id).doit().await.unwrap().1.teachers.clone().unwrap_or_default());
                let topics = Some(course.hub.clone().unwrap().courses().topics_list(&id).doit().await.unwrap().1.topic.clone().unwrap_or_default());
                println!("Took {:?}", start_time.elapsed());
                println!("Course: {}, {}", name.clone().unwrap(), id.clone());
                context.insert("name", &name.clone().unwrap());
                if course_announcements.is_some() {
                    context.insert("course_announcements", &course_announcements.clone().unwrap());
                }
                if course_work.is_some() {
                    context.insert("coursework", &course_work.clone().unwrap());
                }
                if course_materials.is_some() {
                    context.insert("course_materials", &course_materials.clone().unwrap());
                }
                if teachers.is_some() {
                    context.insert("teachers", &teachers.clone().unwrap());
                }
                if topics.is_some() {
                    context.insert("topics", &topics.clone().unwrap());
                }
                //let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = course.hub.clone().courses.unwrap().course_work_student_submissions_list(course_id: &id).doit().await.unwrap();
                //println!("{:#?}", &context);
                course.tera.unwrap().render_to("course", &context, &mut buffer).unwrap();
                let mut file = File::create(format!("html/courses/{}.html", id)).expect("Failed to create file");
                file.write_all(&buffer).expect("Failed to write to file");
                println!("Render Time: {:?}", start_time.elapsed());
                total_duration += start_time.elapsed();
        }}))
        .buffer_unordered(threadcount)
        .collect::<Vec<()>>();
        fetches.await;

        println!(
            "loop time is: {:?}",
            lastloop.elapsed(),
        );
    }
}