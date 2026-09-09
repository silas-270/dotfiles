#!/usr/bin/env python3
"""Ask AI: WebKitGTK Overlay for Screenshot Q&A with Markdown & KaTeX Math."""

import os

# Disable hardware DMABUF compositor to allow true Wayland RGBA transparency
os.environ["WEBKIT_DISABLE_COMPOSITING_MODE"] = "1"
os.environ["WEBKIT_DISABLE_DMABUF_RENDERER"] = "1"

import json
import shutil
import subprocess
import sys
import threading
import uuid
import warnings
import cairo

warnings.filterwarnings("ignore", category=DeprecationWarning)

import gi

gi.require_version("Gdk", "3.0")
gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
gi.require_version("JavaScriptCore", "4.1")
from gi.repository import Gdk, GLib, Gtk, WebKit2


def find_claude_binary():
    candidate = shutil.which("claude")
    if candidate:
        return candidate
    local_path = os.path.expanduser("~/.local/bin/claude")
    if os.path.isfile(local_path) and os.access(local_path, os.X_OK):
        return local_path
    cargo_path = os.path.expanduser("~/.cargo/bin/claude")
    if os.path.isfile(cargo_path) and os.access(cargo_path, os.X_OK):
        return cargo_path
    return "claude"


def load_theme_colors():
    paths = [
        os.path.expanduser("~/.config/theme/generated/colors.json"),
        os.path.expanduser("~/.config/dotfiles/theme/generated/colors.json"),
        os.path.expanduser("~/dotfiles/theme/generated/colors.json"),
    ]
    colors_path = next((p for p in paths if os.path.isfile(p)), paths[0])
    if os.path.isfile(colors_path):
        try:
            with open(colors_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                return data.get("colors", {})
        except Exception as e:
            print(f"[ask-ai] could not load colors.json: {e}", file=sys.stderr)
    return {
        "bg_base": "#54382B",
        "bg_input": "#664839",
        "fg_primary": "#F2E3D5",
        "fg_muted": "#C2AA95",
        "accent": "#FF8C00",
        "border": "#7A523D",
    }


class AskAiOverlay:

    def __init__(self, screenshot_path=None, demo_mode=False):
        self.screenshot_path = screenshot_path
        self.demo_mode = demo_mode
        self.session_id = str(uuid.uuid4())
        self.first_turn = True
        self.theme_colors = load_theme_colors()
        self.claude_bin = find_claude_binary()

        GLib.set_prgname("ask-ai-overlay")
        GLib.set_application_name("ask-ai-overlay")

        # Window setup
        self.window = Gtk.Window(type=Gtk.WindowType.TOPLEVEL)
        self.window.set_wmclass("ask-ai-overlay", "ask-ai-overlay")
        self.window.set_title("Ask AI")
        self.window.set_default_size(900, 650)
        self.window.set_position(Gtk.WindowPosition.CENTER)
        self.window.set_decorated(False)

        # Transparency & visual setup
        screen = self.window.get_screen()
        visual = screen.get_rgba_visual()
        if visual and screen.is_composited():
            self.window.set_visual(visual)
        self.window.set_app_paintable(True)

        css_provider = Gtk.CssProvider()
        css_provider.load_from_data(b"""
            window, decoration, .background {
                background-color: transparent;
                background-image: none;
                box-shadow: none;
                border: none;
            }
        """)
        Gtk.StyleContext.add_provider_for_screen(
            screen, css_provider, Gtk.STYLE_PROVIDER_PRIORITY_APPLICATION
        )

        self.window.connect("destroy", Gtk.main_quit)
        self.window.connect("key-press-event", self.on_key_press)
        self.window.connect("draw", self.on_draw)

        # WebKit content manager and bridge
        self.content_manager = WebKit2.UserContentManager()
        self.content_manager.register_script_message_handler("pybridge")
        self.content_manager.connect(
            "script-message-received::pybridge", self.on_js_message
        )

        self.webview = WebKit2.WebView.new_with_user_content_manager(
            self.content_manager
        )
        bg = Gdk.RGBA()
        bg.parse("rgba(0,0,0,0)")
        self.webview.set_background_color(bg)

        # Settings
        settings = self.webview.get_settings()
        settings.set_enable_javascript(True)
        settings.set_enable_developer_extras(True)
        settings.set_hardware_acceleration_policy(
            WebKit2.HardwareAccelerationPolicy.NEVER
        )

        self.window.add(self.webview)

        # Load UI
        ui_dir = os.path.dirname(os.path.abspath(__file__))
        html_path = os.path.join(ui_dir, "ask-ai-ui", "index.html")
        self.webview.load_uri(f"file://{html_path}")

    def show(self):
        self.window.show_all()

    def on_draw(self, _widget, cr):
        cr.set_operator(cairo.OPERATOR_CLEAR)
        cr.paint()
        return False

    def on_key_press(self, _widget, event):
        if event.keyval == Gdk.KEY_Escape:
            Gtk.main_quit()
            return True
        return False

    def eval_js(self, script):
        GLib.idle_add(lambda: self.webview.run_javascript(script, None, None, None))

    def on_js_message(self, _manager, js_result):
        try:
            val_str = js_result.get_js_value().to_string()
            data = json.loads(val_str)
            action = data.get("action")

            if action == "ready":
                colors_json = json.dumps(self.theme_colors)
                self.eval_js(f"setTheme({colors_json});")
                if self.demo_mode:
                    self.load_demo()

            elif action == "ask":
                prompt = data.get("prompt", "")
                if prompt:
                    threading.Thread(
                        target=self.run_claude_query, args=(prompt,), daemon=True
                    ).start()

            elif action == "log":
                print(f"[ask-ai js] {data.get('message')}", file=sys.stderr)

            elif action == "error":
                print(f"[ask-ai js error] {data.get('message')}", file=sys.stderr)

        except Exception as e:
            print(f"[ask-ai] JS message error: {e}", file=sys.stderr)

    def load_demo(self):
        demo_q = "Ich checke den IFT nicht"
        demo_a = (
            "**Satz 4.32 (IFT):** Sei $X \\subset \\mathbb{R}^n$, $Y \\subset \\mathbb{R}^m$ offen, $n > m$, $f \\in C^k(X; Y)$.\n\n"
            "Sei $\\bar{x} \\in X$, $\\bar{y} := f(\\bar{x}) \\in Y$. Ist $f'(\\bar{x})$ **surjektiv**, dann ist\n\n"
            "$$\\mathfrak{M} := f^{-1}(\\{\\bar{y}\\})$$\n\n"
            "bei $\\bar{x}$ eine lokale $(n-m)$-dimensionale Untermannigfaltigkeit der Klasse $C^k$, mit Tangentialraum\n\n"
            "$$T_{\\bar{x}}\\mathfrak{M} = \\ker f'(\\bar{x}).$$\n\n"
            "**Intuition:** Die Niveaumenge (Lösungsmenge von $f(x) = \\bar{y}$) einer Abbildung mit surjektiver Ableitung ist lokal eine glatte Fläche der Dimension \"Anzahl Variablen minus Anzahl Gleichungen\", und ihr Tangentialraum ist genau der Kern der Ableitung."
        )
        self.eval_js(f"addUserMessage({json.dumps(demo_q)});")
        self.eval_js(f"appendAiChunk({json.dumps(demo_a)});")
        self.eval_js("finishAiResponse();")

    def run_claude_query(self, user_prompt):
        try:
            if not shutil.which(self.claude_bin) and not os.path.isfile(self.claude_bin):
                self.eval_js(
                    f"showAiError({json.dumps(f'Claude CLI executable not found at: {self.claude_bin}')});"
                )
                return

            math_sys_prompt = (
                "You are a concise AI study aid. "
                "MANDATORY FORMATTING: Format ALL mathematical notation, formulas, symbols, variables, sets, and equations in LaTeX. "
                "Use $...$ for inline math (e.g. $X \\subset \\mathbb{R}^n$, $\\bar{x} \\in X$, $\\bar{y} := f(\\bar{x}) \\in Y$, $\\mathfrak{M}$, $f \\in C^k(X; Y)$, $f'(\\bar{x})$) "
                "and $$...$$ for display equations. "
                "NEVER use raw unicode characters for math (do NOT use ℝ, ⁿ, ᵐ, ᵏ, ⊂, ∈, ⁻¹, x̄, ȳ, 𝔐). Always use proper LaTeX macros (\\mathbb{R}^n, \\subset, \\in, \\mathfrak{M}, \\bar{x}, \\bar{y}, ^{-1})."
            )

            if self.first_turn:
                self.first_turn = False
                if self.screenshot_path and os.path.isfile(self.screenshot_path):
                    full_prompt = (
                        f"Read the image at '{self.screenshot_path}' using the Read tool, "
                        f"then answer this question about it concisely and directly, like a study aid — "
                        f"no preamble, no restating the question, format ALL math in LaTeX $...$: {user_prompt}"
                    )
                    cmd = [
                        self.claude_bin,
                        "-p",
                        full_prompt,
                        "--allowedTools",
                        "Read",
                        "--append-system-prompt",
                        math_sys_prompt,
                        "--permission-mode",
                        "bypassPermissions",
                        "--session-id",
                        self.session_id,
                        "--verbose",
                        "--output-format=stream-json",
                        "--include-partial-messages",
                    ]
                else:
                    cmd = [
                        self.claude_bin,
                        "-p",
                        user_prompt,
                        "--append-system-prompt",
                        math_sys_prompt,
                        "--permission-mode",
                        "bypassPermissions",
                        "--session-id",
                        self.session_id,
                        "--verbose",
                        "--output-format=stream-json",
                        "--include-partial-messages",
                    ]
            else:
                cmd = [
                    self.claude_bin,
                    "-p",
                    user_prompt,
                    "--append-system-prompt",
                    math_sys_prompt,
                    "--permission-mode",
                    "bypassPermissions",
                    "--resume",
                    self.session_id,
                    "--verbose",
                    "--output-format=stream-json",
                    "--include-partial-messages",
                ]

            proc = subprocess.Popen(
                cmd,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                bufsize=1,
            )

            stderr_lines = []
            def drain_stderr():
                for line in proc.stderr:
                    stderr_lines.append(line)

            threading.Thread(target=drain_stderr, daemon=True).start()

            has_received_chunk = False
            collected_errors = []

            for line in proc.stdout:
                line = line.strip()
                if not line:
                    continue
                try:
                    obj = json.loads(line)
                    evt_type = obj.get("type")

                    if evt_type == "stream_event":
                        event = obj.get("event", {})
                        if event.get("type") == "content_block_delta":
                            delta = event.get("delta", {})
                            text = delta.get("text")
                            if text:
                                has_received_chunk = True
                                self.eval_js(
                                    f"appendAiChunk({json.dumps(text)});"
                                )

                    elif evt_type == "result":
                        if obj.get("is_error"):
                            errs = obj.get("errors", [])
                            if errs:
                                collected_errors.extend(errs)
                        res_text = obj.get("result")
                        if not has_received_chunk and res_text:
                            has_received_chunk = True
                            self.eval_js(
                                f"appendAiChunk({json.dumps(res_text)});"
                            )

                except json.JSONDecodeError:
                    continue

            proc.wait()
            stderr_text = "".join(stderr_lines).strip()

            if proc.returncode != 0 or collected_errors:
                err_msg = "\n".join(collected_errors) if collected_errors else (stderr_text or f"Claude exited with status {proc.returncode}")
                print(f"[ask-ai error] {err_msg}", file=sys.stderr)
                self.eval_js(f"showAiError({json.dumps(err_msg)});")
            elif not has_received_chunk:
                err_msg = stderr_text or "No response received from Claude."
                print(f"[ask-ai error] {err_msg}", file=sys.stderr)
                self.eval_js(f"showAiError({json.dumps(err_msg)});")
            else:
                self.eval_js("finishAiResponse();")

        except Exception as e:
            print(f"[ask-ai exception] {e}", file=sys.stderr)
            self.eval_js(f"showAiError({json.dumps(str(e))});")


def main():
    args = sys.argv[1:]
    demo_mode = "--demo" in args

    screenshot_path = None
    if not demo_mode and args:
        screenshot_path = args[0]

    app = AskAiOverlay(screenshot_path=screenshot_path, demo_mode=demo_mode)
    app.show()
    Gtk.main()


if __name__ == "__main__":
    main()
