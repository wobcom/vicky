use reqwest::StatusCode;
use yansi::Paint;


pub fn print_http(status: Option<StatusCode>, msg: &str) {
    println!("{}", format_http_msg(status, msg));
}

pub fn format_http_msg(status: Option<StatusCode>, msg: &str) -> String {
    let prefix = if let Some(code) = status {
        if code.is_informational() {
            code.resetting()
        } else if code.is_redirection() {
            code.yellow()
        } else if code.is_success() {
            code.bright_green()
        } else {
            code.bright_red()
        }
        .bold()
        .to_string()
    } else {
        "HTTP Send Error".bold().bright_red().to_string()
    };
    format!("[ {prefix} ] {msg}")
}
