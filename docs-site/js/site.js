/* The documentation site.
 *
 * Vanilla, no build step, no dependencies. Every behaviour here is an
 * enhancement: the page is readable, navigable and complete with this file
 * blocked entirely.
 *
 * The one thing that genuinely needs script is the contrast matrix, because
 * the honest way to show what a pair measures is to measure the colors the
 * page is actually painted with — not to bake numbers into the HTML and hope
 * they still match.
 */

(function () {
  "use strict";

  var root = document.documentElement;

  /* --- Appearance and language -------------------------------------------
   *
   * Three stored preferences, each written the moment it changes and applied
   * before the first paint by the inline bootstrap in the document head.
   *
   *   noctua-mode     'light' | 'dark' | 'system'
   *   noctua-palette  a theme name
   *   noctua-locale   a BCP 47 tag matching a <link rel=alternate>
   *
   * `system` is a real, selectable state rather than the absence of a choice.
   * A two-position switch could not express it, so once a visitor had touched
   * the old toggle there was no way back to following the operating system.
   */

  var STORE = {
    mode: "noctua-mode",
    palette: "noctua-palette",
    accent: "noctua-accent",
    saturation: "noctua-saturation",
    locale: "noctua-locale",
  };

  function remember(key, value) {
    try {
      localStorage.setItem(key, value);
    } catch (e) {
      /* Private browsing. The choice still applies for this page view. */
    }
  }

  function recall(key) {
    try {
      return localStorage.getItem(key);
    } catch (e) {
      return null;
    }
  }

  var systemPrefersDark = window.matchMedia("(prefers-color-scheme: dark)");

  /* The mode the visitor asked for, which may be 'system'. */
  function chosenMode() {
    var stored = recall(STORE.mode);
    return stored === "light" || stored === "dark" ? stored : "system";
  }

  /* There is deliberately no `effectiveMode()` here any more. Resolving
   * 'system' to a concrete mode in script means the answer only exists after
   * this file has run, and anything that depended on it was therefore wrong
   * for the first paint. The stylesheet resolves the same three states in the
   * cascade, where the answer exists before anything is painted. */

  function applyMode(mode) {
    root.classList.add("theme-switching");
    if (mode === "system") {
      // No attribute at all: the generated CSS already carries a
      // prefers-color-scheme block, so removing this hands control back.
      root.removeAttribute("data-theme");
    } else {
      root.setAttribute("data-theme", mode);
    }
    remember(STORE.mode, mode);
    window.setTimeout(function () {
      root.classList.remove("theme-switching");
    }, 260);
    syncModeControl();
    measureContrast();
  }

  var modeButtons = Array.prototype.slice.call(
    document.querySelectorAll(".mode-option")
  );

  function syncModeControl() {
    var chosen = chosenMode();
    modeButtons.forEach(function (button) {
      button.setAttribute(
        "aria-pressed",
        String(button.getAttribute("data-mode") === chosen)
      );
    });
  }

  modeButtons.forEach(function (button) {
    button.addEventListener("click", function () {
      applyMode(button.getAttribute("data-mode"));
    });
  });

  /* --- Palette -----------------------------------------------------------
   *
   * Two axes, an accent hue and a saturation, resolving to one theme name.
   * Only that theme's stylesheet and swatches are ever loaded: the alternative
   * is shipping thirty-six palettes to show one, which is two megabytes of CSS
   * and two of markup before a single colour appears.
   */

  var accentSelect = document.getElementById("accent-select");
  var saturationSelect = document.getElementById("saturation-select");
  var flatSelect = document.getElementById("palette-select");
  var browser = document.getElementById("ramp-browser");

  /* The grid, emitted alongside the palette so the page never has to guess
   * which theme an (accent, saturation) pair resolves to. */
  var axes = null;

  function resolveTheme() {
    if (flatSelect) return flatSelect.value;
    if (!accentSelect || !saturationSelect || !axes) return null;
    return axes.themes[accentSelect.value + "/" + saturationSelect.value] || null;
  }

  /* Stylesheets already in the document, so a palette is fetched once.
   *
   * Two arrive before this runs: the default palette, linked in the markup, and
   * — when the visitor had chosen another one — the sheet the inline bootstrap
   * injected to stop the default flashing in first. Missing the second would
   * append a duplicate on every load. Both are read off their href rather than
   * assumed, so the naming lives in one place per file. */
  var loadedStyles = {};
  ["palette-stylesheet", "palette-stylesheet-restored"].forEach(function (id) {
    var sheet = document.getElementById(id);
    if (!sheet) return;
    var name = sheet.getAttribute("href").match(/(?:theme-)?([^/]+)\.css$/);
    if (name) loadedStyles[name[1]] = true;
  });

  function ensureStylesheet(theme, done) {
    if (loadedStyles[theme]) {
      done();
      return;
    }
    var link = document.createElement("link");
    link.rel = "stylesheet";
    // Non-default themes are emitted with the prefix; the default is already
    // in the document, so anything reaching here is a non-default.
    link.href = "tokens/css/theme-" + theme + ".css";
    // Appended last, so it wins the equal-specificity tie against the default
    // theme's `:root` block. That ordering is the whole switching mechanism.
    link.addEventListener("load", function () {
      loadedStyles[theme] = true;
      done();
    });
    link.addEventListener("error", function () {
      // A missing stylesheet would leave the page painted in the previous
      // palette with a picker claiming otherwise. Say so rather than lie.
      say(theme + ": stylesheet failed to load");
      done();
    });
    document.head.appendChild(link);
  }

  function say(text) {
    var status = document.getElementById("palette-status");
    if (status) status.textContent = text;
  }

  /* `animate` is false when this is restoring a stored choice rather than
   * acting on one. A 220ms cross-fade is right when a visitor turns a dial and
   * wrong on load, where it draws the eye to exactly the repaint this whole
   * file is trying to make invisible. */
  function applyPalette(animate) {
    var theme = resolveTheme();
    if (!theme) return;

    /* Written on every path, not just the flat-select one.
     *
     * This is the key the inline bootstrap in the document head reads to
     * restore the palette before the first paint. With two axis pickers —
     * which is what the shipped spec produces — `flatSelect` is null, so it
     * was never written, the bootstrap always found nothing, and the whole
     * render-blocking injection it performs was unreachable. Every reload
     * painted the default palette and swapped once three requests had landed.
     *
     * The two axis values are stored as well, because they are what the two
     * pickers restore; this is the resolved answer, which is what the
     * bootstrap needs and cannot compute for itself. */
    remember(STORE.palette, theme);
    if (accentSelect) remember(STORE.accent, accentSelect.value);
    if (saturationSelect) remember(STORE.saturation, saturationSelect.value);

    if (animate) root.classList.add("theme-switching");
    ensureStylesheet(theme, function () {
      root.setAttribute("data-palette", theme);
      if (animate) {
        window.setTimeout(function () {
          root.classList.remove("theme-switching");
        }, 260);
      }
      renderRamps(theme);
      measureContrast();
    });
  }

  [accentSelect, saturationSelect, flatSelect].forEach(function (select) {
    if (select) {
      select.addEventListener("change", function () {
        applyPalette(true);
      });
    }
  });

  /* --- Language ----------------------------------------------------------
   *
   * The switch is an ordinary link, so this only records the decision. It has
   * to be recorded *before* the navigation, or the bootstrap on the next page
   * would read the old preference and send the visitor straight back.
   */

  var languageSwitch = document.querySelector(".lang-switch");
  if (languageSwitch) {
    languageSwitch.addEventListener("click", function () {
      remember(STORE.locale, languageSwitch.getAttribute("data-locale"));
    });
  }

  // Landing here by any route confirms this language, so a visitor who
  // navigated deliberately is not redirected away on the next visit.
  remember(STORE.locale, root.getAttribute("lang"));

  /* Which mode's blocks are on screen is decided by CSS — see `[data-mode]`
   * in `site.css`. It was decided here, after load, which meant a dark-mode
   * visitor watched the light ramp table and the light matrix paint and then
   * disappear on every reload, and meant the page was wrong in dark mode with
   * this file blocked. Both groups are always in the document; only one is
   * ever displayed, from the first paint onward.
   *
   * Client-built groups carry the same `data-mode`, so they are covered by the
   * same rules with nothing to synchronise. */

  /* --- The palette browser, built here for every theme but the first -------
   *
   * This mirrors `ramp_table` and `swatch` in
   * `crates/noctua-docs/src/sections.rs`. Two renderers for one thing is a
   * cost, and it is paid deliberately: the page ships the default palette as
   * markup so it works without script, and building the other thirty-five
   * server-side would mean two megabytes of HTML to display one of them.
   *
   * The duplication is guarded — `the_two_ramp_renderers_agree` in
   * `crates/noctua-docs/tests/page.rs` fails the build if the Rust swatch
   * grows a `data-` attribute this does not write.
   */

  var themeCache = {};

  function labelOf(name, fallback) {
    return (browser && browser.getAttribute("data-label-" + name)) || fallback;
  }

  function buildSwatch(step, family, role) {
    // A step carries one rendition per emitted gamut, primary first.
    var color = step.renditions[0];
    var stem = family + "-" + role;

    var button = document.createElement("button");
    button.type = "button";
    button.className = "swatch";
    // The token, not the value it resolved to — matching `swatch` in
    // `sections.rs`, which paints the same way and for the same reason. Using
    // the value here would make the grid change appearance when this replaces
    // the server-rendered one.
    button.style.background = "var(--nc-" + stem + ")";
    button.dataset.stem = stem;
    button.dataset.hex = color.hex;
    button.dataset.css = color.css;
    button.dataset.l = color.oklch.l.toFixed(4);
    button.dataset.c = color.oklch.c.toFixed(4);
    button.dataset.h = color.oklch.h.toFixed(2);
    button.dataset.cr = color.relativeChroma.toFixed(3);
    button.dataset.headroom = color.chromaHeadroom.toFixed(4);
    // Light text on dark swatches and vice versa, from the step's own
    // lightness rather than guessed.
    button.dataset.ink = color.oklch.l > 0.6 ? "dark" : "light";
    button.setAttribute("aria-label", stem + ", " + color.hex);

    var roleSpan = document.createElement("span");
    roleSpan.className = "swatch-role";
    roleSpan.textContent = role;
    var hexSpan = document.createElement("span");
    hexSpan.className = "swatch-hex";
    hexSpan.textContent = color.hex;

    button.appendChild(roleSpan);
    button.appendChild(hexSpan);
    return button;
  }

  function buildRamp(title, note, steps, roleOf) {
    var ramp = document.createElement("div");
    ramp.className = "ramp";

    var head = document.createElement("div");
    head.className = "ramp-head";
    var heading = document.createElement("h4");
    heading.textContent = title;
    var meta = document.createElement("span");
    meta.className = "muted small";
    meta.textContent = note;
    head.appendChild(heading);
    head.appendChild(meta);

    var row = document.createElement("div");
    row.className = "ramp-steps";
    steps.forEach(function (step) {
      row.appendChild(buildSwatch(step, title, roleOf(step)));
    });

    ramp.appendChild(head);
    ramp.appendChild(row);
    return ramp;
  }

  function buildGroup(theme, modeName, mode) {
    var group = document.createElement("div");
    group.className = "ramp-group reveal";
    group.dataset.themeName = theme;
    group.dataset.mode = modeName;
    group.dataset.reveal = "shown";

    var ramps = document.createElement("div");
    ramps.className = "ramps";

    Object.keys(mode.families).forEach(function (family) {
      var resolved = mode.families[family];
      ramps.appendChild(
        buildRamp(
          family,
          labelOf("hue", "hue") + " " + Math.round(resolved.baseHue) + "\u00b0",
          resolved.steps,
          function (step) {
            return step.role;
          }
        )
      );
    });

    /* Every scale, keyed by stem. A categorical set spreads hues around the
     * wheel so a legend tells its series apart; an ordered one walks a hue path
     * so a reader can tell which stop is worse without one.
     *
     * The kind comes from the palette, not from the name. Testing the stem
     * against `chart` was right until a second categorical set existed, and
     * then labelled every one of them "ordered" — which is the one thing the
     * label is there to prevent. */
    Object.keys(mode.scales).forEach(function (name) {
      var scale = mode.scales[name];
      var kind =
        scale.kind !== "categorical"
          ? labelOf("ordered", "ordered")
          : scale.labelled
            ? labelOf("chart-labelled", "categorical, labelled")
            : labelOf("chart", "categorical");
      ramps.appendChild(
        buildRamp(name, kind, scale.steps, function (step) {
          return step.role;
        })
      );
    });

    var note = document.createElement("p");
    note.className = "muted small ramp-note";
    note.textContent =
      labelOf("roles", "Roles are canonical; the numbers are aliases for interop.") +
      " " +
      (browser ? browser.getAttribute("data-gamut-count") : "") +
      " " +
      labelOf("gamuts", "gamuts emitted per token.");

    group.appendChild(ramps);
    group.appendChild(note);
    return group;
  }

  /* The hero quotes one colour by number. Those numbers belong to whichever
   * palette is showing, so they move with it — the swatch itself is painted
   * from `var(--nc-color-accent)` and needs nothing. */
  function updateHero(data) {
    var light = data.light;
    if (!light || !light.families || !light.families.accent) return;

    var solid = light.families.accent.steps.filter(function (step) {
      return step.role === "solid";
    })[0];
    if (!solid) return;

    var color = solid.renditions[0];
    var hex = document.getElementById("hero-hex");
    var css = document.getElementById("hero-css");
    var chroma = document.getElementById("hero-chroma");
    if (hex) hex.textContent = color.hex;
    if (css) css.textContent = color.css;
    if (chroma) {
      chroma.textContent = Math.round(color.relativeChroma * 100) + "%";
    }
  }

  /* The palette JSON, from wherever it is furthest along.
   *
   * The inline bootstrap starts this request while the head is still parsing,
   * because this file cannot ask for it until it has resolved the palette out
   * of `axes.json` — which put three requests in series, every one of them
   * after the first paint. `theme` is checked rather than assumed: the stored
   * choice can be stale, and the wrong palette's numbers are worse than late
   * ones. A prefetch that failed resolves to null and is refetched here, where
   * a failure has somewhere to be reported.
   */
  function themeData(theme) {
    var prefetched = window.__noctuaThemeFetch;
    if (prefetched && prefetched.theme === theme && prefetched.data) {
      return prefetched.data.then(function (data) {
        return data || fetchTheme(theme);
      });
    }
    return fetchTheme(theme);
  }

  function fetchTheme(theme) {
    return fetch("tokens/json/themes/" + theme + ".json").then(function (r) {
      if (!r.ok) throw new Error(String(r.status));
      return r.json();
    });
  }

  function renderRamps(theme) {
    if (!browser) return;

    var existing = browser.querySelector("[data-theme-name]");
    if (existing && existing.getAttribute("data-theme-name") === theme) return;

    if (themeCache[theme]) {
      paint(themeCache[theme]);
      return;
    }

    themeData(theme)
      .then(function (data) {
        themeCache[theme] = data;
        paint(data);
      })
      .catch(function (error) {
        say(theme + ": could not load the palette (" + error.message + ")");
      });

    function paint(data) {
      updateHero(data);
      browser.textContent = "";
      ["light", "dark"].forEach(function (modeName) {
        if (data[modeName]) {
          browser.appendChild(buildGroup(theme, modeName, data[modeName]));
        }
      });
    }
  }

  /* --- Contrast, measured live -------------------------------------------
   *
   * APCA-W3 0.1.9, the same revision the compiler gates on. Reimplemented
   * here in twenty lines rather than shipped as a dependency, and fed from
   * `getComputedStyle` so it reports the contrast of the colors on screen.
   */

  function screenLuminance(rgb) {
    function channel(v) {
      return Math.pow(Math.max(0, Math.min(1, v / 255)), 2.4);
    }
    return (
      0.2126729 * channel(rgb[0]) +
      0.7151522 * channel(rgb[1]) +
      0.0721750 * channel(rgb[2])
    );
  }

  function apca(textRgb, backgroundRgb) {
    var BLACK_THRESHOLD = 0.022;
    var BLACK_CLAMP = 1.414;

    function softClamp(y) {
      return y > BLACK_THRESHOLD
        ? y
        : y + Math.pow(BLACK_THRESHOLD - y, BLACK_CLAMP);
    }

    var text = softClamp(screenLuminance(textRgb));
    var background = softClamp(screenLuminance(backgroundRgb));

    if (Math.abs(background - text) < 0.0005) return 0;

    var sapc, result;
    if (background > text) {
      sapc = (Math.pow(background, 0.56) - Math.pow(text, 0.57)) * 1.14;
      result = sapc < 0.1 ? 0 : sapc - 0.027;
    } else {
      sapc = (Math.pow(background, 0.65) - Math.pow(text, 0.62)) * 1.14;
      result = sapc > -0.1 ? 0 : sapc + 0.027;
    }
    return result * 100;
  }

  /* Resolves a custom property to the eight-bit channels APCA is defined on.
   *
   * The conversion is done by painting the color into a 1x1 canvas and
   * reading the pixel back, rather than by parsing the computed value. That
   * is not defensiveness, it is a correction: `getComputedStyle(el).color`
   * does **not** return an `rgb(...)` string — allow-literal: naming the CSS
   * function, not a color. A modern browser preserves the authored
   * color space, so a token declared in `oklch()` computes to the string
   * `oklch(0.948 0.0007 59.3)`.
   *
   * Reading the first three numbers out of that yields 0.948, 0.0007 and
   * 59.3 — lightness, chroma and an angle in degrees — which, fed to a
   * function expecting channels in 0-255, made every pair on this page
   * measure Lc 0.0 and report as failing. The whole matrix was wrong and
   * nothing said so.
   *
   * The canvas has no such problem: it does the conversion the browser
   * already knows how to do, clamps to what the screen can actually show
   * (which is what a screen-contrast metric is about), and keeps working for
   * whatever color syntax CSS gains next.
   */
  var probe = document.createElement("span");
  probe.setAttribute("aria-hidden", "true");
  probe.style.display = "none";
  document.body.appendChild(probe);

  var canvas = document.createElement("canvas");
  canvas.width = 1;
  canvas.height = 1;
  var pixel = canvas.getContext("2d", { willReadFrequently: true });

  function channels(cssColor) {
    if (!pixel) return null;
    // Reset to a known value first: an unparseable color leaves fillStyle
    // untouched, and without this the previous color would be read back as
    // though it were this one.
    pixel.fillStyle = "#000";
    pixel.fillStyle = cssColor;
    pixel.fillRect(0, 0, 1, 1);
    var data = pixel.getImageData(0, 0, 1, 1).data;
    return [data[0], data[1], data[2]];
  }

  function resolve(token) {
    probe.style.color = "";
    probe.style.color = "var(--nc-color-" + token + ")";
    var computed = window.getComputedStyle(probe).color;
    if (!computed) return null;
    return channels(computed);
  }

  function measureContrast() {
    document.querySelectorAll(".contrast-row").forEach(function (row) {
      var foreground = resolve(row.getAttribute("data-fg"));
      var background = resolve(row.getAttribute("data-bg"));
      var cell = row.querySelector(".measured");
      if (!foreground || !background || !cell) return;

      var lc = Math.abs(apca(foreground, background));
      var minimum = parseFloat(row.getAttribute("data-min"));
      var passes = lc >= minimum;

      /* Three states, not two. The compiler grades each pair hard or soft,
       * and a soft pair falling short is a judgement call a component can
       * mitigate — an accent used as text can be made bolder or larger, or
       * not used as text. Marking that with the same cross as body text
       * failing its floor would erase the distinction the gates exist to
       * make, and would tell a reader this palette is broken when the
       * compiler says it ships. */
      var soft = row.getAttribute("data-severity") === "warn";
      var state = passes ? "pass" : soft ? "warn" : "fail";

      // A symbol as well as a colour: a red number and a green number look
      // identical to a deuteranope, which is the whole subject of this site.
      var mark = { pass: "✓ ", warn: "! ", fail: "✗ " }[state];

      /* Four decimals, not one. A pair that misses by 0.0003 Lc printed as
       * "45.0" beside a failure mark, which reads as a contradiction — the
       * number said it met the target and the mark said it did not. Four is the
       * precision the palette is quantized at, so nothing shown here can round
       * across a threshold it did not cross. */
      cell.textContent = mark + lc.toFixed(4);
      row.setAttribute("data-state", state);
      cell.setAttribute(
        "title",
        passes
          ? "meets the " + minimum + " Lc target"
          : soft
            ? "short of the " + minimum + " Lc target, which this pair may be"
            : "short of the " + minimum + " Lc target"
      );
    });
  }

  /* --- Swatch detail ----------------------------------------------------- */

  var detail = document.getElementById("detail");

  function openDetail(swatch) {
    if (!detail) return;
    var data = swatch.dataset;

    // The panel is built here, so its wording cannot come from the markup.
    // The generator puts each label on the container as a data attribute,
    // already in the page's language.
    function label(name, fallback) {
      return detail.getAttribute("data-label-" + name) || fallback;
    }

    var formats = [
      ["hex", data.hex],
      ["OKLCH", data.css],
      ["CSS var", "var(--nc-" + data.stem + ")"],
      ["Tailwind", "bg-" + data.stem],
      ["Rust", data.stem.replace(/-/g, "::").toUpperCase()],
    ];

    detail.innerHTML = "";
    var grid = document.createElement("div");
    grid.className = "detail-grid";

    var swatchBox = document.createElement("div");
    swatchBox.className = "detail-swatch";
    swatchBox.style.background = data.css;
    grid.appendChild(swatchBox);

    var facts = document.createElement("dl");
    [
      [label("token", "token"), "--nc-" + data.stem],
      [label("lightness", "lightness"), data.l],
      [label("chroma", "chroma"), data.c],
      [label("hue", "hue"), data.h + "°"],
      [
        label("relative", "relative chroma"),
        (parseFloat(data.cr) * 100).toFixed(1) + "%",
      ],
      [label("headroom", "to gamut edge"), data.headroom],
    ].forEach(function (pair) {
      var dt = document.createElement("dt");
      dt.textContent = pair[0];
      var dd = document.createElement("dd");
      dd.textContent = pair[1];
      facts.appendChild(dt);
      facts.appendChild(dd);
    });
    grid.appendChild(facts);

    var copies = document.createElement("div");
    var copyHeading = document.createElement("p");
    copyHeading.className = "muted small";
    copyHeading.textContent = detail.getAttribute("data-label-copy") || "Copy as";
    copies.appendChild(copyHeading);

    var row = document.createElement("div");
    row.className = "copy-row";
    formats.forEach(function (format) {
      var button = document.createElement("button");
      button.type = "button";
      button.className = "copy-button";
      button.textContent = format[0];
      button.addEventListener("click", function () {
        copy(format[1], button);
      });
      row.appendChild(button);
    });
    copies.appendChild(row);
    grid.appendChild(copies);

    var close = document.createElement("button");
    close.type = "button";
    close.className = "detail-close";
    close.setAttribute(
      "aria-label",
      detail.getAttribute("data-label-close") || "Close"
    );
    close.textContent = "×";
    close.addEventListener("click", closeDetail);

    detail.appendChild(grid);
    detail.appendChild(close);
    detail.hidden = false;
    lastFocused = swatch;
    close.focus();
  }

  var lastFocused = null;

  function closeDetail() {
    if (!detail) return;
    detail.hidden = true;
    if (lastFocused) lastFocused.focus();
  }

  function copy(text, button) {
    function done() {
      button.dataset.copied = "true";
      window.setTimeout(function () {
        delete button.dataset.copied;
      }, 1200);
    }
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(text).then(done, fallback);
    } else {
      fallback();
    }
    function fallback() {
      // Older browsers, and any context where the clipboard API is blocked.
      var field = document.createElement("textarea");
      field.value = text;
      field.setAttribute("readonly", "");
      field.style.position = "fixed";
      field.style.opacity = "0";
      document.body.appendChild(field);
      field.select();
      try {
        document.execCommand("copy");
        done();
      } catch (e) {
        /* Nothing sensible left to try. */
      }
      document.body.removeChild(field);
    }
  }

  document.addEventListener("click", function (event) {
    var swatch = event.target.closest(".swatch");
    if (swatch) openDetail(swatch);
  });

  document.addEventListener("keydown", function (event) {
    if (event.key === "Escape") closeDetail();
  });

  /* --- Context filter ----------------------------------------------------
   *
   * Three hundred and fifty contexts is more than a reader scans. This narrows
   * them by substring and hides a group that has nothing left, so the answer to
   * "is there a context called X" takes one keystroke rather than a scroll.
   *
   * An enhancement, like everything else here: without it the groups are all
   * open and every chip is on the page. Its wording comes off the input as data
   * attributes, because the page is built once per language and a string in
   * this file would be English on both.
   */

  var contextFilter = document.getElementById("context-filter");
  var contextCount = document.getElementById("context-count");

  if (contextFilter) {
    var chips = Array.prototype.slice.call(
      document.querySelectorAll(".context-chip[data-slot]")
    );
    var groups = Array.prototype.slice.call(
      document.querySelectorAll(".context-group")
    );
    var countTemplate = contextCount ? contextCount.textContent : "";

    contextFilter.addEventListener("input", function () {
      var query = contextFilter.value.trim().toLowerCase();
      var shown = 0;

      chips.forEach(function (chip) {
        var matches =
          !query || chip.getAttribute("data-slot").indexOf(query) !== -1;
        chip.hidden = !matches;
        if (matches) shown += 1;
      });

      groups.forEach(function (group) {
        var any = Array.prototype.some.call(
          group.querySelectorAll(".context-chip[data-slot]"),
          function (chip) {
            return !chip.hidden;
          }
        );
        group.hidden = !any;
        // A filtered group opens itself, so a match is never hidden behind a
        // closed summary the reader has to think to expand.
        if (any && query) group.open = true;
      });

      if (!contextCount) return;
      if (!query) {
        contextCount.textContent = countTemplate;
      } else if (shown === 0) {
        contextCount.textContent =
          contextFilter.getAttribute("data-label-none") || "no match";
      } else {
        contextCount.textContent =
          shown +
          " " +
          (contextFilter.getAttribute("data-label-matches") || "matching");
      }
    });
  }

  /* --- Integration tabs --------------------------------------------------
   *
   * Full keyboard support, per the ARIA tabs pattern: arrows move between
   * tabs, Home and End jump to the ends.
   */

  var tabs = Array.prototype.slice.call(document.querySelectorAll(".tab"));

  function selectTab(tab) {
    tabs.forEach(function (other) {
      var selected = other === tab;
      other.setAttribute("aria-selected", String(selected));
      other.tabIndex = selected ? 0 : -1;
      var panel = document.getElementById(
        other.getAttribute("aria-controls")
      );
      if (panel) panel.hidden = !selected;
    });
  }

  tabs.forEach(function (tab, index) {
    tab.addEventListener("click", function () {
      selectTab(tab);
    });
    tab.addEventListener("keydown", function (event) {
      var next = null;
      if (event.key === "ArrowRight") next = tabs[(index + 1) % tabs.length];
      if (event.key === "ArrowLeft")
        next = tabs[(index - 1 + tabs.length) % tabs.length];
      if (event.key === "Home") next = tabs[0];
      if (event.key === "End") next = tabs[tabs.length - 1];
      if (next) {
        event.preventDefault();
        selectTab(next);
        next.focus();
      }
    });
  });

  /* --- Reveal on scroll --------------------------------------------------
   *
   * The pending state is applied here rather than in CSS. Set in the
   * stylesheet, a script failure would leave the whole page invisible.
   *
   * Only what is below the fold is ever hidden. Marking every `.reveal`
   * pending — including the ones the browser had already painted — made the
   * top of the page vanish the moment this file ran and fade back in one
   * observer callback later, on every single load. An element already on
   * screen has nothing to reveal: it has been seen.
   */

  var reveals = Array.prototype.slice.call(document.querySelectorAll(".reveal"));

  if ("IntersectionObserver" in window) {
    reveals = reveals.filter(function (element) {
      return element.getBoundingClientRect().top >= window.innerHeight;
    });
    reveals.forEach(function (element) {
      element.dataset.reveal = "pending";
    });

    var observer = new IntersectionObserver(
      function (entries) {
        entries.forEach(function (entry) {
          if (entry.isIntersecting) {
            entry.target.dataset.reveal = "shown";
            observer.unobserve(entry.target);
          }
        });
      },
      { rootMargin: "0px 0px -10% 0px", threshold: 0.05 }
    );

    reveals.forEach(function (element) {
      observer.observe(element);
    });
  }

  /* --- Start ------------------------------------------------------------- */

  syncModeControl();
  measureContrast();

  /* The palette grid, and the visitor's stored choice.
   *
   * Fetched rather than inlined: it is a few hundred bytes that only matter
   * once someone touches a control, and the page is already correct without
   * it — the default palette is server-rendered. */
  if (accentSelect || flatSelect) {
    var storedAccent = recall(STORE.accent);
    var storedSaturation = recall(STORE.saturation);
    var storedPalette = recall(STORE.palette);

    fetch("tokens/json/axes.json")
      .then(function (response) {
        return response.ok ? response.json() : null;
      })
      .catch(function () {
        return null;
      })
      .then(function (data) {
        axes = data;

        var changed = false;
        if (accentSelect && storedAccent) {
          // Only honour a stored value the page still offers; an accent
          // removed from the spec must not leave the picker showing nothing.
          if (Array.prototype.some.call(accentSelect.options, function (o) {
            return o.value === storedAccent;
          })) {
            changed = changed || accentSelect.value !== storedAccent;
            accentSelect.value = storedAccent;
          }
        }
        if (saturationSelect && storedSaturation) {
          if (Array.prototype.some.call(saturationSelect.options, function (o) {
            return o.value === storedSaturation;
          })) {
            changed = changed || saturationSelect.value !== storedSaturation;
            saturationSelect.value = storedSaturation;
          }
        }
        if (flatSelect && storedPalette) {
          changed = changed || flatSelect.value !== storedPalette;
          flatSelect.value = storedPalette;
        }

        /* Recorded even when nothing changed. This is the key the inline
         * bootstrap reads to restore the palette before the first paint, and a
         * visitor who never touches a control would otherwise never have one —
         * so the *next* reload would have nothing to restore and would paint
         * the default. Writing the resolved name here means one visit is
         * enough. `applyPalette` writes it too, for the path where a choice is
         * actually made. */
        var resolved = resolveTheme();
        if (resolved) remember(STORE.palette, resolved);

        // Only repaint when the stored choice differs from what was rendered.
        // Never animated: this is a restore, not a decision, and cross-fading
        // it turns an invisible correction into a visible one.
        if (changed) applyPalette(false);
      });
  }

  // The operating system can change while the page is open — at sunset, on a
  // schedule, or because the visitor changed it in another window. That only
  // matters while 'system' is the chosen mode.
  systemPrefersDark.addEventListener("change", function () {
    if (chosenMode() === "system") {
      measureContrast();
    }
  });
})();
