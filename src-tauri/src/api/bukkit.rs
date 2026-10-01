/// Prebuilt Spigot/CraftBukkit jars from GetBukkit (no BuildTools required).
pub fn spigot_url(mc: &str) -> String {
    format!("https://download.getbukkit.org/spigot/spigot-{mc}.jar")
}

pub fn craftbukkit_url(mc: &str) -> String {
    format!("https://download.getbukkit.org/craftbukkit/craftbukkit-{mc}.jar")
}
