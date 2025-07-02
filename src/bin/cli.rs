use std::env;
use std::string::ToString;
use reqwest::header::{HeaderMap, HeaderValue};
use space_trader::ansi::*;

struct Context {
    do_secure: bool,
    address: String,
    user_name: String,
    password: String,
}

impl Context {
    fn compose_url(&self, path: &str) -> String {
        let protocol = if self.do_secure { "https" } else { "http" }.to_string();
        format!("{protocol}://{}{}", self.address, path)
    }
}
const DEFAULT_ADDRESS: &str = "127.0.0.1:2000";

/// cli client for space trader
fn main() {
    println!("{}Space Trader CLI{}", ANSI_BOLD_CYAN, ANSI_RESET);
    let ctx = setup();
    if ctx.is_some() {
        process(ctx.unwrap());
    }
}

fn polling_thread(ctx: &Context) {
    // TODO
}

fn print_error(text: &str) {
    eprintln!("{}ERROR:{text}{}", ANSI_BOLD_RED, ANSI_RESET);
}

fn log_in(ctx: &Context) -> Option<String> {
    // Log in
    let client = reqwest::blocking::Client::new();
    match client.get(ctx.compose_url("/"))
        .basic_auth(ctx.user_name.clone(), Some(ctx.password.clone()))
        .body("")
        .send() {
        Ok(response) => {
            let sid = response.headers().get("x-session-id");
            if sid.is_some() { Some(sid.unwrap().to_str().unwrap().to_string()) } else { None }
        },
        Err(error) => {
            print_error(&error.to_string());
            None
        }
    }
}

fn process(ctx: Context) {
    let session_id = log_in(&ctx);
    if session_id.is_none() {
        print_error("Login failed");
        return;
    }
    let session_id = session_id.unwrap();

    //TODO temp code - do a quit command
    println!("Session id = {session_id}"); // TODO remove
    let client = reqwest::blocking::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("X-Session-Id", HeaderValue::from_str(&*session_id).unwrap());
    match client.post(ctx.compose_url("/admin/quit"))
        .headers(headers)
        .body("")
        .send() {
        Ok(response) => {
            println!("Response: {}", response.status());
            println!("Body: {}", response.text().unwrap_or_default());
        },
        Err(error) => {
            print_error(&error.to_string());
            return;
        }
    }
    //TODO end temp code

    // Start polling thread
    // TODO
    
    // Loop on accepting and handling input from the user
    // TODO

    // Log out
    // TODO

    // Stop polling thread
    // TODO
}

fn setup() -> Option<Context> {
    let args: Vec<String> = env::args().collect();

    // handle command line options
    let mut address: Option<String> = Some(DEFAULT_ADDRESS.to_string());
    let mut user_name: Option<String> = None;
    let mut password: Option<String> = None;

    let mut ax: usize = 1;
    while ax < args.len() {
        let switch = args.get(ax);
        ax += 1;
        if switch?.to_lowercase() == "-a" {
            if ax == args.len() {
                print_error("-a switch has no value");
                return None;
            }
            address = Some(args.get(ax).unwrap().clone());
            ax += 1;
        } else if switch?.to_lowercase() == "-p" {
            if ax == args.len() {
                print_error("-p switch has no value");
                return None;
            }
            password = Some(args.get(ax).unwrap().clone());
            ax += 1;
        } else if switch?.to_lowercase() == "-u" {
            if ax == args.len() {
                print_error("-u switch has no value");
                return None;
            }
            user_name = Some(args.get(ax).unwrap().clone());
            ax += 1;
        } else {
            print_error("Invalid switch {switch}");
            return None;
        }
    }

    if user_name.is_none() {
        print_error("User name is required");
        usage(&args.get(0).unwrap());
        return None;
    } else if password.is_none() {
        print_error("Password is required");
        usage(&args.get(0).unwrap());
        return None;
    }

    Some(Context{
        do_secure: false, // TODO only http for now
        address: address.unwrap().clone(),
        user_name: user_name.unwrap().clone(),
        password: password.unwrap().clone()})
}

fn usage(cli_name: &String) {
    eprintln!("{}Usage: {cli_name} {{options}} {{ip-address}}:{{port}}", ANSI_BOLD_YELLOW);
    eprintln!("options:");
    eprintln!("  [-a {{ip_address}}:{{port}}] - IP address and port of server (defaults to 127.0.0.1:2000)");
    eprintln!("  -u {{user_name}}");
    eprintln!("  -p {{password}}");
    eprintln!("{}", ANSI_RESET);
}
