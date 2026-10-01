pub mod bukkit;
pub mod curseforge;
pub mod fabric;
pub mod forge;
pub mod modrinth;
pub mod paper;
pub mod purpur;
pub mod vanilla;

pub fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("mc-server-manager/0.1.0")
        .connect_timeout(std::time::Duration::from_secs(10))
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap_or_default()
}
