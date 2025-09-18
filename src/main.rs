#[macro_use]
extern crate actix_web;
mod command;
mod config;

use crate::config::CompleteSettings;
use actix_settings::{ApplySettings, Mode};
use actix_web::dev::ServiceRequest;
use actix_web::error::ErrorUnauthorized;
use actix_web::middleware::Logger;
use actix_web::web::Data;
use actix_web::{App, Error, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;
use log::{warn, LevelFilter};
use log4rs::append::console::ConsoleAppender;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Root};
use log4rs::Config;
use std::io;

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    let data: Option<&Data<CompleteSettings>> = req.app_data();
    if credentials.token() != data.unwrap().application.auth {
        warn!("Wrong Token: {}", credentials.token());
        return Err((ErrorUnauthorized("Wrong Token"), req));
    }
    Ok(req)
}

#[actix_rt::main]
async fn main() -> io::Result<()> {
    let settings = CompleteSettings::parse_toml("config.toml")
        .expect("Failed to parse `Settings` from config.toml");

    init_logger(&settings);

    HttpServer::new({
        // clone settings into each worker thread
        let settings = settings.clone();

        move || {
            let auth = HttpAuthentication::bearer(validator);
            App::new()
                // enable logger - always register actix-web Logger middleware last
                .wrap(Logger::default())
                .wrap(auth)
                .app_data(Data::new(settings.clone()))
                // register HTTP requests handlers
                .service(command::exec)
        }
    })
    .try_apply_settings(&settings)?
    .run()
    .await
}

/// Initialize the logging infrastructure.
fn init_logger(settings: &CompleteSettings) {
    if !settings.actix.enable_log {
        return;
    }

    let stdout = ConsoleAppender::builder().build();
    let appender = FileAppender::builder()
        .build(settings.application.logfile.as_str())
        .unwrap();

    let loglevel = match settings.actix.mode {
        Mode::Development => LevelFilter::Debug,
        Mode::Production => LevelFilter::Info,
    };

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(appender)))
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .logger(log4rs::config::Logger::builder().build("actix_server", loglevel))
        .logger(log4rs::config::Logger::builder().build("actix_web", loglevel))
        .logger(log4rs::config::Logger::builder().build("command_proxy", loglevel))
        .build(
            Root::builder()
                .appender("file")
                .appender("stdout")
                .build(LevelFilter::Info),
        )
        .unwrap();

    log4rs::init_config(config).expect("Logging not initialized");
}
