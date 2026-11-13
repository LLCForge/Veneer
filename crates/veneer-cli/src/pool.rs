//! `veneer pool add|take|list|stats`: a file-backed pool of validated profiles with usage counts.
//!
//! Every command takes an exclusive lock (`<pool>.lock`, created atomically with `create_new`) for
//! its whole read-modify-write, and writes through a temporary file that is renamed over the pool,
//! so concurrent processes never lose an update and readers never see a half-written file.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use clap::Subcommand;
use serde::{Deserialize, Serialize};
use veneer::{Constraints, Profile};

use crate::{print, ConstraintArgs, Format};

/// Largest pool file read (bytes); a profile is about 3 KB.
const MAX_POOL_BYTES: u64 = 256 * 1024 * 1024;
/// Most profiles one pool may hold.
const MAX_ENTRIES: usize = 50_000;
/// How long a command waits for the lock.
const LOCK_WAIT: Duration = Duration::from_secs(10);
/// A lock older than this belongs to a crashed process and is removed.
const STALE_LOCK: Duration = Duration::from_secs(60);

// Parsed once per process; boxing the constraints would only complicate clap's flatten.
#[allow(clippy::large_enum_variant)]
#[derive(Subcommand)]
pub(crate) enum PoolCommand {
    /// Generate validated profiles and add them to the pool.
    Add {
        #[command(flatten)]
        constraints: ConstraintArgs,
        /// Number of profiles (1-1000).
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=1000))]
        count: u32,
    },
    /// Print the least-used profile and count one more use of it.
    Take {
        /// Fail instead of returning a profile that has been used this many times.
        #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
        max_uses: Option<u32>,
    },
    /// One line per profile: index, uses, last use, seed, device, browser.
    List,
    /// Totals and usage distribution as JSON.
    Stats,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Pool {
    version: u32,
    entries: Vec<Entry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Entry {
    uses: u32,
    /// Unix seconds of the last `take`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_used: Option<u64>,
    profile: Profile,
}

/// Removes the lock file when dropped, including on early returns and errors.
struct Lock(PathBuf);

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

fn lock(path: &Path) -> Result<Lock, String> {
    let lock_path = sibling(path, ".lock");
    let start = Instant::now();
    loop {
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
        {
            Ok(mut f) => {
                let _ = writeln!(f, "{}", std::process::id());
                return Ok(Lock(lock_path));
            }
            // Windows reports a lock file that is being deleted as "access denied"; wait for it like a held lock.
            Err(e)
                if e.kind() == ErrorKind::AlreadyExists
                    || (cfg!(windows) && e.kind() == ErrorKind::PermissionDenied) =>
            {
                let stale = fs::metadata(&lock_path)
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .is_some_and(|age| age > STALE_LOCK);
                if stale {
                    // shortcut: two waiters can both judge a lock stale; only after 60 s without progress.
                    let _ = fs::remove_file(&lock_path);
                    continue;
                }
                if start.elapsed() > LOCK_WAIT {
                    return Err(format!(
                        "cannot lock {} ({e}); if no veneer process is running, remove the lock file",
                        lock_path.display()
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => return Err(format!("cannot lock {}: {e}", lock_path.display())),
        }
    }
}

fn load(path: &Path) -> Result<Pool, String> {
    match fs::metadata(path) {
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(Pool {
                version: 1,
                entries: Vec::new(),
            })
        }
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
        Ok(m) if m.len() > MAX_POOL_BYTES => {
            return Err(format!(
                "{} is larger than {MAX_POOL_BYTES} bytes",
                path.display()
            ))
        }
        Ok(_) => {}
    }
    let text =
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let pool: Pool = serde_json::from_str(&text)
        .map_err(|e| format!("{} is not a Veneer pool: {e}", path.display()))?;
    if pool.version != 1 {
        return Err(format!("unsupported pool version {}", pool.version));
    }
    Ok(pool)
}

fn save(path: &Path, pool: &Pool) -> Result<(), String> {
    let tmp = sibling(path, &format!(".tmp-{}", std::process::id()));
    let text = serde_json::to_string(pool).map_err(|e| e.to_string())?;
    let written = fs::File::create(&tmp)
        .and_then(|mut f| f.write_all(text.as_bytes()).and_then(|()| f.sync_all()))
        .and_then(|()| fs::rename(&tmp, path));
    written.map_err(|e| {
        let _ = fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

fn add(pool: &mut Pool, constraints: &Constraints, count: u32) -> Result<usize, String> {
    if pool.entries.len() + count as usize > MAX_ENTRIES {
        return Err(format!("a pool holds at most {MAX_ENTRIES} profiles"));
    }
    let list = crate::fingerprints(constraints, count, false, false).map_err(|e| e.to_string())?;
    let before = pool.entries.len();
    for fp in list {
        // The same seed and constraints give the same profile; keep one copy.
        if pool
            .entries
            .iter()
            .all(|e| e.profile.seed != fp.profile.seed)
        {
            pool.entries.push(Entry {
                uses: 0,
                last_used: None,
                profile: fp.profile,
            });
        }
    }
    Ok(pool.entries.len() - before)
}

/// Least-used entry; ties go to the one used longest ago, then to the oldest entry.
fn take(pool: &mut Pool, max_uses: Option<u32>) -> Result<&Profile, String> {
    let entry = pool
        .entries
        .iter_mut()
        .min_by_key(|e| (e.uses, e.last_used.unwrap_or(0)))
        .ok_or("the pool is empty; add profiles with `veneer pool add`")?;
    if max_uses.is_some_and(|m| entry.uses >= m) {
        return Err(format!(
            "every profile has been used at least {} time(s)",
            entry.uses
        ));
    }
    entry.uses += 1;
    entry.last_used = Some(now());
    Ok(&entry.profile)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Stats {
    profiles: usize,
    total_uses: u64,
    unused: usize,
    min_uses: u32,
    max_uses: u32,
    by_browser: BTreeMap<String, usize>,
    by_os: BTreeMap<String, usize>,
}

fn stats(pool: &Pool) -> Stats {
    let mut s = Stats {
        profiles: pool.entries.len(),
        total_uses: pool.entries.iter().map(|e| u64::from(e.uses)).sum(),
        unused: pool.entries.iter().filter(|e| e.uses == 0).count(),
        min_uses: pool.entries.iter().map(|e| e.uses).min().unwrap_or(0),
        max_uses: pool.entries.iter().map(|e| e.uses).max().unwrap_or(0),
        by_browser: BTreeMap::new(),
        by_os: BTreeMap::new(),
    };
    for e in &pool.entries {
        *s.by_browser
            .entry(e.profile.browser.id.clone())
            .or_default() += 1;
        *s.by_os.entry(e.profile.os.id.clone()).or_default() += 1;
    }
    s
}

pub(crate) fn run(file: &Path, command: PoolCommand) -> Result<ExitCode, String> {
    let _lock = lock(file)?;
    let mut pool = load(file)?;
    match command {
        PoolCommand::Add { constraints, count } => {
            let added = add(&mut pool, &constraints.into(), count)?;
            save(file, &pool)?;
            eprintln!(
                "added {added} profile(s); the pool holds {}",
                pool.entries.len()
            );
        }
        PoolCommand::Take { max_uses } => {
            let profile = take(&mut pool, max_uses)?;
            let text = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
            save(file, &pool)?;
            println!("{text}");
        }
        PoolCommand::List => {
            for (i, e) in pool.entries.iter().enumerate() {
                let p = &e.profile;
                println!(
                    "{i:>5}  uses {:>4}  last {:>10}  {}  {} / {} {} / {}",
                    e.uses,
                    e.last_used.map_or_else(|| "-".into(), |t| t.to_string()),
                    p.seed,
                    p.device.id,
                    p.browser.id,
                    p.browser.version,
                    p.locale
                );
            }
        }
        PoolCommand::Stats => print(&stats(&pool), Format::Pretty)?,
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_pool(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("veneer-pool-test-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(name);
        let _ = fs::remove_file(&path);
        path
    }

    fn with_pool<T>(path: &Path, f: impl FnOnce(&mut Pool) -> T) -> T {
        let _lock = lock(path).expect("lock");
        let mut pool = load(path).expect("load");
        let out = f(&mut pool);
        save(path, &pool).expect("save");
        out
    }

    #[test]
    fn concurrent_takes_never_lose_an_update() {
        let path = temp_pool("concurrent.json");
        let constraints = Constraints {
            seed: Some("pool".into()),
            ..Default::default()
        };
        let added = with_pool(&path, |p| add(p, &constraints, 4)).expect("add");
        assert_eq!(added, 4);
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let path = path.clone();
                std::thread::spawn(move || {
                    for _ in 0..5 {
                        with_pool(&path, |p| take(p, None).map(|_| ())).expect("take");
                    }
                })
            })
            .collect();
        for t in threads {
            t.join().expect("thread");
        }
        let pool = load(&path).expect("load");
        let s = stats(&pool);
        assert_eq!(s.total_uses, 40);
        assert_eq!((s.min_uses, s.max_uses), (10, 10), "least-used first");
        assert!(!sibling(&path, ".lock").exists());
        let again = with_pool(&path, |p| add(p, &constraints, 4)).expect("add");
        assert_eq!(again, 0, "same seeds are not added twice");
        let limited = with_pool(&path, |p| take(p, Some(10)).map(|_| ()));
        assert!(limited.is_err());
        let _ = fs::remove_file(&path);
    }
}
