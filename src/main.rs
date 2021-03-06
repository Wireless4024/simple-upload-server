use std::fs::{create_dir_all, rename};
use std::path::PathBuf;

use actix_multipart::Multipart;
use actix_web::{App, get, HttpServer, post, Responder, web};
use actix_web::web::Query;
use anyhow::Result;
use base64_url::encode;
use futures_util::StreamExt;
use rand::Rng;
use serde::Deserialize;
use sha2::{Digest, Sha512_224};
use tokio_uring::fs::{File, remove_file};

static UPLOAD_PATH: &'static str = "static";
static mut CODE: String = String::new();

#[actix_rt::main]
async fn main() -> Result<()> {
	dotenv::dotenv().ok();

	let upload_dir = PathBuf::from(UPLOAD_PATH);
	if !upload_dir.exists() {
		create_dir_all(upload_dir)?;
	}
	if let Ok(a) = std::env::var("CODE") {
		unsafe { CODE = a }
	}

	HttpServer::new(move || {
		let app = App::new()
			.service(actix_files::Files::new(UPLOAD_PATH, ".").disable_content_disposition())
			.service(delete)
			.service(upload);

		app
	})
		.bind(std::env::var("BIND").unwrap_or(String::from("0.0.0.0:8080")))?
		.run()
		.await?;

	Ok(())
}

fn rand_str() -> String {
	let mut rng = rand::thread_rng();
	(0..32).map(|_| rng.gen_range('a'..'z')).collect()
}

#[derive(Deserialize)]
struct CodeCheck {
	code: Option<String>,
}

fn check_code(code: Option<String>) -> bool {
	unsafe {
		return if CODE.is_empty() {
			true
		} else {
			code.is_some() && CODE.as_str() == code.unwrap().as_str()
		};
	}
}

#[post("/upload")]
async fn upload(mut data: Multipart, Query(CodeCheck { code }): Query<CodeCheck>) -> impl Responder {
	if !check_code(code) {
		return String::new();
	}

	while let Some(Ok(mut data)) = data.next().await {
		let name = data.name();
		if name == "file" {
			let ext = data
				.content_disposition()
				.get_filename()
				.map(|it| match it.rsplit_once('.') {
					Some((_, ext)) => {
						let mut str = String::with_capacity(ext.len() + 1);
						str.push('.');
						str.push_str(ext);
						str
					}
					_ => String::new()
				})
				.unwrap();
			let dir = PathBuf::from(UPLOAD_PATH);
			let upload_target = dir.join(rand_str().as_str());
			let target_file = File::create(&upload_target).await.unwrap();
			let mut cursor = 0u64;
			let mut buffer: Option<Vec<u8>> = None;
			let mut hasher = Sha512_224::default();
			while let Some(Ok(data)) = data.next().await {
				let buf = match buffer {
					Some(mut b) => {
						let bytes = data.as_ref();
						Digest::update(&mut hasher, bytes);
						b.copy_from_slice(bytes);
						b
					}
					None => data.to_vec()
				};
				let (len, b) = target_file.write_at(buf, cursor).await;
				match len {
					Ok(0) | Err(..) => {
						break;
					}
					Ok(len) => {
						cursor += len as u64;
						buffer = Some(b)
					}
				}
			}
			let hash = hasher.finalize();
			let mut filename = encode(hash.as_slice());
			filename.reserve_exact(ext.len());
			filename.push_str(ext.as_str());
			let outfile = dir.join(&filename);
			target_file.close().await.unwrap();
			rename(upload_target, outfile).unwrap();
			return filename;
		}
	}
	String::new()
}

#[derive(Deserialize)]
struct DeleteFile {
	hash: String,
	ext: String,
}

impl DeleteFile {
	fn join(self) -> String {
		let mut base = self.hash;
		base.reserve_exact(self.ext.len() + 1);
		base.push('.');
		base.push_str(self.ext.as_str());
		base
	}
}

#[get("/delete/{hash}/{ext}")]
async fn delete(path: web::Path<DeleteFile>, Query(CodeCheck { code }): Query<CodeCheck>) -> impl Responder {
	if !check_code(code) {
		return "false";
	}

	let info = path.into_inner();
	if info.hash.chars().any(|it| !it.is_alphanumeric()) || info.ext.chars().any(|it| !it.is_alphanumeric()) {
		return "false";
	}
	let file = PathBuf::from(UPLOAD_PATH).join(info.join());
	if file.exists() {
		remove_file(file).await.unwrap();

		return "true";
	}
	return "false";
}