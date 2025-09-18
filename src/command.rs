use crate::config::CompleteSettings;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use log::debug;
use serde::{Deserialize, Serialize};
use std::process::{Command, Output};

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandRequest {
    pub command: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CommandOutput {
    pub output: String,
    pub exitcode: Option<i32>,
    pub stderr: Option<String>,
}

impl CommandOutput {
    fn new(output_val: &Output) -> Self {
        let output = String::from_utf8(output_val.stdout.clone()).unwrap();
        let stderr = String::from_utf8(output_val.stderr.clone()).ok();
        let exitcode = output_val.status.code();
        Self {
            output,
            exitcode,
            stderr,
        }
    }
}

#[post("/command")]
pub async fn exec(
    command_req: Json<CommandRequest>,
    config: Data<CompleteSettings>,
) -> HttpResponse {
    if command_req.command.as_ref().is_none() {
        return HttpResponse::Forbidden()
            .append_header(("x-error", "command is needed"))
            .finish();
    }
    let command_str = command_req.command.as_ref().unwrap().as_str();
    debug!("Get Command: {}", command_str);
    let command = match config.application.commands.get(command_str) {
        Some(val) => {
            let mut values = val.as_str().unwrap().split(" ");
            let mut command_in = Command::new(values.next().expect(""));
            command_in.args(values);
            Ok(command_in)
        }
        None => Err(format!("command {} not found", command_str)),
    };
    if command.is_err() {
        return HttpResponse::BadRequest()
            .append_header(("X-ERROR", command.err().unwrap()))
            .finish();
    }
    debug!(
        "Execute Command: {} {}",
        command
            .as_ref()
            .ok()
            .unwrap()
            .get_program()
            .to_str()
            .unwrap(),
        command
            .as_ref()
            .ok()
            .unwrap()
            .get_args()
            .map(|s| String::from_utf8(Vec::from(s.as_encoded_bytes())).unwrap())
            .collect::<Vec<String>>()
            .join(" ")
    );
    let output = command.expect("unexpected command").output();
    match output {
        Ok(output_ok) => {
            debug!(
                "output: {}",
                String::from_utf8(output_ok.clone().stdout).unwrap()
            );
            HttpResponse::Ok().json(CommandOutput::new(&output_ok))
        }
        Err(error) => HttpResponse::BadRequest().body(format!("Error {}", error)),
    }
}
