use crate::config::CompleteSettings;
use actix_web::web::{Data, Json};
use actix_web::HttpResponse;
use base64::engine::general_purpose::STANDARD;
use base64::read::DecoderReader;
use log::{debug, error};
use serde::{Deserialize, Serialize};
use std::fs::{remove_file, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize, Serialize)]
pub struct FileCopyRequest {
    pub path: String,
    pub data: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileDeleteRequest {
    pub path: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileCopyResult {
    pub path: String,
    pub size: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileDeleteResult {
    pub path: String,
    pub success: bool,
}

impl FileCopyResult {
    fn new(final_path: PathBuf, size: usize) -> Self {
        let path = final_path.display().to_string();
        Self { size, path }
    }
}

impl FileDeleteResult {
    fn new(final_path: PathBuf, success: bool) -> Self {
        let path = final_path.display().to_string();
        Self { path, success }
    }
}

#[post("/file")]
pub async fn file_copy(
    copy_req: Json<FileCopyRequest>,
    config: Data<CompleteSettings>,
) -> HttpResponse {
    let data: &[u8] = copy_req.data.as_ref();
    let mut decoder = DecoderReader::new(data, &STANDARD);
    let mut buffer = vec![];
    if decoder.read_to_end(buffer.as_mut()).is_err() {
        error!("failed to decode base64 value");
        return HttpResponse::Forbidden()
            .append_header(("x-error", "data cannot be decoded"))
            .finish();
    }
    let final_path;
    match check_path(&copy_req.path, &config) {
        Ok(path) => {
            final_path = path;
        }
        Err(response) => {
            return response;
        }
    }
    let file = File::create(&final_path);
    if file.is_err() {
        error!("File creation error: {}", file.as_ref().unwrap_err());
        return HttpResponse::Forbidden()
            .append_header((
                "x-error",
                "file cannot be created: ".to_owned() + file.unwrap_err().to_string().as_str(),
            ))
            .finish();
    }
    let write_result = file.unwrap().write(&buffer);
    if write_result.is_err() {
        error!("File write error: {}", write_result.as_ref().unwrap_err());
        return HttpResponse::Forbidden()
            .append_header((
                "x-error",
                "error in writing file".to_owned() + &*write_result.unwrap_err().to_string(),
            ))
            .finish();
    }
    debug!("Successfully wrote {}", final_path.display());
    HttpResponse::Ok().json(FileCopyResult::new(final_path, write_result.unwrap()))
}

#[delete("/file")]
pub async fn file_delete(
    delete_req: Json<FileDeleteRequest>,
    config: Data<CompleteSettings>,
) -> HttpResponse {
    let final_path;
    match check_path(&delete_req.path, &config) {
        Ok(path) => {
            final_path = path;
        }
        Err(response) => {
            return response;
        }
    }
    if !final_path.exists() {
        debug!("file not exists: {}", final_path.display());
        return HttpResponse::Ok().json(FileDeleteResult::new(final_path, true));
    }
    match remove_file(&final_path) {
        Ok(_) => {
            debug!("file successfully deleted: {}", final_path.display());
            HttpResponse::Ok().json(FileDeleteResult::new(final_path, true))
        }
        Err(error) => {
            error!("File delete error: {}", error);
            HttpResponse::Forbidden()
                .append_header((
                    "x-error",
                    "file cannot be deleted: ".to_owned() + error.to_string().as_str(),
                ))
                .json(FileDeleteResult::new(final_path, false))
        }
    }
}

fn check_path(path_string: &String, config: &CompleteSettings) -> Result<PathBuf, HttpResponse> {
    let upload_path = Path::new(&config.application.upload_path);
    if !upload_path.exists() || !upload_path.is_dir() {
        error!("upload path not exists: {}", upload_path.display());
        return Err(HttpResponse::Forbidden()
            .append_header(("x-error", "configured upload path not exists"))
            .finish());
    }
    let file_path_complete = upload_path.join(path_string);
    debug!("given path: {}", file_path_complete.display());
    let file_path_parent = file_path_complete
        .parent()
        .expect("parent not found")
        .canonicalize();
    let file_name = file_path_complete.file_name();
    if file_name.as_ref().is_none() {
        error!("no filename in path: {}", file_path_complete.display());
        return Err(HttpResponse::Forbidden()
            .append_header(("x-error", "path doesn't point to a file"))
            .finish());
    }
    if file_path_parent.is_err() {
        error!("path not found: {}", file_path_parent.unwrap_err());
        return Err(HttpResponse::Forbidden()
            .append_header(("x-error", "file path invalid"))
            .finish());
    }
    if !file_path_parent.as_ref().unwrap().is_dir() {
        error!(
            "path not exist: {}",
            file_path_parent.as_ref().unwrap().display()
        );
        return Err(HttpResponse::Forbidden()
            .append_header(("x-error", "path not exist"))
            .finish());
    }
    if !file_path_parent.as_ref().unwrap().starts_with(upload_path) {
        error!(
            "path outside of upload path: {}",
            file_path_parent.as_ref().unwrap().display()
        );
        return Err(HttpResponse::Forbidden()
            .append_header(("x-error", "path outside upload path"))
            .finish());
    }
    let final_path = file_path_parent.unwrap().join(file_name.unwrap());
    debug!("final path is: {}", final_path.display());
    Ok(final_path)
}
