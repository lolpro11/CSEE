use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};
mod classroom_v1_types;
#[tokio::main]
async fn main() {
    let secret = yup_oauth2::read_application_secret("credentials.json")
        .await
        .expect("credentials.json");
    let mut auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::HTTPRedirect)
    .persist_tokens_to_disk("tokens.json")
    .build()
    .await
    .unwrap();

    let scopes = &["https://www.googleapis.com/auth/classroom.courses.readonly",
        "https://www.googleapis.com/auth/classroom.announcements.readonly",
        "https://www.googleapis.com/auth/classroom.student-submissions.me.readonly",
        "https://www.googleapis.com/auth/drive"
    ];
    match auth.token(scopes).await {
        Ok(token) => println!("Token acquired!"),
        Err(e) => println!("error: {:?}", e),
    }
}