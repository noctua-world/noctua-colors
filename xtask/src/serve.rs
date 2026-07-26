//! A development server with live reload.
//!
//! Deliberately tiny: a blocking thread pool over `tiny_http`, a directory of
//! static files, and one server-sent-events endpoint the page listens to.
//! Pulling in an async runtime for this would add a minute to a fresh clone's
//! first build in exchange for nothing a developer would notice.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use tiny_http::{Header, Response, Server};

use crate::ui;

/// Bumped on every rebuild. The page polls it and reloads when it changes.
pub(crate) static GENERATION: AtomicU64 = AtomicU64::new(0);

/// The snippet injected into every served page.
///
/// Injected rather than written into the site source, because live reload is a
/// development concern and shipping it in the deployed site would leave every
/// visitor holding open a connection to a server that is not there.
const RELOAD_SCRIPT: &str = r"<script>
(function () {
  var current = null;
  function poll() {
    fetch('/__generation', { cache: 'no-store' })
      .then(function (r) { return r.text(); })
      .then(function (value) {
        if (current === null) { current = value; }
        else if (value !== current) { location.reload(); }
      })
      .catch(function () { /* the server went away; keep trying */ })
      .finally(function () { setTimeout(poll, 400); });
  }
  poll();
})();
</script>
</body>";

/// Serves `directory` until the process is stopped.
///
/// Claims the port.
///
/// Separate from [`serve`] so a caller can find out whether the port is
/// available *before* committing to a long-running process. `dev` used to
/// spawn the server into a thread and carry on: a port already in use killed
/// that thread, printed one line, and left a file watcher running with
/// nothing serving — which looks exactly like success once the build output
/// has scrolled the error away.
///
/// # Errors
///
/// A port already in use, with the flag that fixes it.
pub(crate) fn bind(port: u16) -> Result<Server, String> {
    Server::http(("127.0.0.1", port))
        .map_err(|error| format!("could not listen on port {port}: {error}. Try `--port <other>`."))
}

/// Serves `directory` on an already-bound server. Never returns.
// The directory is moved into the connection threads, which outlive this
// call, so it has to be owned however it looks from the signature.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn serve(server: Server, directory: PathBuf, port: u16) {
    let server = Arc::new(server);

    ui::ok(&format!("serving http://127.0.0.1:{port}"));

    let workers: Vec<_> = (0..4)
        .map(|_| {
            let server = Arc::clone(&server);
            let directory = directory.clone();
            std::thread::spawn(move || {
                for request in server.incoming_requests() {
                    let _ = handle(request, &directory);
                }
            })
        })
        .collect();

    for worker in workers {
        let _ = worker.join();
    }
}

fn handle(request: tiny_http::Request, directory: &Path) -> std::io::Result<()> {
    let url = request.url().split('?').next().unwrap_or("/").to_owned();

    if url == "/__generation" {
        let body = GENERATION.load(Ordering::Relaxed).to_string();
        return request.respond(
            Response::from_string(body)
                .with_header(header("Content-Type", "text/plain"))
                .with_header(header("Cache-Control", "no-store")),
        );
    }

    let relative = url.trim_start_matches('/');
    let relative = if relative.is_empty() {
        "index.html"
    } else {
        relative
    };

    // Refuse anything that climbs out of the served directory.
    if relative.contains("..") {
        return request.respond(Response::from_string("no").with_status_code(400));
    }

    let path = directory.join(relative);
    let Ok(bytes) = std::fs::read(&path) else {
        return request.respond(Response::from_string("not found").with_status_code(404));
    };

    let mime = mime_for(&path);
    if mime == "text/html" {
        let text = String::from_utf8_lossy(&bytes).replace("</body>", RELOAD_SCRIPT);
        return request.respond(
            Response::from_string(text)
                .with_header(header("Content-Type", "text/html; charset=utf-8"))
                .with_header(header("Cache-Control", "no-store")),
        );
    }

    request.respond(
        Response::from_data(bytes)
            .with_header(header("Content-Type", mime))
            .with_header(header("Cache-Control", "no-store")),
    )
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes(), value.as_bytes()).expect("static header is valid")
}

fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html",
        Some("css") => "text/css; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        // Without this the playground still loads, but the browser refuses to
        // stream-compile it and says so in the console on every visit.
        Some("wasm") => "application/wasm",
        Some("woff2") => "font/woff2",
        Some("svg") => "image/svg+xml",
        Some("md") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A port that cannot be claimed has to surface as an error the caller can
    /// act on, not as a dead thread. `dev` spawns the server, so if `bind`
    /// succeeded lazily the failure would land somewhere nobody is looking —
    /// which is exactly what used to happen: a watcher ran, nothing served,
    /// and the one error line scrolled away behind the build output.
    #[test]
    fn an_occupied_port_is_an_error_with_the_flag_that_fixes_it() {
        // Claim an arbitrary free port, then ask for the same one.
        let held = Server::http(("127.0.0.1", 0)).expect("a free port exists");
        let port = held.server_addr().to_ip().expect("an ip address").port();

        let Err(error) = bind(port) else {
            panic!("binding a port already held must fail");
        };
        assert!(error.contains(&port.to_string()), "{error}");
        assert!(
            error.contains("--port"),
            "the error must say how to fix it: {error}"
        );
    }

    #[test]
    fn a_free_port_binds() {
        // Port 0 asks the OS for any free port.
        assert!(bind(0).is_ok());
    }
}
