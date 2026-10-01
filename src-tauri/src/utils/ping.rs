use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Default)]
pub struct ServerPing {
    pub online: Option<i64>,
    pub max: Option<i64>,
    pub players: Vec<String>,
    pub motd: Option<String>,
    pub version_name: Option<String>,
    pub protocol: Option<i64>,
    pub latency_ms: u64,
}

#[derive(Deserialize)]
struct StatusResp {
    players: Option<Players>,
    description: Option<serde_json::Value>,
    version: Option<Version>,
}

#[derive(Deserialize)]
struct Players {
    online: Option<i64>,
    max: Option<i64>,
    sample: Option<Vec<Sample>>,
}

#[derive(Deserialize)]
struct Sample {
    name: Option<String>,
}

#[derive(Deserialize)]
struct Version {
    name: Option<String>,
    protocol: Option<i64>,
}

fn write_varint(out: &mut Vec<u8>, value: i64) {
    let mut v = value;
    loop {
        let b = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            out.push(b | 0x80);
        } else {
            out.push(b);
            break;
        }
    }
}

fn read_varint(buffer: &[u8], pos: &mut usize) -> Option<i64> {
    let mut value: i64 = 0;
    let mut shift = 0;
    loop {
        let b = *buffer.get(*pos)?;
        *pos += 1;
        value |= ((b & 0x7F) as i64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return None;
        }
    }
    Some(value)
}

fn strip_motd(value: &serde_json::Value) -> String {
    if let Some(txt) = value.get("text") {
        txt.as_str().map(|s| s.to_string()).unwrap_or_default()
    } else {
        value.as_str().map(|s| s.to_string()).unwrap_or_default()
    }
}

/// Modern (1.7+) Minecraft server-list status ping.
pub async fn ping(host: &str, port: u16) -> Result<ServerPing, String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    let wait = std::time::Duration::from_secs(3);
    let start = std::time::Instant::now();

    let mut sock = tokio::time::timeout(wait, TcpStream::connect((host, port)))
        .await
        .map_err(|_| "ping timed out".to_string())?
        .map_err(|e| e.to_string())?;

    let protocol = 758i64;
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0);
    write_varint(&mut handshake, protocol);
    write_varint(&mut handshake, host.len() as i64);
    handshake.extend_from_slice(host.as_bytes());
    handshake.extend_from_slice(&port.to_be_bytes());
    handshake.push(1);

    let mut packet = Vec::new();
    write_varint(&mut packet, handshake.len() as i64);
    packet.extend_from_slice(&handshake);
    tokio::time::timeout(wait, sock.write_all(&packet))
        .await
        .map_err(|_| "ping write timed out".to_string())?
        .map_err(|e| e.to_string())?;

    tokio::time::timeout(wait, sock.write_all(&[0x01, 0x00]))
        .await
        .map_err(|_| "ping write timed out".to_string())?
        .map_err(|e| e.to_string())?;

    // read varint packet length byte by byte
    let mut len_bytes = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        tokio::time::timeout(wait, sock.read(&mut byte))
            .await
            .map_err(|_| "ping read timed out".to_string())?
            .map_err(|e| e.to_string())?;
        len_bytes.push(byte[0]);
        if byte[0] & 0x80 == 0 {
            break;
        }
    }
    let mut p = 0;
    let len = read_varint(&len_bytes, &mut p).unwrap_or(0);
    if len <= 0 || len > 1 << 20 {
        return Err("bad ping response length".into());
    }
    let mut buf = vec![0u8; len as usize];
    tokio::time::timeout(wait, sock.read_exact(&mut buf))
        .await
        .map_err(|_| "ping read timed out".to_string())?
        .map_err(|e| e.to_string())?;

    let mut p = 0;
    let _packet_id = read_varint(&buf, &mut p);
    let json_len = read_varint(&buf, &mut p).map(|l| l as usize).unwrap_or(0);
    if p + json_len > buf.len() {
        return Err("bad ping json length".into());
    }
    let json = &buf[p..p + json_len];
    let status: StatusResp =
        serde_json::from_slice(json).map_err(|e| format!("bad status json: {e}"))?;

    Ok(ServerPing {
        online: status.players.as_ref().and_then(|p| p.online),
        max: status.players.as_ref().and_then(|p| p.max),
        players: status
            .players
            .as_ref()
            .and_then(|p| p.sample.as_ref())
            .map(|s| s.iter().filter_map(|x| x.name.clone()).collect())
            .unwrap_or_default(),
        motd: status.description.as_ref().map(strip_motd),
        version_name: status.version.as_ref().and_then(|v| v.name.clone()),
        protocol: status.version.as_ref().and_then(|v| v.protocol),
        latency_ms: start.elapsed().as_millis() as u64,
    })
}
