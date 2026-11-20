use crate::ProfileResponse;
use async_trait::async_trait;

#[async_trait]
pub trait SessionProvider: Send + Sync {
    fn name(&self) -> &str;
    fn url(&self) -> &str;
    async fn has_joined(&self, username: &str, server_id: &str) -> Option<ProfileResponse> {
        let url = format!(
            "{}hasJoined?username={}&serverId={}",
            self.url(),
            username,
            server_id
        );
        let resp = reqwest::get(&url).await.ok()?;
        if resp.status().is_success() {
            resp.json::<ProfileResponse>().await.ok()
        } else {
            None
        }
    }

    async fn profile(&self, uuid: &str, unsigned: bool) -> Option<ProfileResponse> {
        let url = format!("{}profile/{}?unsigned={}", self.url(), uuid, unsigned);
        let resp = reqwest::get(&url).await.ok()?;
        if resp.status().is_success() {
            resp.json::<ProfileResponse>().await.ok()
        } else {
            None
        }
    }
}

pub struct MojangSessionProvider;

#[async_trait]
impl SessionProvider for MojangSessionProvider {
    fn name(&self) -> &'static str {
        "Mojang"
    }
    fn url(&self) -> &'static str {
        "https://sessionserver.mojang.com/session/minecraft/"
    }
}

pub struct CustomSessionProvider {
    name: String,
    url: String,
}

impl CustomSessionProvider {
    pub fn new(name: String, mut url: String) -> Self {
        if !url.ends_with('/') {
            url.push('/');
        }
        Self { name, url }
    }
}

#[async_trait]
impl SessionProvider for CustomSessionProvider {
    fn name(&self) -> &str {
        &self.name
    }
    fn url(&self) -> &str {
        &self.url
    }
}
