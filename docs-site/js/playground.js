/* The playground.
 *
 * Loads the compiler as a WebAssembly module and drives it. Everything here
 * is plumbing — reading the textarea, debouncing, painting results. Not one
 * line of color mathematics, because there is none to write: the module is
 * the same Rust the command line runs, and a second implementation in
 * JavaScript is exactly the thing this project exists to abolish.
 */

import init, {
  compile,
  check,
  emit,
  targets,
  default_spec as defaultSpec,
} from "../playground/noctua_wasm.js";

const status = document.getElementById("pg-status");

/* Wording the generator put on the page, already in the right language. The
 * fallbacks are English because that is the project's working language, and
 * a missing attribute should degrade to something readable rather than to
 * an empty string. */
const strings = document.getElementById("main");
function s(name, fallback) {
  const value = strings && strings.getAttribute("data-s-" + name);
  return value || fallback;
}
const editor = document.getElementById("pg-spec");
const errorBox = document.getElementById("pg-error");
const targetSelect = document.getElementById("pg-target");
const fileSelect = document.getElementById("pg-file");

/* Recompiling on every keystroke would run the solver mid-word, against a
 * spec that is briefly invalid, and flash an error the typist already knows
 * about. A short pause is enough for the pause after a token. */
const DEBOUNCE_MS = 250;

let timer = null;
let lastEmitted = [];

function say(text) {
  status.textContent = text;
}

function showError(message) {
  errorBox.textContent = message;
  errorBox.hidden = false;
}

function clearError() {
  errorBox.hidden = true;
  errorBox.textContent = "";
}

/* --- URL state ---------------------------------------------------------
 *
 * The spec lives in the fragment, so a link carries the whole thing and the
 * text never reaches a server. Base64 over UTF-8 bytes: `btoa` alone throws
 * on anything above U+00FF, and a spec can contain a comment in any language.
 */

function encodeSpec(text) {
  const bytes = new TextEncoder().encode(text);
  let binary = "";
  bytes.forEach(function (b) {
    binary += String.fromCharCode(b);
  });
  return btoa(binary).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function decodeSpec(encoded) {
  try {
    const padded = encoded.replace(/-/g, "+").replace(/_/g, "/");
    const binary = atob(padded);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
    return new TextDecoder().decode(bytes);
  } catch (e) {
    // A truncated or hand-edited link. Falling back to the shipped spec beats
    // showing an empty editor and an error nobody can act on.
    return null;
  }
}

/* --- Rendering ---------------------------------------------------------- */

function renderRamps(palette) {
  const body = document.getElementById("pg-body-ramps");
  body.textContent = "";

  Object.keys(palette.themes).forEach(function (themeName) {
    const modes = palette.themes[themeName];
    Object.keys(modes).forEach(function (modeName) {
      const mode = modes[modeName];

      const heading = document.createElement("h3");
      heading.className = "pg-ramp-heading";
      heading.textContent = themeName + " / " + modeName;
      body.appendChild(heading);

      Object.keys(mode.families).forEach(function (familyName) {
        const row = document.createElement("div");
        row.className = "pg-ramp";

        const label = document.createElement("span");
        label.className = "pg-ramp-label";
        label.textContent = familyName;
        row.appendChild(label);

        const strip = document.createElement("div");
        strip.className = "pg-strip";
        mode.families[familyName].steps.forEach(function (step) {
          // A step carries one rendition per emitted gamut, primary first.
          // The first is the one this palette is authored against, and the
          // one the rest of the page is painted in.
          const rendition = step.renditions[0];
          const cell = document.createElement("div");
          cell.className = "pg-cell";
          cell.style.background = rendition.hex;
          cell.title =
            step.role +
            "  " +
            rendition.hex +
            "  cr " +
            rendition.relativeChroma.toFixed(3);
          strip.appendChild(cell);
        });
        row.appendChild(strip);
        body.appendChild(row);
      });
    });
  });
}

function renderGates(report) {
  const body = document.getElementById("pg-body-gates");
  body.textContent = "";

  const summary = document.createElement("p");
  summary.className = "muted";
  const failures = report.findings.filter(function (f) {
    return f.severity === "FAIL";
  });
  const warnings = report.findings.filter(function (f) {
    return f.severity !== "FAIL";
  });
  summary.textContent =
    report.checked +
    " " +
    s("checks", "checks") +
    " · " +
    failures.length +
    " " +
    s("failing-count", "failing") +
    " · " +
    warnings.length +
    " " +
    (warnings.length === 1 ? s("warning-one", "warning") : s("warnings", "warnings"));
  body.appendChild(summary);

  if (report.findings.length === 0) {
    const clean = document.createElement("p");
    clean.textContent = s("all-passed", "Every gate passed.");
    body.appendChild(clean);
    return;
  }

  const list = document.createElement("ul");
  list.className = "pg-findings";

  failures.concat(warnings).forEach(function (finding) {
    const item = document.createElement("li");
    item.className = "pg-finding";
    item.dataset.severity = finding.severity === "FAIL" ? "fail" : "warn";

    const badge = document.createElement("span");
    badge.className = "pg-badge";
    // A word as well as a color: this is a tool about color vision.
    badge.textContent = finding.severity === "FAIL" ? "FAIL" : "warn";
    item.appendChild(badge);

    const text = document.createElement("span");
    text.textContent = "[" + finding.gate + "] " + finding.where + ": " + finding.message;
    item.appendChild(text);

    if (finding.margin !== null && finding.margin !== undefined) {
      const margin = document.createElement("span");
      margin.className = "pg-margin";
      // The margin is the useful part: how much room is left, not a verdict.
      margin.textContent = (finding.margin >= 0 ? "+" : "") + finding.margin.toFixed(4);
      item.appendChild(margin);
    }

    list.appendChild(item);
  });

  body.appendChild(list);
}

function renderEmitted() {
  const body = document.getElementById("pg-body-emit");
  body.textContent = "";

  const index = Number(fileSelect.value || 0);
  const file = lastEmitted[index];
  if (!file) {
    const none = document.createElement("p");
    none.className = "muted";
    none.textContent = s("no-files", "This target produced no files.");
    body.appendChild(none);
    return;
  }

  const pre = document.createElement("pre");
  pre.className = "pg-code";
  const code = document.createElement("code");
  // textContent, not innerHTML: generated files are data, and a spec that
  // produced a token named `<script>` must not become one.
  code.textContent = file.contents;
  pre.appendChild(code);
  body.appendChild(pre);
}

function refreshEmitted(spec) {
  const target = targetSelect.value;
  if (!target) return;

  try {
    lastEmitted = JSON.parse(emit(spec, target));
  } catch (e) {
    lastEmitted = [];
    return;
  }

  const previous = fileSelect.value;
  fileSelect.textContent = "";
  lastEmitted.forEach(function (file, index) {
    const option = document.createElement("option");
    option.value = String(index);
    option.textContent = file.path;
    fileSelect.appendChild(option);
  });
  // Keep the reader on the same file across a recompile where possible.
  fileSelect.value = previous && previous < lastEmitted.length ? previous : "0";
  renderEmitted();
}

/* --- The compile cycle -------------------------------------------------- */

function run() {
  const spec = editor.value;

  let palette;
  try {
    palette = JSON.parse(compile(spec));
  } catch (e) {
    // The message is the compiler's own diagnostic, which already says what
    // is wrong and what to change.
    showError(String(e));
    say(s("failing", "not compiling"));
    return;
  }

  clearError();
  renderRamps(palette);

  try {
    renderGates(JSON.parse(check(spec)));
  } catch (e) {
    showError(String(e));
  }

  refreshEmitted(spec);

  const themes = Object.keys(palette.themes).length;
  say(
    themes +
      " " +
      (themes === 1
        ? s("compiled-one", "theme compiled")
        : s("compiled", "themes compiled"))
  );

  window.history.replaceState(null, "", "#" + encodeSpec(spec));
}

function schedule() {
  window.clearTimeout(timer);
  timer = window.setTimeout(run, DEBOUNCE_MS);
}

/* --- Tabs --------------------------------------------------------------- */

function wireTabs() {
  const tabs = Array.prototype.slice.call(document.querySelectorAll(".pg-tabs .tab"));

  function select(tab) {
    tabs.forEach(function (other) {
      const chosen = other === tab;
      other.setAttribute("aria-selected", String(chosen));
      other.tabIndex = chosen ? 0 : -1;
      const panel = document.getElementById(other.getAttribute("aria-controls"));
      if (panel) panel.hidden = !chosen;
    });
  }

  tabs.forEach(function (tab, index) {
    tab.addEventListener("click", function () {
      select(tab);
    });
    tab.addEventListener("keydown", function (event) {
      let next = null;
      if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length];
      if (event.key === "ArrowLeft") next = tabs[(index - 1 + tabs.length) % tabs.length];
      if (event.key === "Home") next = tabs[0];
      if (event.key === "End") next = tabs[tabs.length - 1];
      if (next) {
        event.preventDefault();
        select(next);
        next.focus();
      }
    });
  });
}

/* --- Start -------------------------------------------------------------- */

/* The path has to be given explicitly.
 *
 * Calling `init()` with no argument used to make wasm-bindgen resolve the
 * bundle next to this module; as of 0.2.126 it does not. `module_or_path`
 * stays `undefined`, never becomes a `fetch`, and lands in
 * `WebAssembly.instantiate(undefined)` — which fails with "Argument 0 must be
 * a buffer source", a message that says nothing about the real cause.
 *
 * Resolving against `import.meta.url` also means the bundle is found wherever
 * the site is deployed, rather than only at the server root.
 */
init({ module_or_path: new URL("../playground/noctua_wasm_bg.wasm", import.meta.url) })
  .then(function () {
    JSON.parse(targets()).forEach(function (id) {
      const option = document.createElement("option");
      option.value = id;
      option.textContent = id;
      targetSelect.appendChild(option);
    });
    targetSelect.value = "css";

    const shared = window.location.hash.slice(1);
    const restored = shared ? decodeSpec(shared) : null;
    editor.value = restored || defaultSpec();

    wireTabs();

    editor.addEventListener("input", schedule);
    targetSelect.addEventListener("change", function () {
      refreshEmitted(editor.value);
    });
    fileSelect.addEventListener("change", renderEmitted);

    document.getElementById("pg-reset").addEventListener("click", function () {
      editor.value = defaultSpec();
      run();
    });

    document.getElementById("pg-share").addEventListener("click", function () {
      const link = window.location.href;
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(link).then(
          function () {
            say(s("copied", "link copied"));
          },
          function () {
            say(s("copy-failed", "could not copy — the link is in the address bar"));
          }
        );
      } else {
        say(s("in-bar", "the link is in the address bar"));
      }
    });

    run();
  })
  .catch(function (error) {
    say(s("load-failed", "the compiler did not load"));
    showError(
      s("wasm-error", "The WebAssembly module failed to load: ") +
        String(error) +
        "\n\n" +
        s("reference-works", "The reference page works without it.")
    );
  });
