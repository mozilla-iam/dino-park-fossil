use crate::retrieve::app::retrieve_app;
use crate::send::app::internal_send_app;
use crate::send::app::send_app;
use crate::settings::AvatarSettings;
use crate::storage::loader::filesystem::FilesystemLoader;
use crate::storage::saver::filesystem::FilesystemSaver;
use actix_web::dev::Body;
use actix_web::dev::ResponseBody;
use actix_web::dev::Service;
use actix_web::middleware::Logger;
use actix_web::test;
use actix_web::web;
use actix_web::web::Data;
use actix_web::App;
use actix_web::HttpMessage;
use bytes::Bytes;
use cis_client::getby::GetBy;
use cis_client::AsyncCisClientTrait;
use cis_client::CisFut;
use cis_profile::crypto::SecretStore;
use cis_profile::schema::Profile;
use dino_park_gate::scope::ScopeAndUser;
use dino_park_trust::AALevel;
use dino_park_trust::GroupsTrust;
use dino_park_trust::Trust;
use failure::Error;
use lru_time_cache::LruCache;
use serde::Deserialize;
use serde_json::Value;
use std::env;
use std::marker::Send;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct MockCisClient {}

unsafe impl Sync for MockCisClient {}

unsafe impl Send for MockCisClient {}

impl AsyncCisClientTrait for MockCisClient {
    fn get_user_by(&self, _id: &str, _by: &GetBy, _filter: Option<&str>) -> CisFut<Profile> {
        unimplemented!();
    }
    fn get_inactive_user_by(
        &self,
        _id: &str,
        _by: &GetBy,
        _filter: Option<&str>,
    ) -> CisFut<Profile> {
        unimplemented!();
    }
    fn get_any_user_by(&self, _id: &str, _by: &GetBy, _filter: Option<&str>) -> CisFut<Profile> {
        unimplemented!();
    }
    fn update_user(&self, _id: &str, _profile: Profile) -> CisFut<Value> {
        unimplemented!();
    }
    fn update_users(&self, _profiles: &[Profile]) -> CisFut<Value> {
        unimplemented!();
    }
    fn delete_user(&self, _id: &str, _profile: Profile) -> CisFut<Value> {
        unimplemented!();
    }
    fn get_secret_store(&self) -> &SecretStore {
        unimplemented!();
    }
}

#[actix_rt::test]
async fn healthz_check_returns_success() -> Result<(), Error> {
    let app = App::new()
        .wrap(Logger::default())
        .service(web::scope("").service(crate::healthz::healthz_app()));
    let mut app = test::init_service(app).await;

    let req = test::TestRequest::get().uri("/healthz").to_request();

    let res = test::call_service(&mut app, req).await;
    assert!(res.response().status().is_success());

    Ok(())
}

#[actix_rt::test]
async fn avatar_test() -> Result<(), Error> {
    env::set_var(
        "RUST_LOG",
        "actix_http=trace,actix_web=trace,dino_park_fossil=trace",
    );

    let path = env::temp_dir();

    let saver = Data::new(FilesystemSaver {
        path: Arc::new(path.clone()),
    });
    let loader = Data::new(FilesystemLoader {
        path: Arc::new(path.clone()),
    });

    let avatar_settings = Data::new(AvatarSettings {
        s3_bucket: String::from("test_avatar_bucket"),
        retrieve_by_id_path: String::from("/avatar/get/id/"),
        picture_api_url: String::from("http://localhost"),
    });

    let cis_client = Data::new(MockCisClient {});

    let time_to_live = ::std::time::Duration::from_secs(60 * 60 * 24);
    let cache = Data::new(Mutex::new(
        LruCache::<String, String>::with_expiry_duration_and_capacity(time_to_live, 2000),
    ));

    // insert the cache entry for the current user
    {
        cache.try_lock().unwrap().insert(
            String::from("1"),
            // random uuid4 without hyphens
            String::from("78b814ab025e4da380836ff683be79e1"),
        );
    }

    let app = App::new()
        .wrap_fn(|req, srv| {
            req.extensions_mut().insert(ScopeAndUser {
                aa_level: AALevel::Medium,
                groups_scope: GroupsTrust::Admin,
                scope: if req.query_string().contains("@@testScope@@=authenticated") {
                    Trust::Authenticated
                } else {
                    Trust::Public
                },
                user_id: String::from("1"),
            });
            srv.call(req)
        })
        .app_data(cis_client)
        .app_data(loader)
        .app_data(saver)
        .app_data(avatar_settings)
        .app_data(cache)
        .service(
            web::scope("/avatar")
                .service(retrieve_app::<MockCisClient, FilesystemLoader>())
                .service(send_app::<FilesystemSaver, FilesystemLoader>()),
        )
        .service(internal_send_app::<FilesystemSaver, FilesystemLoader>());
    let mut app = test::init_service(app).await;

    let sample_image_data = include_bytes!("data/sample_image.png");

    let head = b"----abbc761f78ff4d7cb7573b5a23f96ef0\r\nContent-Disposition: form-data; name=\"file\"; filename=\"sample_image.png\"\r\nContent-Type: image/png\r\n\r\n";

    let footer = b"\r\n----abbc761f78ff4d7cb7573b5a23f96ef0--\r\n\r\n";

    let mut data = Vec::new();

    data.extend(head.iter());
    data.extend(sample_image_data.iter());
    data.extend(footer.iter());

    // construct payload
    let payload = Bytes::from(data);

    // dbg!(&payload);

    let req = test::TestRequest::post()
        .uri("/avatar/send/intermediate?@@testScope@@=authenticated")
        .header(
            "Content-Type",
            "multipart/form-data; boundary=\"--abbc761f78ff4d7cb7573b5a23f96ef0\"",
        )
        .set_payload(payload)
        .to_request();

    #[derive(Deserialize, Debug)]
    struct UuidResponse {
        pub uuid: String,
    }

    let res_json: UuidResponse = test::read_response_json(&mut app, req).await;

    // make sure returned uuid is valid
    assert!(res_json.uuid.parse::<uuid::Uuid>().is_ok());

    // save avatar
    let req = test::TestRequest::post()
        .uri(&format!(
            "/internal/save/{uuid}?@@testScope@@=authenticated",
            uuid = res_json.uuid
        ))
        .header("Content-type", "application/json")
        .set_json(&serde_json::json!({
            "intermediate": res_json.uuid,
            "display": "public"
            // don't supply the old_url field as we're saving a new avatar
            // for a user that doesn't have one avatar saved yet
        }))
        .to_request();

    #[derive(Deserialize, Debug)]
    struct PictureResponse {
        pub url: String,
    }

    let res_json: PictureResponse = test::read_response_json(&mut app, req).await;
    let mut iccp_crc = None;

    for size in &["raw", "528", "264", "100", "40"] {
        let req = test::TestRequest::get()
            .uri(&format!("{}?size={}", res_json.url, size))
            .to_request();

        let mut res = test::call_service(&mut app, req).await;

        assert!(res.status().is_success());

        let body: ResponseBody<Body> = res.take_body().into_body();

        if let ResponseBody::Other(Body::Bytes(bytes)) = body {
            // test that iCCP still exists in downscaled image (and it is valid)
            let mut decoder = lodepng::Decoder::new();
            decoder.remember_unknown_chunks(true);

            assert!(decoder.decode(bytes).is_ok(), "invalid png image returned");

            let iccp_chunk = decoder
                .info_png()
                .get("iCCP")
                .expect("returned image does not have a iccp chunk anymore!");

            // validate that the data is consistent across all images
            if iccp_crc.is_none() {
                iccp_crc = Some(iccp_chunk.crc());
            } else {
                assert_eq!(iccp_crc.unwrap(), iccp_chunk.crc());
            }
        } else {
            panic!("byte response expected when fetching images successfully");
        }
    }

    assert_ne!(iccp_crc, None, "no crc checksums were compared?!");

    Ok(())
}
