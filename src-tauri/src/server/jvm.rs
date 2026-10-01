use crate::server::manager::ServerMeta;

/// Aikar's default flags — the community-standard tuning for Paper and its
/// forks (https://mcflags.emc.gs). Uses a throughput-first G1GC configuration
/// that is safe on Java 17+.
pub const AIKAR_FLAGS: &str = "-XX:+UseG1GC \
-XX:+ParallelRefProcEnabled \
-XX:MaxGCPauseMillis=200 \
-XX:+UnlockExperimentalVMOptions \
-XX:+DisableExplicitGC \
-XX:+AlwaysPreTouch \
-XX:G1NewSizePercent=30 \
-XX:G1MaxNewSizePercent=40 \
-XX:G1HeapRegionSize=8M \
-XX:G1ReservePercent=20 \
-XX:G1HeapWastePercent=5 \
-XX:G1MixedGCCountTarget=4 \
-XX:InitiatingHeapOccupancyPercent=15 \
-XX:G1MixedGCLiveThresholdPercent=90 \
-XX:G1RSetUpdatingPauseTimePercent=5 \
-XX:SurvivorRatio=32 \
-XX:+PerfDisableSharedMem \
-XX:MaxTenuringThreshold=1 \
-XX:+UseStringDeduplication";

/// Conservative G1GC tuning for vanilla / Forge / Fabric. Keeps garbage
/// collection predictable without the experimental Paper-only options.
pub const VANILLA_FLAGS: &str = "-XX:+UseG1GC \
-XX:MaxGCPauseMillis=100 \
-XX:+UnlockExperimentalVMOptions \
-XX:+DisableExplicitGC \
-XX:+ParallelRefProcEnabled";

/// Resolves the extra JVM arguments for a server, honouring the preset.
/// Returns an empty vector for "auto" and "none" (the default, bare launch).
pub fn extra_args(meta: &ServerMeta) -> Vec<String> {
    match meta.jvm_preset.as_str() {
        "aikar" => AIKAR_FLAGS.split_whitespace().map(str::to_string).collect(),
        "vanilla" => VANILLA_FLAGS
            .split_whitespace()
            .map(str::to_string)
            .collect(),
        "custom" => meta
            .jvm_args
            .split_whitespace()
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// The full java command line for a server, as shown in the UI so the user can
/// see exactly what will be executed.
pub fn full_command(java: &str, meta: &ServerMeta, server_dir: &std::path::Path) -> String {
    let mut parts: Vec<String> = vec![java.to_string()];
    parts.push(format!("-Xms{}M", meta.min_ram));
    parts.push(format!("-Xmx{}M", meta.max_ram));
    parts.extend(extra_args(meta));
    parts.push("-jar".into());
    parts.push(server_dir.join(&meta.jar).to_string_lossy().to_string());
    parts.push("nogui".into());
    parts.join(" ")
}
