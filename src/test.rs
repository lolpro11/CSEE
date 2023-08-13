extern crate google_classroom1 as classroom1;
use actix_rt::spawn;
use actix_web::{web, App, HttpResponse, HttpServer, Result, ResponseError};
use chrono::{Datelike, Timelike};
use classroom1::oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, ApplicationSecret};
use hyper::{Response, Body};
use oauth2::basic::{BasicClient, BasicTokenType};
use oauth2::reqwest::http_client;
use oauth2::{
    AuthorizationCode, AuthUrl, ClientId, ClientSecret, CsrfToken, TokenUrl, RedirectUrl, TokenResponse, EmptyExtraTokenFields, AccessToken, RefreshToken, Scope, StandardTokenResponse,
};
use serde::{Deserialize, Serialize};
use tera::Tera;
use std::fs::File;
use std::io::{self, Read, Write};
use classroom1::api::{ListAnnouncementsResponse, ListCoursesResponse, ListCourseWorkResponse, ListCourseWorkMaterialResponse, ListTeachersResponse, ListTopicResponse};
use classroom1::{Classroom, hyper, hyper_rustls, chrono};
use serde_json::Value;
use tera::Context;
use tokio::task;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::runtime::Builder;
// AuthCallbackParams struct for deserialization of query parameters
#[derive(Deserialize)]
struct AuthCallbackParams {
    code: String,
    state: String,
}

// Credentials struct to deserialize from JSON
#[derive(Debug, Deserialize)]
struct Credentials {
    installed: Installed,
}

#[derive(Debug, Deserialize)]
struct Installed {
    client_id: String,
    client_secret: String,
}

lazy_static::lazy_static! {
    static ref CLIENT: BasicClient = {
        // Read credentials from the JSON file
        let credentials = get_credentials().expect("Failed to read credentials from JSON");

        // Google OAuth2 credentials
        let client_id = ClientId::new(credentials.installed.client_id);
        let client_secret = ClientSecret::new(credentials.installed.client_secret);
        let auth_url = AuthUrl::new("https://accounts.google.com/o/oauth2/auth".to_string())
            .expect("Failed to parse Auth URL");
        let token_url = TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
            .expect("Failed to parse Token URL");
        let redirect_url =
            RedirectUrl::new("http://localhost:8080/auth/callback".to_string())
                .expect("Failed to parse Redirect URL");
        // Create an OAuth2 client
        BasicClient::new(client_id, Some(client_secret), auth_url, Some(token_url))
            .set_redirect_uri(redirect_url)
    };
}

fn get_credentials() -> io::Result<Credentials> {
    let mut file = File::open("credentials.json")?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let credentials: Credentials = serde_json::from_str(&contents)?;
    Ok(credentials)
}

// Custom error type that implements ResponseError
#[derive(Debug)]
struct MyError(String);

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred: {}", self.0)
    }
}

impl ResponseError for MyError {}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/login", web::get().to(login))
            .route("/auth/callback", web::get().to(auth_callback))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

async fn login() -> HttpResponse {
    // Redirect the user to the Google OAuth2 authorization URL
    let (auth_url, _csrf_state) = CLIENT
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/drive.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.announcements.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.courses.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.coursework.students.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.coursework.me.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.courseworkmaterials.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.rosters.readonly".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.profile.emails".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.profile.photos".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/classroom.topics.readonly".to_string()))
        .url();
    
    HttpResponse::Found()
        .append_header(("Location", auth_url.to_string()))
        .finish()
}

async fn fetch_classroom_data(auth_secret: ApplicationSecret) -> Result<(), MyError> {
    // Create the OAuth2 authenticator using the ApplicationSecret
    let authenticator = InstalledFlowAuthenticator::builder(auth_secret, InstalledFlowReturnMethod::HTTPRedirect)
        .persist_tokens_to_disk("tokens.json")
        .build()
        .await
        .expect("Failed to build InstalledFlowAuthenticator");

    let hub = Classroom::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().build()), authenticator);
    let courses = hub.courses();
    let response: (Response<Body>, ListCoursesResponse) = courses.list().page_size(100).doit().await.unwrap();
    
    println!("{:?}", response);
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
            context.insert("topics", &topics.1.topic.clone().unwrap());
        }
        //let course_work_student_submission_list: (Response<Body>, ListStudentSubmissionsResponse) = courses.course_work_student_submissions_list(course_id: &the_id).doit().await.unwrap();
        println!("{:#?}", &context);
        tera.render_to("course", &context, &mut buffer).unwrap();
        let mut file = File::create(format!("html/courses/{}.html", the_id)).expect("Failed to create file");
        file.write_all(&buffer).expect("Failed to write to file");
        buffer = Vec::new();
    }
    
    Ok(())
}


async fn auth_callback(params: web::Query<AuthCallbackParams>) -> Result<HttpResponse, MyError> {
    // Verify the CSRF token (in a real app, you should use a proper session store)
    let csrf_state = CsrfToken::new(params.state.clone());
    let csrf_secret = csrf_state.secret();

    if &params.state != csrf_secret {
        return Ok(HttpResponse::InternalServerError().finish());
    }
    
    // Exchange the authorization code for an access token
    let code = AuthorizationCode::new(params.code.clone());

    // Use actix_web::block to run the blocking code asynchronously
    let token_response = actix_web::web::block(move || {
        CLIENT.exchange_code(code).request(http_client)
    })
    .await
    .map_err(|error| MyError(format!("Failed to exchange code for access token: {}", error)))?
    .unwrap();

    let auth_secret = classroom1::oauth2::read_application_secret("credentials.json")
        .await
        .expect("client secret couldn't be read.");

    // Save the token response to a JSON file
    save_tokens_to_file(&[token_response])?; // Save a list with a single token response

    let auth_secret_clone = auth_secret.clone();
    spawn(async move {
        fetch_classroom_data(auth_secret_clone).await.expect("Error fetching classroom data");
    });

    // Return the response without waiting for fetch_classroom_data
    Ok(HttpResponse::Ok().finish())
}



#[derive(Debug, Serialize, Deserialize)]
struct MyTokenResponse {
    scopes: Vec<String>,
    token: TokenInfo,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenInfo {
    access_token: AccessToken,
    refresh_token: Option<RefreshToken>,
    expires_at: [i64; 9], // Array of integers [year, month, day, hour, minute, second, millisecond, microsecond, nanosecond]
    id_token: Option<String>,
}

impl From<&StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for MyTokenResponse {
    fn from(token_response: &StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        MyTokenResponse {
            scopes: token_response.scopes().map_or_else(Vec::new, |scopes| {
                scopes.iter().map(|scope| scope.as_str().to_owned()).collect()
            }),
            token: TokenInfo {
                access_token: token_response.access_token().clone(),
                refresh_token: token_response.refresh_token().cloned(),
                expires_at: token_response
                    .expires_in()
                    .map_or([0; 9], |duration| {
                        let now = chrono::Utc::now();
                        let expiration_time = now + chrono::Duration::from_std(duration).unwrap();
                        [
                            expiration_time.year() as i64,
                            expiration_time.ordinal() as i64,
                            expiration_time.hour() as i64,
                            expiration_time.minute() as i64,
                            expiration_time.second() as i64,
                            expiration_time.timestamp_subsec_nanos() as i64,
                            0,
                            0,
                            0,
                        ]
                    }),
                id_token: None,
            },
        }
    }
}

impl From<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>> for MyTokenResponse {
    fn from(token_response: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>) -> Self {
        MyTokenResponse::from(&token_response)
    }
}

impl From<std::io::Error> for MyError {
    fn from(error: std::io::Error) -> Self {
        MyError(format!("IO Error: {}", error))
    }
}

fn save_tokens_to_file(token_responses: &[StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>]) -> io::Result<()> {
    // Read the existing tokens from the file, if it exists
    let mut existing_tokens: Vec<MyTokenResponse> = match File::open("tokens.json") {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
        }
        Err(_) => Vec::new(),
    };

    // Convert the new token_responses to MyTokenResponse
    let new_tokens: Vec<MyTokenResponse> = token_responses.iter().map(|token_response| token_response.into()).collect();

    // Insert the new tokens at index 0, pushing the existing ones down
    existing_tokens.splice(0..0, new_tokens);

    // Serialize the updated tokens to JSON
    let json = serde_json::to_string(&existing_tokens)
        .expect("Failed to serialize token responses");

    // Write the updated tokens back to the file
    let mut file = File::create("tokens.json")?;
    file.write_all(json.as_bytes())?;

    Ok(())
}


