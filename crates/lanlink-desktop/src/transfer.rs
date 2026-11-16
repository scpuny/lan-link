//! 文件传输引擎（F18–F23）：4MB 分片、seq 单调校验、重复丢弃、真实落盘、
//! 同名重命名、断点续传（ack 回退重传）、暂停/继续/取消、进度事件推送、
//! 完成后 Blake3 整文件校验。
//!
//! 传输线协议（长度前缀帧 over 握手建立的 TCP 连接）：
//!   [4B be len][JSON header][optional binary payload]
//! header.t ∈ { meta, chunk, ack, done }
//!  - meta : { name, size }                 文件开始
//!  - chunk: { seq, len, hash } + payload   分片（hash = 该片 blake3）
//!  - ack  : { seq }                       接收方确认期望的下一个 seq
//!  - done : { hash }                      发送方声明全部片已发完（整文件 blake3）
//!
//! 流控采用 stop-and-wait：每发一片等 ack；ack < 期望 seq 触发回退重传（断点续传），
//! 接收方对已确认 seq 的重复分片直接丢弃并回 ack（去重）。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub const CHUNK_SIZE: usize = 4 * 1024 * 1024;

/// 待发送文件的传输元数据（前端 SendFile 中传输真正需要的子集）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferFile {
    pub path: String,
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Frame {
    t: String,
    #[serde(default)] name: String,
    #[serde(default)] size: u64,
    #[serde(default)] seq: u32,
    #[serde(default)] len: usize,
    #[serde(default)] hash: String,
}

#[derive(Debug, Clone)]
struct TaskCtl {
    paused: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

/// 传输进度事件负载（对齐前端 TransferTask）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProgressEvent {
    pub task_id: String,
    pub name: String,
    pub size: u64,
    pub transferred: u64,
    pub progress: u32,
    pub speed: u64,
    pub seq: u32,
    pub status: String, // transferring | done | cancelled | error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 传输管理器：任务控制 + 事件发射 + 保存目录。
#[derive(Clone)]
pub struct TransferManager {
    app: tauri::AppHandle,
    save_dir: Arc<Mutex<PathBuf>>,
    tasks: Arc<Mutex<HashMap<String, TaskCtl>>>,
}

impl TransferManager {
    pub fn new(app: tauri::AppHandle, save_dir: PathBuf) -> Self {
        Self {
            app,
            save_dir: Arc::new(Mutex::new(save_dir)),
            tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn set_save_dir(&self, dir: PathBuf) {
        *self.save_dir.lock().unwrap() = dir;
    }

    fn new_task(&self, id: &str) {
        self.tasks.lock().unwrap().insert(
            id.to_string(),
            TaskCtl {
                paused: Arc::new(AtomicBool::new(false)),
                cancelled: Arc::new(AtomicBool::new(false)),
            },
        );
    }

    fn ctl(&self, id: &str) -> Option<TaskCtl> {
        self.tasks.lock().unwrap().get(id).cloned()
    }

    pub fn pause(&self, id: &str) -> bool {
        if let Some(c) = self.ctl(id) {
            c.paused.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }
    pub fn resume(&self, id: &str) -> bool {
        if let Some(c) = self.ctl(id) {
            c.paused.store(false, Ordering::Relaxed);
            return true;
        }
        false
    }
    pub fn cancel(&self, id: &str) -> bool {
        if let Some(c) = self.ctl(id) {
            c.cancelled.store(true, Ordering::Relaxed);
            return true;
        }
        false
    }
    pub fn pause_all(&self) {
        for c in self.tasks.lock().unwrap().values() {
            c.paused.store(true, Ordering::Relaxed);
        }
    }
    #[allow(dead_code)]
    pub fn clear_tasks(&self) {
        self.tasks.lock().unwrap().clear();
    }

    fn emit(&self, ev: ProgressEvent) {
        #[cfg(debug_assertions)]
        eprintln!("[lanlink-transfer] {} {} {}/{} {}", ev.task_id, ev.status, ev.transferred, ev.size, ev.progress);
        use tauri::Emitter;
        let _ = self.app.emit("ll-transfer-progress", ev);
    }

    // -----------------------------------------------------------------------
    // 发送方：依次发送 files，每片等 ack
    // -----------------------------------------------------------------------
    pub fn send_files(&self, mut stream: TcpStream, files: Vec<TransferFile>) {
        for f in files {
            let task_id = format!("{}-{}", f.name, now_millis());
            self.new_task(&task_id);
            let ctl = match self.ctl(&task_id) {
                Some(c) => c,
                None => continue,
            };
            let mut data = match File::open(&f.path) {
                Ok(x) => x,
                Err(e) => {
                    self.emit(err_event(&task_id, &f, format!("E009: 打开文件失败 {e}")));
                    continue;
                }
            };
            let total_chunks = if f.size == 0 { 1 } else { (f.size as usize + CHUNK_SIZE - 1) / CHUNK_SIZE };
            let _ = write_frame(&mut stream, &Frame { t: "meta".into(), name: f.name.clone(), size: f.size, ..Default::default() }, None);

            let mut seq: u32 = 0;
            let mut transferred: u64 = 0;
            let mut last = Instant::now();
            let mut last_bytes: u64 = 0;
            let mut speed: u64 = 0;

            while (seq as usize) < total_chunks {
                if ctl.cancelled.load(Ordering::Relaxed) {
                    self.emit(ProgressEvent {
                        task_id: task_id.clone(), name: f.name.clone(), size: f.size,
                        transferred, progress: pct(transferred, f.size), speed: 0, seq,
                        status: "cancelled".into(), error: None,
                    });
                    let _ = stream.shutdown(Shutdown::Both);
                    return;
                }
                // 暂停：每 200ms 检查，不占用 CPU
                while ctl.paused.load(Ordering::Relaxed) && !ctl.cancelled.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(200));
                }

                // 读取第 seq 片
                let offset = seq as u64 * CHUNK_SIZE as u64;
                let mut buf = vec![0u8; CHUNK_SIZE];
                if data.seek(SeekFrom::Start(offset)).is_err() {
                    break;
                }
                let n = match read_at(&mut data, &mut buf) {
                    Ok(n) => n,
                    Err(_) => break,
                };
                buf.truncate(n);
                let hash = hex(&blake3::hash(&buf).as_bytes()[..]);

                loop {
                    if write_frame(&mut stream, &Frame {
                        t: "chunk".into(), seq, len: n, hash: hash.clone(), ..Default::default()
                    }, Some(&buf)).is_err() {
                        self.emit(err_event(&task_id, &f, "E009: 连接中断".into()));
                        return;
                    }
                    let ack = match read_frame(&mut stream) {
                        Some((fr, _)) if fr.t == "ack" => fr.seq,
                        _ => {
                            self.emit(err_event(&task_id, &f, "E009: 未收到确认".into()));
                            return;
                        }
                    };
                    if ack == seq + 1 {
                        break; // 确认，进入下一片
                    }
                    // ack < seq+1：对端要求回退（丢片 / 校验失败）→ 重传当前片
                    if ctl.cancelled.load(Ordering::Relaxed) { return; }
                }

                seq += 1;
                transferred = ((seq as usize) * CHUNK_SIZE).min(f.size as usize) as u64;
                // 速率：按 500ms 窗口统计
                let now = Instant::now();
                let dt = now.duration_since(last).as_millis();
                if dt >= 500 {
                    speed = ((transferred - last_bytes) as u128 * 1000 / dt) as u64;
                    last = now;
                    last_bytes = transferred;
                }
                self.emit(ProgressEvent {
                    task_id: task_id.clone(), name: f.name.clone(), size: f.size,
                    transferred, progress: pct(transferred, f.size), speed, seq,
                    status: "transferring".into(), error: None,
                });
            }

            let file_hash = hex(&hash_file(std::path::Path::new(&f.path)).unwrap_or_default());
            let _ = write_frame(&mut stream, &Frame { t: "done".into(), hash: file_hash, ..Default::default() }, None);
            self.emit(ProgressEvent {
                task_id, name: f.name.clone(), size: f.size, transferred: f.size,
                progress: 100, speed: 0, seq: total_chunks as u32,
                status: "done".into(), error: None,
            });
        }
        let _ = stream.shutdown(Shutdown::Both);
    }

    // -----------------------------------------------------------------------
    // 接收方：落盘、seq 校验、重复丢弃、整文件校验
    // -----------------------------------------------------------------------
    pub fn recv_files(&self, mut stream: TcpStream) {
        let dir = self.save_dir.lock().unwrap().clone();
        let _ = std::fs::create_dir_all(&dir);
        loop {
            // 等待 meta 或连接关闭
            let (fr, _) = match read_frame(&mut stream) {
                Some(v) => v,
                None => break,
            };
            if fr.t != "meta" { break; }
            let task_id = format!("recv-{}-{}", fr.name, now_millis());
            self.new_task(&task_id);
            let ctl = self.ctl(&task_id).unwrap_or_else(|| TaskCtl {
                paused: Arc::new(AtomicBool::new(false)),
                cancelled: Arc::new(AtomicBool::new(false)),
            });
            let target = unique_path(&dir, &fr.name);
            let mut file = match OpenOptions::new().write(true).create(true).truncate(true).open(&target) {
                Ok(f) => f,
                Err(e) => {
                    let _ = write_frame(&mut stream, &Frame { t: "ack".into(), seq: 0, ..Default::default() }, None);
                    self.emit(ProgressEvent {
                        task_id, name: fr.name, size: fr.size, transferred: 0, progress: 0,
                        speed: 0, seq: 0, status: "error".into(),
                        error: Some(format!("E009: 无法写入 {e}")),
                    });
                    break;
                }
            };
            let total_chunks = if fr.size == 0 { 1 } else { (fr.size as usize + CHUNK_SIZE - 1) / CHUNK_SIZE };
            let mut expected: u32 = 0;
            let mut transferred: u64 = 0;
            let mut last = Instant::now();
            let mut last_bytes: u64 = 0;
            let mut speed: u64 = 0;
            let mut final_hash = String::new();

            while expected as usize <= total_chunks {
                let (ch, payload) = match read_frame(&mut stream) {
                    Some((h, p)) if h.t == "chunk" => (h, p),
                    Some((h, _)) if h.t == "done" => { final_hash = h.hash; break; }
                    _ => break,
                };
                while ctl.paused.load(Ordering::Relaxed) && !ctl.cancelled.load(Ordering::Relaxed) {
                    std::thread::sleep(Duration::from_millis(200));
                }
                if ctl.cancelled.load(Ordering::Relaxed) {
                    self.emit(ProgressEvent {
                        task_id: task_id.clone(), name: fr.name.clone(), size: fr.size,
                        transferred, progress: pct(transferred, fr.size), speed: 0, seq: expected,
                        status: "cancelled".into(), error: None,
                    });
                    let _ = stream.shutdown(Shutdown::Both);
                    return;
                }

                // F18：seq 单调校验 + 重复丢弃 + 乱序要求回退
                if ch.seq < expected {
                    // 重复分片：丢弃，回 ack(expected)
                    let _ = write_frame(&mut stream, &Frame { t: "ack".into(), seq: expected, ..Default::default() }, None);
                    continue;
                }
                if ch.seq > expected {
                    // 乱序（丢片）：回 ack(expected) 要求从期望值重传
                    let _ = write_frame(&mut stream, &Frame { t: "ack".into(), seq: expected, ..Default::default() }, None);
                    continue;
                }
                // 分片完整性
                let got = hex(&blake3::hash(&payload).as_bytes()[..]);
                if got != ch.hash {
                    let _ = write_frame(&mut stream, &Frame { t: "ack".into(), seq: expected, ..Default::default() }, None);
                    continue;
                }
                if file.seek(SeekFrom::Start(expected as u64 * CHUNK_SIZE as u64)).is_err()
                    || file.write_all(&payload).is_err() {
                    self.emit(err_event(&task_id, &TransferFile { path: target.to_string_lossy().into(), name: fr.name.clone(), size: fr.size }, "E009: 写盘失败".into()));
                    break;
                }
                let _ = file.flush();
                expected += 1;
                transferred = ((expected as usize) * CHUNK_SIZE).min(fr.size as usize) as u64;
                let _ = write_frame(&mut stream, &Frame { t: "ack".into(), seq: expected, ..Default::default() }, None);

                let now = Instant::now();
                let dt = now.duration_since(last).as_millis();
                if dt >= 500 {
                    speed = ((transferred - last_bytes) as u128 * 1000 / dt) as u64;
                    last = now;
                    last_bytes = transferred;
                }
                self.emit(ProgressEvent {
                    task_id: task_id.clone(), name: fr.name.clone(), size: fr.size,
                    transferred, progress: pct(transferred, fr.size), speed, seq: expected,
                    status: "transferring".into(), error: None,
                });
            }

            // F23：整文件 Blake3 校验
            let actual = hex(&hash_file(&target).unwrap_or_default());
            let ok = final_hash.is_empty() || actual == final_hash;
            self.emit(ProgressEvent {
                task_id, name: fr.name.clone(), size: fr.size,
                transferred: fr.size, progress: 100, speed: 0, seq: expected,
                status: if ok { "done".into() } else { "error".into() },
                error: if ok { None } else { Some("E009: 文件校验失败".into()) },
            });
        }
    }
}

// ---- 纯工具 ----

fn err_event(id: &str, f: &TransferFile, err: String) -> ProgressEvent {
    ProgressEvent {
        task_id: id.to_string(), name: f.name.clone(), size: f.size, transferred: 0,
        progress: 0, speed: 0, seq: 0, status: "error".into(), error: Some(err),
    }
}

fn pct(done: u64, total: u64) -> u32 {
    if total == 0 { 100 } else { ((done as u128 * 100 / total as u128) as u32).min(100) }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn now_millis() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn read_at(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut total = 0;
    while total < buf.len() {
        match file.read(&mut buf[total..]) {
            Ok(0) => break,
            Ok(n) => total += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(total)
}

fn hash_file(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut f = File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().as_bytes().to_vec())
}

/// 同名文件自动加后缀（F19）：report.pdf → report (1).pdf
fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let base = dir.join(name);
    if !base.exists() { return base; }
    let p = Path::new(name);
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("file");
    let ext = p.extension().and_then(|s| s.to_str());
    for i in 1..1000 {
        let candidate = match ext {
            Some(e) => dir.join(format!("{stem} ({i}).{e}")),
            None => dir.join(format!("{stem} ({i})")),
        };
        if !candidate.exists() { return candidate; }
    }
    dir.join(format!("{stem}-{}", now_millis()))
}

fn write_frame(stream: &mut TcpStream, fr: &Frame, payload: Option<&[u8]>) -> std::io::Result<()> {
    let head = serde_json::to_vec(fr).map_err(|e| std::io::Error::other(e))?;
    let total = head.len() + payload.map(|p| p.len()).unwrap_or(0);
    stream.write_all(&(total as u32).to_be_bytes())?;
    stream.write_all(&head)?;
    if let Some(p) = payload { stream.write_all(p)?; }
    stream.flush()
}

/// 返回帧头与剩余 payload（header 长度从帧头 JSON 解析，剩余即二进制负载）。
fn read_frame(stream: &mut TcpStream) -> Option<(Frame, Vec<u8>)> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).ok()?;
    let total = u32::from_be_bytes(len) as usize;
    if total == 0 || total > CHUNK_SIZE + 8192 { return None; }
    let mut all = vec![0u8; total];
    stream.read_exact(&mut all).ok()?;
    // 从头逐步找 JSON 结束位置：header 是不含原始二进制的 UTF-8 JSON 对象
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut end = 0;
    for (i, &b) in all.iter().enumerate() {
        if in_str {
            if esc { esc = false; }
            else if b == b'\\' { esc = true; }
            else if b == b'"' { in_str = false; }
            continue;
        }
        match b {
            b'"' => in_str = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 { end = i + 1; break; }
            }
            _ => {}
        }
    }
    if end == 0 { return None; }
    let fr: Frame = serde_json::from_slice(&all[..end]).ok()?;
    Some((fr, all[end..].to_vec()))
}

impl Default for Frame {
    fn default() -> Self {
        Self { t: String::new(), name: String::new(), size: 0, seq: 0, len: 0, hash: String::new() }
    }
}
