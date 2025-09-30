use axum::{
    Router,
    extract::{ConnectInfo, Form, State},
    response::Html,
    routing::{get, post},
};
use chrono::Local;
use clap::Parser;
use rand::{Rng, distributions::Uniform};
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::Write,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

#[derive(Parser)]
struct Args {
    #[clap(long, default_value = "50")]
    tokens: usize,
    #[clap(long, default_value = "8888")]
    port: u16,
}

#[derive(Clone)]
struct AppState {
    tokens: Arc<Mutex<HashSet<String>>>,
    used_tokens: Arc<Mutex<HashSet<String>>>,
    rate_limit: Arc<Mutex<HashMap<String, Instant>>>,
    records_file: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let now = Local::now();
    let iso = now.format("%Y-%m-%dT%H-%M-%S").to_string();
    println!("Start time: {}", iso);

    // Generate tokens
    let mut rng = rand::thread_rng();
    let dist = Uniform::from(10000000..99999999);
    let tokens: HashSet<String> = (0..args.tokens)
        .map(|_| rng.sample(dist).to_string())
        .collect();

    let tokens_file = format!("tokens-{}.txt", iso);
    let mut f = File::create(&tokens_file).unwrap();
    for t in &tokens {
        writeln!(f, "{}", t).unwrap();
    }

    let records_file = format!("records-{}.txt", iso);

    let state = AppState {
        tokens: Arc::new(Mutex::new(tokens)),
        used_tokens: Arc::new(Mutex::new(HashSet::new())),
        rate_limit: Arc::new(Mutex::new(HashMap::new())),
        records_file,
    };

    let app = Router::new()
        .route("/", get(show_token_form).post(check_token))
        .route("/id", post(enter_id))
        .route("/confirm", post(confirm_id))
        .with_state(state.clone());

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    println!("Listening on http://{}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}

enum NavButton {
    None,
    GoBack,
    GoHome,
}

fn page_wrapper(content: &str, nav: NavButton) -> String {
    let nav_btn = match nav {
        NavButton::None => "".to_string(),
        NavButton::GoBack => {
            r#"<button onclick="window.history.back()" class="go-back">Go Back</button>"#
                .to_string()
        }
        NavButton::GoHome => {
            r#"<button onclick="window.location.href='/'" class="go-back">Go Home</button>"#
                .to_string()
        }
    };
    format!(
        r#"
        <meta name="viewport" content="width=device-width, initial-scale=1">
        <style>
            body {{ font-family: sans-serif; margin: 2em; text-align: center; }}
            .msg {{ margin: 2em 0; font-size: 1.3em; }}
            input, button.form-btn {{ font-size: 1.2em; padding: 0.5em; margin: 0.5em 0; width: 100%; box-sizing: border-box; }}
            form {{ max-width: 400px; margin: auto; }}
            button.go-back {{ width: auto; display: inline-block; margin: 1em auto 0 auto; font-size: 1em; padding: 0.5em 1.2em; }}
        </style>
        <div class="msg">{}</div>
        {}
        "#,
        content, nav_btn
    )
}

#[derive(Deserialize)]
struct TokenForm {
    token: String,
}

async fn show_token_form() -> Html<String> {
    Html(page_wrapper(
        r#"
        <h1>Enter Token</h1>
        <form method="post">
            <input name="token" type="tel" inputmode="numeric" pattern="\d{1,8}" maxlength="8" required placeholder="8-digit token">
            <button type="submit" class="form-btn">Continue</button>
        </form>
        "#,
        NavButton::None,
    ))
}

async fn check_token(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Form(form): Form<TokenForm>,
) -> Html<String> {
    // Only sanitize format here, do not mark token as used yet
    if form.token.len() > 8 || !form.token.chars().all(|c| c.is_ascii_digit()) {
        return Html(page_wrapper(
            "<h2>Invalid token format.</h2>",
            NavButton::GoHome,
        ));
    }

    let ip = addr.ip().to_string();
    let mut rate_limit = state.rate_limit.lock().unwrap();
    if let Some(last) = rate_limit.get(&ip)
        && last.elapsed() < Duration::from_secs(10)
    {
        return Html(page_wrapper(
            "<h2>Rate limit: wait 10 seconds before retrying.</h2>",
            NavButton::GoBack,
        ));
    }
    rate_limit.insert(ip, Instant::now());
    drop(rate_limit);

    // Only check if token exists and not used, do not consume it yet
    let tokens = state.tokens.lock().unwrap();
    let used = state.used_tokens.lock().unwrap();
    if used.contains(&form.token) {
        return Html(page_wrapper(
            "<h2>Token already used.</h2>",
            NavButton::GoHome,
        ));
    }
    if !tokens.contains(&form.token) {
        return Html(page_wrapper("<h2>Invalid token.</h2>", NavButton::GoHome));
    }
    drop(tokens);
    drop(used);

    Html(page_wrapper(
        &format!(
            r#"
        <h1>Enter Student ID</h1>
        <form method="post" action="/id">
            <input name="student_id" type="tel" inputmode="numeric" pattern="\d{{1,20}}" maxlength="20" required placeholder="Student ID">
            <input type="hidden" name="token" value="{}">
            <button type="submit" class="form-btn">Continue</button>
        </form>
        "#,
            form.token
        ),
        NavButton::GoBack,
    ))
}

#[derive(Deserialize)]
struct IdForm {
    student_id: String,
    token: String,
}

async fn enter_id(Form(form): Form<IdForm>) -> Html<String> {
    // Sanitize student_id: only digits, max 20
    if form.student_id.len() > 20 || !form.student_id.chars().all(|c| c.is_ascii_digit()) {
        return Html(page_wrapper(
            "<h2>Invalid student ID format.</h2>",
            NavButton::GoBack,
        ));
    }

    Html(page_wrapper(
        &format!(
            r#"
        <h1>Confirm Student ID</h1>
        <form method="post" action="/confirm">
            <input type="hidden" name="student_id" value="{}">
            <input type="hidden" name="token" value="{}">
            <p>Student ID: <b>{}</b></p>
            <button type="submit" id="confirm-btn" class="form-btn">Confirm</button>
        </form>
        <script>
            setTimeout(function() {{
                document.getElementById('confirm-btn').disabled = false;
            }}, 3000);
            document.getElementById('confirm-btn').disabled = true;
        </script>
        "#,
            form.student_id, form.token, form.student_id
        ),
        NavButton::GoBack,
    ))
}

#[derive(Deserialize)]
struct ConfirmForm {
    student_id: String,
    token: String,
}

async fn confirm_id(State(state): State<AppState>, Form(form): Form<ConfirmForm>) -> Html<String> {
    // Sanitize student_id and token again
    if form.student_id.len() > 20 || !form.student_id.chars().all(|c| c.is_ascii_digit()) {
        return Html(page_wrapper(
            "<h2>Invalid student ID format.</h2>",
            NavButton::GoBack,
        ));
    }
    if form.token.len() > 8 || !form.token.chars().all(|c| c.is_ascii_digit()) {
        return Html(page_wrapper(
            "<h2>Invalid token format.</h2>",
            NavButton::GoHome,
        ));
    }

    // Verify and use the token here
    let mut tokens = state.tokens.lock().unwrap();
    let mut used = state.used_tokens.lock().unwrap();
    if used.contains(&form.token) {
        return Html(page_wrapper(
            "<h2>Token already used.</h2>",
            NavButton::GoHome,
        ));
    }
    if !tokens.contains(&form.token) {
        return Html(page_wrapper("<h2>Invalid token.</h2>", NavButton::GoHome));
    }
    used.insert(form.token.clone());
    tokens.remove(&form.token);
    drop(tokens);
    drop(used);

    let mut f = File::options()
        .append(true)
        .create(true)
        .open(&state.records_file)
        .unwrap();
    writeln!(f, "{},{}", form.token, form.student_id).unwrap();

    Html(page_wrapper(
        "<h2>Sign-in successful!</h2>",
        NavButton::GoHome,
    ))
}
