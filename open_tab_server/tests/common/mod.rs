use std::{borrow::BorrowMut, future::Future};

use axum::{response::Response, http::{Request, request::Builder}, body::Body};
use http_body::{combinators::UnsyncBoxBody, Body as _};
use open_tab_entities::{mock::{self, MockOption}, EntityTypeId};
use open_tab_server::{auth::{CreateUserRequest, CreateUserResponse, GetTokenRequest, GetTokenResponse, create_key, hash_password}, state::AppState};
use sea_orm::{prelude::Uuid, IntoActiveModel, ActiveModelTrait, DatabaseConnection};
use tower::Service;
use base64::{engine::general_purpose, Engine as _};


#[derive(Default)]
pub struct FixtureOptions
 {
    pub mock_default_tournament: bool,
    pub use_participant_account: Option<Uuid>
}

pub struct Fixture {
    pub app: axum::Router,
    pub auth: Auth,
}

pub enum Auth {
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer {
        token: String,
    },
}

pub struct APIResponse {
    response: Response<UnsyncBoxBody<axum::body::Bytes, axum::Error>>,
}

impl APIResponse {
    pub fn status(&self) -> axum::http::StatusCode {
        self.response.status()
    }

    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> T {
        let mut buf = Vec::new();

        let body = self.response.body_mut();

        while let Some(next) = body.data().await {
            buf.extend_from_slice(&next.unwrap());
        }
        serde_json::from_slice(&buf).unwrap()
    }

    pub async fn text(&mut self) -> String {
        let mut buf = Vec::new();

        let body = self.response.body_mut();

        while let Some(next) = body.data().await {
            buf.extend_from_slice(&next.unwrap());
        }
        String::from_utf8(buf).unwrap()
    }
}

impl From<Response<UnsyncBoxBody<axum::body::Bytes, axum::Error>>> for APIResponse {
    fn from(response: Response<UnsyncBoxBody<hyper::body::Bytes, axum::Error>>) -> Self {
        Self {
            response,
        }
    }
}

impl Fixture {
    pub async fn new(options: FixtureOptions) -> Self {
        Self::new_with_setup(options, |_| async {  }).await
    }

    pub async fn new_with_setup<'a, F, Fut>(options: FixtureOptions, setup_func: F) -> Self    
    where
    F: FnOnce(DatabaseConnection) -> Fut,
    Fut: Future<Output = ()>,
     {
        let mut auth = Auth::None;
        let app = if options.mock_default_tournament {
            let state = AppState::new_test_app().await;
            let group = mock::make_mock_tournament_with_options(MockOption {
                deterministic_uuids: true,
                ..Default::default()
            });
            let tournaments = group.as_group_map().tournaments;
            group.save_all_and_log(&state.db).await.unwrap();
            let pwd = hash_password("test".to_string()).unwrap();
            let new_user_uuid = Uuid::from_u128(900_000);
            let model: open_tab_entities::schema::user::Model = open_tab_entities::schema::user::Model {
                uuid: new_user_uuid,
                password_hash: pwd,
                user_email: None
            };
        
            model.into_active_model().insert(&state.db).await.unwrap();

            let raw_key = [0, 0, 0, 0];

            let key = create_key(&raw_key, new_user_uuid, None, None, false).unwrap();
            key.into_active_model().insert(&state.db).await.unwrap();

            if let Some(part_id) = options.use_participant_account {
                let user_participant = open_tab_entities::schema::user_participant::Model {
                    user_id: new_user_uuid,
                    participant_id: part_id,
                    claim_time: chrono::Utc::now().naive_utc(),
                };
                user_participant.into_active_model().insert(&state.db).await.unwrap();
            }
            else {
                let user_tournament = open_tab_entities::schema::user_tournament::Model {
                    user_id: new_user_uuid,
                    tournament_id: tournaments[0].uuid,
                };
                user_tournament.into_active_model().insert(&state.db).await.unwrap();    
            }

            auth = Auth::Bearer { token: base64::engine::general_purpose::URL_SAFE.encode(&raw_key) };
            setup_func(state.db.clone()).await;
            open_tab_server::app_with_state(state).await
        }
        else {
            open_tab_server::app_with_state(AppState::new_test_app().await).await
        };

        Self {
            app,
            auth
        }
    }

    #[allow(dead_code)]
    pub async fn default() -> Self {
        Self::new(FixtureOptions::default()).await
    }

    #[allow(dead_code)]
    pub fn with_auth(self, auth: Auth) -> Self {
        Self {
            auth: auth,
            ..self
        }
    }

    #[allow(dead_code)]
    pub async fn create_user_and_token(&mut self) -> (Uuid, String) {
        let mut response = self
            .post_json("/api/users", CreateUserRequest {
                password: "testtest".to_string(),
                user_email: None,
            })
            .await;
        assert_eq!(response.status(), 200);
        let body = response.json::<CreateUserResponse>().await;
        let user_id = body.uuid;

        self.auth = Auth::Basic {
            username: user_id.to_string(),
            password: "testtest".to_string(),
        };
        
        let mut response = self
            .post_json("/api/tokens", GetTokenRequest {
                tournament: None,
            })
            .await;

        assert_eq!(response.status(), 200);
        let token : GetTokenResponse = response.json().await;
        (user_id, token.token)
    }

    fn get_base_request(&self) -> Builder {
        let builder = Request::builder();

        let builder = match &self.auth {
            Auth::None => builder,
            Auth::Basic { username, password } => {
                builder.header(
                    "Authorization",
                    format!("Basic {}", general_purpose::URL_SAFE.encode(&format!("{}:{}", username, password)))
                )
            },
            Auth::Bearer { token } => {
                builder.header(
                    "Authorization",
                    format!("Bearer {}", token)
                )
            }
        };

        builder
    }

    #[allow(dead_code)]
    pub async fn get(&mut self, path: &str) -> APIResponse {
        let request = self.get_base_request()
            .uri(path)
            .body(Body::empty())
            .unwrap();
        self.app.borrow_mut()
            .call(request)
            .await
            .unwrap().into()
    }

    #[allow(dead_code)]
    pub async fn post_json_no_body(&mut self, path: &str) -> APIResponse
    {
        let request = self.get_base_request()
            .method("POST")
            .header("Content-Type", "application/json")
            .uri(path)
            .body(Body::empty())
            .unwrap();
        self.app.borrow_mut()
            .call(request)
            .await
            .unwrap().into()
    }

    #[allow(dead_code)]
    pub async fn post_json<T>(&mut self, path: &str, body: T) -> APIResponse where T: serde::Serialize
    {
        let request = self.get_base_request()
            .method("POST")
            .header("Content-Type", "application/json")
            .uri(path)
            .body(
                Body::from(
                    serde_json::to_string(&body).unwrap()
                )
            
            )
            .unwrap();
        self.app.borrow_mut()
            .call(request)
            .await
            .unwrap().into()
    }
}

#[allow(dead_code)]
pub async fn get_app_fixture(_options: FixtureOptions) -> axum::Router {
    let app: axum::Router = open_tab_server::app().await;
    app
}


/*
pub async fn send_get_request(path: &str) -> hyper::Response<UnsyncBoxBody<hyper::body::Bytes, axum::Error>>
{
    let app: axum::Router = open_tab_server::app().await;
    app
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
}

pub async fn send_json_post_request<T>(path: &str, body: T) -> hyper::Response<UnsyncBoxBody<hyper::body::Bytes, axum::Error>> where T: serde::Serialize
{
    let app: axum::Router = open_tab_server::app().await;
    app
        .oneshot(Request::builder().uri(path).body(
            Body::from(
                serde_json::to_string(&body).unwrap()
            )
        
        ).unwrap())
        .await
        .unwrap()
}
 */