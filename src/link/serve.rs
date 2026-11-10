// SPDX-FileCopyrightText: 2026 Yanik Fuchs <yanik.fuchs@me.com>
// SPDX-License-Identifier: GPL-3.0-only

//! Serves CIAs over HTTP in the local network for FBI's "Remote Install → Scan QR Code": the QR
//! code holds one URL per line, and FBI downloads each one and installs it while it downloads. So
//! a 3DS without vcinject Link gets it the first time, or any CIA. Modelled on vcinject's
//! `serve.py`.

use std::{
    fs::File,
    io::{self, BufRead, BufReader, Read, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

/// The port tried first; another free one is taken when it is in use
pub const DEFAULT_PORT: u16 = 8080;
const CHUNK: usize = 256 * 1024;
/// How often the server looks whether it should stop while it waits for FBI
const POLL: Duration = Duration::from_millis(100);

/// What a URL serves
#[derive(Debug, Clone)]
pub enum Source {
    /// a CIA the app ships, e.g. vcinject Link
    Bytes(&'static [u8]),
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub struct Served {
    /// the name shown, e.g. "vcinject Link 13"
    pub name: String,
    pub source: Source,
}

impl Served {
    fn size(&self) -> io::Result<u64> {
        match &self.source {
            Source::Bytes(bytes) => Ok(bytes.len() as u64),
            Source::File(path) => Ok(std::fs::metadata(path)?.len()),
        }
    }

    fn reader(&self) -> io::Result<Box<dyn Read + Send>> {
        Ok(match &self.source {
            Source::Bytes(bytes) => Box::new(*bytes),
            Source::File(path) => Box::new(File::open(path)?),
        })
    }
}

/// A download by a console
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Download {
    pub client: IpAddr,
    /// index of the file in what is served
    pub file: usize,
    pub sent: u64,
    pub total: u64,
    pub state: DownloadState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadState {
    Running,
    /// sent completely; FBI finishes the install on its own
    Done,
    /// the console closed the connection halfway
    Aborted,
}

/// The server; it stops when dropped
#[derive(Debug)]
pub struct CiaServer {
    host: IpAddr,
    port: u16,
    files: Arc<Vec<(String, Served)>>,
    downloads: Arc<Mutex<Vec<Download>>>,
    stop: Arc<AtomicBool>,
}

/// The address other devices in the network reach this computer at: the one of the interface a
/// packet to a private address would leave by (no packet is sent). Loopback without a network.
pub fn lan_ip() -> IpAddr {
    UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))
        .and_then(|socket| {
            socket.connect((Ipv4Addr::new(10, 255, 255, 255), 1))?;
            socket.local_addr()
        })
        .map_or(IpAddr::V4(Ipv4Addr::LOCALHOST), |addr| addr.ip())
}

/// The path of a file in the URLs: plain ASCII, so that the QR code stays small and FBI reads it
fn url_path(index: usize, name: &str) -> String {
    // every run of other characters becomes one "_"
    let mut stem = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '-') {
            stem.push(c);
        } else if !stem.ends_with('_') {
            stem.push('_');
        }
    }
    let mut stem = stem.trim_matches('_').to_string();
    if stem.is_empty() {
        stem = "title".to_string();
    }
    format!("/{index}/{stem}.cia")
}

impl CiaServer {
    /// Starts serving `files` on every interface, at [`DEFAULT_PORT`] or a free port
    pub fn start(files: Vec<Served>) -> io::Result<Self> {
        Self::start_at(files, lan_ip(), DEFAULT_PORT)
    }

    fn start_at(files: Vec<Served>, host: IpAddr, port: u16) -> io::Result<Self> {
        if files.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "nothing to serve",
            ));
        }
        for file in &files {
            file.size()?;
        }

        let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
            .or_else(|_| TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)))?;
        listener.set_nonblocking(true)?;
        let port = listener.local_addr()?.port();

        let server = Self {
            host,
            port,
            files: Arc::new(
                files
                    .into_iter()
                    .enumerate()
                    .map(|(i, file)| (url_path(i, &file.name), file))
                    .collect(),
            ),
            downloads: Arc::default(),
            stop: Arc::default(),
        };

        let (files, downloads, stop) = (
            server.files.clone(),
            server.downloads.clone(),
            server.stop.clone(),
        );
        thread::spawn(move || accept(&listener, &files, &downloads, &stop));
        Ok(server)
    }

    /// One URL per file, in the order given
    pub fn urls(&self) -> Vec<String> {
        self.files
            .iter()
            .map(|(path, _)| format!("http://{}{path}", SocketAddr::new(self.host, self.port)))
            .collect()
    }

    /// What the QR code holds: the URLs, one per line
    pub fn qr_text(&self) -> String {
        self.urls().join("\n")
    }

    /// Whether other devices can reach it (there is a network)
    pub fn reachable(&self) -> bool {
        !self.host.is_loopback()
    }

    pub fn file_name(&self, index: usize) -> Option<&str> {
        self.files.get(index).map(|(_, file)| file.name.as_str())
    }

    /// The downloads so far, oldest first
    pub fn downloads(&self) -> Vec<Download> {
        self.downloads
            .lock()
            .map(|downloads| downloads.clone())
            .unwrap_or_default()
    }
}

impl Drop for CiaServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

type Files = Arc<Vec<(String, Served)>>;
type Downloads = Arc<Mutex<Vec<Download>>>;

fn accept(listener: &TcpListener, files: &Files, downloads: &Downloads, stop: &Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, client)) => {
                let (files, downloads, stop) = (files.clone(), downloads.clone(), stop.clone());
                thread::spawn(move || {
                    let _ = serve(stream, client.ip(), &files, &downloads, &stop);
                });
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => thread::sleep(POLL),
            Err(_) => thread::sleep(POLL),
        }
    }
}

/// The method and path of a request; the headers are read and dropped
fn read_request(stream: &TcpStream) -> io::Result<(String, String)> {
    let mut reader = BufReader::new(stream.take(16 * 1024));
    let mut first = String::new();
    reader.read_line(&mut first)?;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" || line == "\n" {
            break;
        }
    }

    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or_default();
    let path = target.split('?').next().unwrap_or_default().to_string();
    Ok((method, path))
}

fn serve(
    mut stream: TcpStream,
    client: IpAddr,
    files: &Files,
    downloads: &Downloads,
    stop: &AtomicBool,
) -> io::Result<()> {
    stream.set_nonblocking(false)?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(60)))?;

    let (method, path) = read_request(&stream)?;
    let found = files.iter().enumerate().find(|(_, (p, _))| *p == path);
    let (index, file) = match (method.as_str(), found) {
        ("GET" | "HEAD", Some((index, (_, file)))) => (index, file),
        ("GET" | "HEAD", None) => {
            return stream.write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
        _ => {
            return stream.write_all(
                b"HTTP/1.1 405 Method Not Allowed\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    };

    let total = file.size()?;
    let mut reader = file.reader()?;
    stream.write_all(
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: \
             {total}\r\nConnection: close\r\n\r\n"
        )
        .as_bytes(),
    )?;
    if method == "HEAD" {
        return Ok(());
    }

    let slot = {
        let mut list = downloads.lock().map_err(|_| io::Error::other("poisoned"))?;
        list.push(Download {
            client,
            file: index,
            sent: 0,
            total,
            state: DownloadState::Running,
        });
        list.len() - 1
    };
    let update = |sent: u64, state: DownloadState| {
        if let Ok(mut list) = downloads.lock() {
            list[slot].sent = sent;
            list[slot].state = state;
        }
    };

    let mut buf = vec![0; CHUNK];
    let mut sent = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        if stop.load(Ordering::Relaxed) || stream.write_all(&buf[..n]).is_err() {
            update(sent, DownloadState::Aborted);
            return Ok(());
        }
        sent += n as u64;
        update(sent, DownloadState::Running);
    }
    stream.flush()?;
    update(sent, DownloadState::Done);
    Ok(())
}

/// `text` as a QR code: black modules on white with a quiet zone of 4 modules, `scale` pixels
/// per module; its width in pixels and RGBA pixels. None when the text is too long for one.
pub fn qr_rgba(text: &str, scale: usize) -> Option<(u32, Vec<u8>)> {
    const QUIET: usize = 4;
    let code = qrcode::QrCode::new(text.as_bytes()).ok()?;
    let modules = code.width();
    let colors = code.to_colors();
    let side = (modules + 2 * QUIET) * scale;

    let mut rgba = vec![0xFF; side * side * 4];
    for y in 0..side {
        for x in 0..side {
            let (mx, my) = (
                (x / scale).checked_sub(QUIET),
                (y / scale).checked_sub(QUIET),
            );
            let dark = matches!((mx, my), (Some(mx), Some(my))
                if mx < modules && my < modules && colors[my * modules + mx] == qrcode::Color::Dark);
            if dark {
                let i = (y * side + x) * 4;
                rgba[i..i + 3].fill(0);
            }
        }
    }
    Some((u32::try_from(side).ok()?, rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(server: &CiaServer, url: &str, method: &str) -> Vec<u8> {
        let path = url.split_once(&server.port.to_string()).unwrap().1;
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, server.port)).unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: x\r\nUser-Agent: FBI\r\n\r\n"
        )
        .unwrap();
        let mut response = Vec::new();
        stream.read_to_end(&mut response).unwrap();
        response
    }

    fn body(response: &[u8]) -> &[u8] {
        let end = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        &response[end + 4..]
    }

    #[test]
    fn serves_each_file_at_its_url() {
        static LINK: &[u8] = b"vcinject link cia";
        let dir = tempfile::tempdir().unwrap();
        let game = dir.path().join("Pokémon Mystery Dungeon (Europe).cia");
        std::fs::write(&game, vec![7u8; 600_000]).unwrap();

        let server = CiaServer::start_at(
            vec![
                Served {
                    name: "vcinject Link 13".into(),
                    source: Source::Bytes(LINK),
                },
                Served {
                    name: "Pokémon Mystery Dungeon (Europe)".into(),
                    source: Source::File(game),
                },
            ],
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            0,
        )
        .unwrap();

        let urls = server.urls();
        let port = server.port;
        assert_eq!(
            urls,
            vec![
                format!("http://127.0.0.1:{port}/0/vcinject_Link_13.cia"),
                format!("http://127.0.0.1:{port}/1/Pok_mon_Mystery_Dungeon_Europe.cia"),
            ]
        );
        assert_eq!(server.qr_text(), urls.join("\n"));
        assert!(!server.reachable());

        let link = get(&server, &urls[0], "GET");
        assert!(link.starts_with(b"HTTP/1.1 200 OK\r\n"));
        assert!(String::from_utf8_lossy(&link).contains("Content-Length: 17\r\n"));
        assert_eq!(body(&link), LINK);

        let game = get(&server, &urls[1], "GET");
        assert_eq!(body(&game).len(), 600_000);

        let head = get(&server, &urls[1], "HEAD");
        assert!(String::from_utf8_lossy(&head).contains("Content-Length: 600000\r\n"));
        assert!(body(&head).is_empty());

        let missing = get(&server, &format!("http://x:{port}/2/other.cia"), "GET");
        assert!(missing.starts_with(b"HTTP/1.1 404"));

        let downloads = server.downloads();
        assert_eq!(downloads.len(), 2);
        assert_eq!(
            (downloads[1].file, downloads[1].sent, downloads[1].state),
            (1, 600_000, DownloadState::Done)
        );
    }

    #[test]
    fn a_qr_code_is_drawn_with_its_quiet_zone() {
        let (side, rgba) = qr_rgba("http://192.168.1.20:8080/0/vcinject_Link_13.cia", 2).unwrap();
        assert_eq!(rgba.len(), (side * side * 4) as usize);
        // white border, and the top left finder pattern starts dark after it
        assert_eq!(&rgba[..4], &[0xFF; 4]);
        let at = |x: u32, y: u32| rgba[((y * side + x) * 4) as usize];
        assert_eq!(at(8, 8), 0);
        assert_eq!(at(7, 8), 0xFF);
    }

    #[test]
    fn stops_when_dropped() {
        let server = CiaServer::start_at(
            vec![Served {
                name: "a".into(),
                source: Source::Bytes(b"x"),
            }],
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            0,
        )
        .unwrap();
        let port = server.port;
        drop(server);
        thread::sleep(POLL * 3);
        assert!(TcpStream::connect((Ipv4Addr::LOCALHOST, port)).is_err());
    }

    #[test]
    fn needs_something_to_serve() {
        assert!(CiaServer::start(Vec::new()).is_err());
        let missing = Served {
            name: "x".into(),
            source: Source::File(PathBuf::from("/no/such/file.cia")),
        };
        assert!(CiaServer::start(vec![missing]).is_err());
    }
}
