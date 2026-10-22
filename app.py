#!/usr/bin/python3
"""Pitch Presenter — нативное окно (GTK3 + WebKit) поверх present.py."""
import json
import subprocess
import sys
import threading
from pathlib import Path
from types import SimpleNamespace

import gi

gi.require_version("Gtk", "3.0")
gi.require_version("WebKit2", "4.1")
from gi.repository import GLib, Gtk, WebKit2  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
import present  # noqa: E402

APP_ID = "pitch-presenter"
CONFIG = Path(GLib.get_user_config_dir()) / APP_ID / "config.json"
DATA = Path(GLib.get_user_data_dir()) / APP_ID
PORT = 47615  # постоянный порт — чтобы localStorage (слайд, шрифт, таймер) не терялся между запусками

DECK_TYPES = ("*.pdf", "*.pptx", "*.ppt", "*.odp")
NOTE_TYPES = ("*.toml", "*.md", "*.markdown", "*.txt")


def load_config():
    try:
        return json.loads(CONFIG.read_text())
    except Exception:
        return {}


def save_config(cfg):
    try:
        CONFIG.parent.mkdir(parents=True, exist_ok=True)
        CONFIG.write_text(json.dumps(cfg, ensure_ascii=False, indent=1))
    except OSError:
        pass


def file_button(title, patterns, label):
    btn = Gtk.FileChooserButton(title=title, action=Gtk.FileChooserAction.OPEN)
    f = Gtk.FileFilter()
    f.set_name(label)
    for p in patterns:
        f.add_pattern(p)
        f.add_pattern(p.upper())
    btn.add_filter(f)
    btn.set_hexpand(True)
    return btn


class StartWindow(Gtk.ApplicationWindow):
    def __init__(self, app, deck=None, notes=None):
        super().__init__(application=app, title="Pitch Presenter")
        self.app = app
        self.set_default_size(560, -1)
        self.set_resizable(False)
        self.set_position(Gtk.WindowPosition.CENTER)

        hb = Gtk.HeaderBar(title="Pitch Presenter", subtitle="режим докладчика", show_close_button=True)
        self.set_titlebar(hb)

        grid = Gtk.Grid(column_spacing=12, row_spacing=12, margin=20)
        self.add(grid)

        self.deck_btn = file_button("Презентация", DECK_TYPES, "PDF / PPTX")
        self.deck_btn.connect("file-set", self.on_deck_set)
        self.notes_btn = file_button("Текст спикера", NOTE_TYPES, "TOML / Markdown")
        clear = Gtk.Button.new_from_icon_name("edit-clear-symbolic", Gtk.IconSize.BUTTON)
        clear.set_tooltip_text("Без текста")
        clear.connect("clicked", lambda *_: self.notes_btn.unselect_all())
        self.minutes = Gtk.SpinButton.new_with_range(0, 180, 1)
        self.minutes.set_tooltip_text("0 — без ограничения")

        def label(text):
            return Gtk.Label(label=text, xalign=0)

        grid.attach(label("Презентация"), 0, 0, 1, 1)
        grid.attach(self.deck_btn, 1, 0, 2, 1)
        grid.attach(label("Текст спикера"), 0, 1, 1, 1)
        grid.attach(self.notes_btn, 1, 1, 1, 1)
        grid.attach(clear, 2, 1, 1, 1)
        grid.attach(label("Длительность, мин"), 0, 2, 1, 1)
        grid.attach(self.minutes, 1, 2, 1, 1)

        self.status = Gtk.Label(xalign=0, wrap=True, max_width_chars=60)
        self.status.get_style_context().add_class("dim-label")
        self.spinner = Gtk.Spinner()
        self.start_btn = Gtk.Button(label="Начать")
        self.start_btn.get_style_context().add_class("suggested-action")
        self.start_btn.connect("clicked", self.on_start)
        row = Gtk.Box(spacing=10)
        row.pack_start(self.spinner, False, False, 0)
        row.pack_start(self.status, True, True, 0)
        row.pack_end(self.start_btn, False, False, 0)
        grid.attach(row, 0, 3, 3, 1)

        cfg = load_config()
        deck = deck or cfg.get("deck")
        notes = notes or (cfg.get("notes") if deck == cfg.get("deck") else None)
        if deck and Path(deck).exists():
            self.deck_btn.set_filename(str(deck))
            notes = notes or self.guess_notes(Path(deck))
        if notes and Path(notes).exists():
            self.notes_btn.set_filename(str(notes))
        self.minutes.set_value(float(cfg.get("minutes", 0)))
        self.start_btn.grab_focus()

    @staticmethod
    def guess_notes(deck: Path):
        for ext in (".toml", ".md", ".markdown", ".txt"):
            p = deck.with_suffix(ext)
            if p.exists():
                return p
        return None

    def on_deck_set(self, btn):
        guess = self.guess_notes(Path(btn.get_filename()))
        if guess and not self.notes_btn.get_filename():
            self.notes_btn.set_filename(str(guess))

    def busy(self, on, text=""):
        self.spinner.start() if on else self.spinner.stop()
        self.start_btn.set_sensitive(not on)
        self.status.set_text(text)

    def on_start(self, *_):
        deck = self.deck_btn.get_filename()
        if not deck:
            self.status.set_text("Выберите файл презентации")
            return
        notes = self.notes_btn.get_filename()
        minutes = self.minutes.get_value()
        save_config({"deck": deck, "notes": notes, "minutes": minutes})
        self.busy(True, "Готовлю слайды…")
        args = SimpleNamespace(deck=Path(deck), minutes=minutes)
        notes_path = Path(notes) if notes else None
        threading.Thread(target=self.prepare, args=(args, notes_path), daemon=True).start()

    def prepare(self, args, notes_path):
        try:
            slides = present.render_slides(args.deck, 1920)
            if not slides:
                raise RuntimeError("Не удалось получить слайды")
            _, notes, warnings = present.parse_notes(notes_path)
            srv, port = present.serve(slides, notes_path, args, PORT)
        except (RuntimeError, OSError, ValueError, subprocess.CalledProcessError) as e:
            GLib.idle_add(self.busy, False, f"Ошибка: {e}")
            return
        threading.Thread(target=srv.serve_forever, daemon=True).start()
        info = f"Слайдов: {len(slides)}, текстов: {len(notes)}"
        if warnings:
            info += " · " + "; ".join(warnings)
        GLib.idle_add(self.launch, srv, port, info)

    def launch(self, srv, port, info):
        self.busy(False, info)
        win = PresenterWindow(self.app, f"http://127.0.0.1:{port}/")

        def closed(*_):
            srv.bus.closed = True
            threading.Thread(target=srv.shutdown, daemon=True).start()
            srv.server_close()
            self.show()

        win.connect("destroy", closed)
        win.show_all()
        self.hide()


def web_context():
    DATA.mkdir(parents=True, exist_ok=True)
    mgr = WebKit2.WebsiteDataManager(base_data_directory=str(DATA / "data"),
                                     base_cache_directory=str(DATA / "cache"))
    return WebKit2.WebContext.new_with_website_data_manager(mgr)


def tune(view):
    s = view.get_settings()
    s.set_enable_fullscreen(True)
    s.set_javascript_can_open_windows_automatically(True)
    s.set_enable_back_forward_navigation_gestures(False)
    view.connect("create", on_create)
    view.connect("context-menu", lambda *_: True)  # без меню «Назад/Перезагрузить»
    return view


def on_create(view, action):
    """window.open('/audience') → отдельное нативное окно показа."""
    child = tune(WebKit2.WebView.new_with_related_view(view))
    win = Gtk.Window(title="Показ")
    win.set_default_size(1280, 720)
    win.add(child)
    child.connect("ready-to-show", lambda *_: win.show_all())
    child.connect("close", lambda *_: win.destroy())
    child.connect("notify::title", lambda v, _: win.set_title(v.get_title() or "Показ"))
    view.get_toplevel().connect("destroy", lambda *_: win.destroy())
    return child


class PresenterWindow(Gtk.ApplicationWindow):
    _ctx = None

    def __init__(self, app, url):
        super().__init__(application=app, title="Докладчик")
        if PresenterWindow._ctx is None:
            PresenterWindow._ctx = web_context()
        self.set_default_size(1400, 860)
        self.maximize()
        view = tune(WebKit2.WebView.new_with_context(PresenterWindow._ctx))
        view.connect("notify::title", lambda v, _: self.set_title(v.get_title() or "Докладчик"))
        self.add(view)
        view.load_uri(url)


class App(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="dev.semao0.PitchPresenter")
        self.files = []
        GLib.set_prgname(APP_ID)
        GLib.set_application_name("Pitch Presenter")

    def do_activate(self):
        deck = notes = None
        for f in self.files:
            if Path(f).suffix.lower() in (".toml", ".md", ".markdown", ".txt"):
                notes = f
            else:
                deck = f
        win = next((w for w in self.get_windows() if isinstance(w, StartWindow)), None)
        if win is None:
            win = StartWindow(self, deck, notes)
            win.show_all()
        win.present()


def main():
    app = App()
    app.files = [str(Path(a).resolve()) for a in sys.argv[1:] if not a.startswith("-")]
    sys.exit(app.run([sys.argv[0]]))


if __name__ == "__main__":
    main()
