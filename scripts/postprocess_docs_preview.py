from __future__ import annotations

from pathlib import Path
import json


ROOT = Path(__file__).resolve().parent.parent
SITE = ROOT / "site"
SEARCH_DIR = SITE / "search"

SHIM_MARKER = "<!-- skadi-file-preview-shim -->"

FILE_PREVIEW_SHIM = r"""<!-- skadi-file-preview-shim -->
<script>
(function () {
  if (location.protocol !== "file:") {
    return;
  }

  var NativeXHR = window.XMLHttpRequest;
  if (!NativeXHR) {
    return;
  }

  var EMPTY_SITEMAP = '<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"></urlset>';

  window.XMLHttpRequest = function SkadiFileFriendlyXHR() {
    var xhr = new NativeXHR();
    var state = {
      stub: false,
      url: "",
      readyState: 0,
      status: 0,
      statusText: "",
      responseText: "",
      responseXML: null,
    };

    var proxy = new Proxy(xhr, {
      get: function (target, prop, receiver) {
        if (prop === "open") {
          return function (method, url, async, user, password) {
            state.url = String(url);
            state.stub = state.url.toLowerCase().indexOf("sitemap.xml") !== -1;
            if (!state.stub) {
              return target.open(method, url, async, user, password);
            }
            state.readyState = 1;
            if (typeof target.onreadystatechange === "function") {
              target.onreadystatechange.call(proxy);
            }
          };
        }

        if (prop === "send") {
          return function () {
            if (!state.stub) {
              return target.send.apply(target, arguments);
            }

            state.readyState = 2;
            if (typeof target.onreadystatechange === "function") {
              target.onreadystatechange.call(proxy);
            }

            state.readyState = 3;
            if (typeof target.onreadystatechange === "function") {
              target.onreadystatechange.call(proxy);
            }

            state.readyState = 4;
            state.status = 200;
            state.statusText = "OK";
            state.responseText = EMPTY_SITEMAP;
            state.responseXML = new DOMParser().parseFromString(EMPTY_SITEMAP, "application/xml");

            if (typeof target.onreadystatechange === "function") {
              target.onreadystatechange.call(proxy);
            }
            if (typeof target.onload === "function") {
              target.onload.call(proxy, new Event("load"));
            }
            if (typeof target.onloadend === "function") {
              target.onloadend.call(proxy, new Event("loadend"));
            }
          };
        }

        if (state.stub) {
          if (prop === "readyState") return state.readyState;
          if (prop === "status") return state.status;
          if (prop === "statusText") return state.statusText;
          if (prop === "responseText") return state.responseText;
          if (prop === "responseXML") return state.responseXML;
          if (prop === "responseURL") return new URL(state.url, location.href).href;
          if (prop === "getAllResponseHeaders") {
            return function () {
              return "";
            };
          }
          if (prop === "getResponseHeader") {
            return function () {
              return null;
            };
          }
          if (prop === "abort") {
            return function () {
              state.readyState = 0;
              if (typeof target.onabort === "function") {
                target.onabort.call(proxy, new Event("abort"));
              }
            };
          }
        }

        var value = Reflect.get(target, prop, receiver);
        if (typeof value === "function") {
          return value.bind(target);
        }
        return value;
      },
      set: function (target, prop, value, receiver) {
        return Reflect.set(target, prop, value, receiver);
      },
    });

    return proxy;
  };

  window.XMLHttpRequest.prototype = NativeXHR.prototype;
})();
</script>
"""


def ensure_search_index_js() -> None:
    search_json = SEARCH_DIR / "search_index.json"
    if not search_json.exists():
        return
    data = json.loads(search_json.read_text(encoding="utf-8"))
    search_js = SEARCH_DIR / "search_index.js"
    search_js.write_text("var __index = " + json.dumps(data, ensure_ascii=False) + ";\n", encoding="utf-8")


def inject_shim_into_html() -> None:
    for html_path in SITE.rglob("*.html"):
        text = html_path.read_text(encoding="utf-8")
        if SHIM_MARKER in text:
            continue
        if "</head>" not in text:
            continue
        text = text.replace("</head>", FILE_PREVIEW_SHIM + "\n</head>", 1)
        html_path.write_text(text, encoding="utf-8")


def main() -> None:
    ensure_search_index_js()
    inject_shim_into_html()
    print("Post-processed docs preview assets")


if __name__ == "__main__":
    main()
