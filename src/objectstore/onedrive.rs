use std::io::Read;
use std::sync::{Arc, RwLock};
use hyper::client::Client;
use hyper::header::ContentType;
use json;
use ::config::Config;

const DESKTOP_APP_URI: &'static str = "https%3A%2F%2Flogin.live.com%2Foauth20_desktop.srf";
const SCOPE: &'static str = "onedrive.readwrite%20offline_access";

pub struct OneDriveClient<TConfig: Config> {
    client: Client,
    client_id: String,
    config: Arc<RwLock<TConfig>>,
    access_token: String,
    refresh_token: String,
    expires_in: u32,
    user_id: String,
}

impl<TConfig: Config> OneDriveClient<TConfig> {
    pub fn new(client_id: String, config: Arc<RwLock<TConfig>>) -> OneDriveClient<TConfig> {
        let refresh_token =
            config.read().unwrap().get_str("onedrive.refresh_token").unwrap_or_default();
        OneDriveClient {
            client: Client::new(),
            client_id: client_id,
            config: config,
            access_token: String::new(),
            refresh_token: refresh_token,
            expires_in: 0,
            user_id: String::new(),
        }
    }

    pub fn access_test(&mut self) {
        if self.refresh_token.is_empty() {
            let code = {
                self.config.read().unwrap().get_str("onedrive.code").unwrap_or_default()
            };
            if !code.is_empty() {
                self.config.write().unwrap().delete("onedrive.code");
                let post_body = format!("client_id={client_id}&redirect_uri={redirect_uri}&grant_type=authorization_code&code={code}",
                                        client_id = self.client_id,
                                        redirect_uri = DESKTOP_APP_URI,
                                        code = code);
                if self._update_access_token(post_body).is_ok() {
                    return;
                }
            }

            // TODO: panic以外にいい方法あれば...
            let ep = format!("https://login.live.com/oauth20_authorize.\
                              srf?client_id={client_id}&scope={scope}&response_type=code&redirect_uri={redirect_uri}",
                             client_id = self.client_id,
                             scope = SCOPE,
                             redirect_uri = DESKTOP_APP_URI);
            self.config.write().unwrap().set("onedrive.code", ep);
            panic!("required authorization_code");
        }

        // refresh tokenを元にaccess tokenを取得
        self.update_access_token().unwrap();
    }

    fn update_access_token(&mut self) -> Result<(), ()> {
        let post_body = format!("client_id={client_id}&redirect_uri={redirect_uri}&refresh_token={refresh_token}&grant_type=refresh_token",
                                client_id = self.client_id,
                                redirect_uri = DESKTOP_APP_URI,
                                refresh_token = self.refresh_token);
        self._update_access_token(post_body)
    }

    fn _update_access_token(&mut self, body: String) -> Result<(), ()> {
        let mut res_body = String::new();
        try!(self.client
            .post("https://login.live.com/oauth20_token.srf")
            .header(ContentType::form_url_encoded())
            .body(body.as_str())
            .send()
            .map_err(|_| ())
            .and_then(|mut ret| ret.read_to_string(&mut res_body).map(|_| ()).map_err(|_| ())));
        json::parse(res_body.as_str())
            .map_err(|_| ())
            .and_then(|v| {
                self.user_id = try!(v["user_id"].as_str().ok_or(())).to_string();
                self.expires_in = try!(v["expires_in"].as_u32().ok_or(()));
                self.access_token = try!(v["access_token"].as_str().ok_or(())).to_string();
                self.refresh_token = try!(v["refresh_token"].as_str().ok_or(())).to_string();
                self.config.write().unwrap().set("onedrive.refresh_token", &self.refresh_token);
                Ok(())
            })
    }
}
