#!/usr/bin/env python3
"""Режим докладчика для PDF/PPTX + текст спикера в TOML (или Markdown).

    present.py deck.pdf notes.toml [--minutes 8] [--port 8765]
    present.py --to-toml notes.md > notes.toml

Открывает в браузере окно докладчика (слайд, следующий слайд, текст, таймер)
и по кнопке — окно для показа, которое шарится в созвон. Окна синхронизируются
между собой, текст перечитывается на лету.
"""
import argparse
import hashlib
import html
import json
import re
import shutil
import subprocess
import sys
import threading
import time
import webbrowser
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import parse_qs, urlparse

CACHE = Path.home() / ".cache" / "pitch-presenter"


# ---------- слайды ----------

def render_slides(deck: Path, width: int) -> list[Path]:
    st = deck.stat()
    key = hashlib.sha1(f"{deck.resolve()}|{st.st_mtime_ns}|{st.st_size}|{width}".encode()).hexdigest()[:16]
    out = CACHE / key
    pngs = sorted(out.glob("s-*.png"))
    if pngs:
        return pngs
    out.mkdir(parents=True, exist_ok=True)
    pdf = deck
    if deck.suffix.lower() in (".pptx", ".ppt", ".odp", ".key"):
        soffice = shutil.which("soffice") or shutil.which("libreoffice") or next(
            (p for p in ("/Applications/LibreOffice.app/Contents/MacOS/soffice",) if Path(p).exists()), None)
        if not soffice:
            raise RuntimeError("Для PPTX нужен LibreOffice (soffice)")
        print("Конвертирую PPTX → PDF через LibreOffice…")
        subprocess.run([soffice, "--headless", "--convert-to", "pdf", "--outdir", str(out), str(deck)],
                       check=True, stdout=subprocess.DEVNULL)
        pdf = out / (deck.stem + ".pdf")
        if not pdf.exists():
            raise RuntimeError("LibreOffice не смог сконвертировать презентацию")
    print("Рендерю слайды…")
    try:
        import pypdfium2 as pdfium  # в сборке для Mac poppler нет — рендерим через PDFium
    except ImportError:
        pdfium = None
    if pdfium:
        doc = pdfium.PdfDocument(str(pdf))
        try:
            for i, page in enumerate(doc, 1):
                page.render(scale=width / page.get_width()).to_pil().save(out / f"s-{i:03d}.png")
        finally:
            doc.close()
    else:
        if not shutil.which("pdftoppm"):
            raise RuntimeError("Нужен pdftoppm (пакет poppler-utils)")
        subprocess.run(["pdftoppm", "-png", "-scale-to-x", str(width), "-scale-to-y", "-1",
                        str(pdf), str(out / "s")], check=True)
    return sorted(out.glob("s-*.png"))


# ---------- заметки ----------

def inline(s: str) -> str:
    s = html.escape(s, quote=False)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", s)
    s = re.sub(r"==(.+?)==", r"<mark>\1</mark>", s)
    s = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", s)
    s = re.sub(r"(?<!\w)_(?!\s)(.+?)(?<!\s)_(?!\w)", r"<em>\1</em>", s)
    return s


BULLET = re.compile(r"^\s*[-*+]\s")


def body_html(text: str) -> str:
    parts = []
    for block in re.split(r"\n\s*\n", text.strip()):
        lines = [l.rstrip() for l in block.splitlines() if l.strip()]
        if not lines:
            continue
        if all(re.match(r"\s*[-*+]\s", l) for l in lines):
            items = "".join(f"<li>{inline(re.sub(BULLET, '', l))}</li>" for l in lines)
            parts.append(f"<ul>{items}</ul>")
        elif all(l.startswith(">") for l in lines):
            parts.append(f'<div class="cue">{inline(" ".join(l.lstrip("> ") for l in lines))}</div>')
        else:
            parts.append(f"<p>{inline(' '.join(lines))}</p>")
    return "".join(parts)


def parse_notes(path: Path | None):
    """Читает .toml или .md → (meta, {номер слайда: заметка}, предупреждения)."""
    if not path:
        return {}, {}, []
    meta, sections = read_toml(path) if path.suffix.lower() == ".toml" else read_md(path)
    notes, warnings = {}, []
    for sec in sections:
        n = sec["n"]
        if n in notes:
            warnings.append(f"слайд {n}: текст указан дважды")
        notes[n] = {"title": sec["title"], "html": body_html(sec["text"]), "words": spoken_words(sec["text"]),
                    "seconds": sec["time"], "reserve": sec["reserve"]}
    return meta, notes, warnings


RESERVE = re.compile(r"\s*[·|-]?\s*резерв\b", re.I)


def read_md(path: Path):
    """## 03 · Заголовок @0:45 — текст к слайду 3; заголовок без номера — к следующему слайду."""
    meta, sections = {}, []
    text = path.read_text(encoding="utf-8")
    m = re.match(r"---\n(.*?)\n---\n", text, re.S)
    if m:
        for line in m.group(1).splitlines():
            if ":" in line:
                k, v = line.split(":", 1)
                meta[k.strip()] = v.strip().strip("\"'")
        text = text[m.end():]
    cur = 0
    for sec in re.split(r"^## ", text, flags=re.M)[1:]:
        head, _, body = sec.partition("\n")
        head, seconds = split_time(head.strip())
        num = re.match(r"(\d+)\s*[·.:)\-–—|]?\s*(.*)", head)
        if num:
            cur, title = int(num.group(1)), num.group(2) or head
        else:
            cur, title = cur + 1, head
        sections.append({"n": cur, "title": RESERVE.sub("", title).strip(), "text": body,
                         "time": seconds, "reserve": bool(RESERVE.search(title))})
    return meta, sections


TOP_KEYS = {"title", "minutes", "slide"}
SLIDE_KEYS = {"n", "title", "time", "reserve", "text"}


def read_toml(path: Path):
    """Строгий формат: неизвестное поле или неверный тип — ошибка с указанием слайда."""
    import tomllib
    with path.open("rb") as f:
        data = tomllib.load(f)
    if extra := set(data) - TOP_KEYS:
        raise ValueError(f"неизвестные поля: {', '.join(sorted(extra))}")
    meta = {}
    if "title" in data:
        meta["title"] = expect(data["title"], str, "title")
    if "minutes" in data:
        meta["minutes"] = expect(data["minutes"], (int, float), "minutes")
    sections = []
    for i, sl in enumerate(expect(data.get("slide", []), list, "slide"), 1):
        where = f"[[slide]] №{i}"
        if extra := set(sl) - SLIDE_KEYS:
            raise ValueError(f"{where}: неизвестные поля: {', '.join(sorted(extra))}")
        if "n" not in sl or "text" not in sl:
            raise ValueError(f"{where}: обязательны поля n и text")
        n = expect(sl["n"], int, f"{where}.n")
        if n < 1:
            raise ValueError(f"{where}.n: слайды нумеруются с 1")
        sections.append({
            "n": n,
            "title": expect(sl.get("title", f"Слайд {n}"), str, f"{where}.title"),
            "text": expect(sl["text"], str, f"{where}.text"),
            "time": parse_time(sl["time"], f"{where}.time") if "time" in sl else None,
            "reserve": expect(sl.get("reserve", False), bool, f"{where}.reserve"),
        })
    return meta, sections


def expect(value, types, where):
    # bool — подкласс int в Python, поэтому true не должно проходить как число
    if not isinstance(value, types) or (isinstance(value, bool) and types is not bool):
        raise ValueError(f"{where}: неверное значение {value!r}")
    return value


def parse_time(value, where):
    """45 → 45 с; "0:45" → 45 с; "45s" → 45 с."""
    if isinstance(value, int) and not isinstance(value, bool):
        return value
    if isinstance(value, str):
        if m := re.fullmatch(r"\s*(\d+):([0-5]\d)\s*", value):
            return int(m.group(1)) * 60 + int(m.group(2))
        if m := re.fullmatch(r"\s*(\d+)\s*[sс]\s*", value):
            return int(m.group(1))
    raise ValueError(f'{where}: время пишется как 45, "45s" или "0:45", а не {value!r}')


def to_toml(meta, sections) -> str:
    """Разобранный md → TOML."""
    def q(s):
        return json.dumps(s, ensure_ascii=False)

    out = []
    if "title" in meta:
        out.append(f"title = {q(meta['title'])}")
    if "minutes" in meta:
        out.append(f"minutes = {float(meta['minutes']):g}")
    for sec in sections:
        out += ["", "[[slide]]", f"n = {sec['n']}", f"title = {q(sec['title'])}"]
        if sec["time"] is not None:
            out.append(f'time = "{sec["time"] // 60}:{sec["time"] % 60:02d}"')
        if sec["reserve"]:
            out.append("reserve = true")
        body = sec["text"].strip()
        quote = "'" * 3
        out.append(f"text = {quote}\n{body}\n{quote}" if quote not in body else f"text = {q(body)}")
    return "\n".join(out) + "\n"


def spoken_words(body: str) -> int:
    """Слова, которые произносятся: без ремарок «> …» и без markdown-разметки."""
    spoken = "\n".join(l for l in body.splitlines() if not l.lstrip().startswith(">"))
    return len(re.findall(r"\w[\w\-–]*", re.sub(r"[*_=`]", "", spoken)))


def split_time(head: str):
    """«Заголовок @0:45» / «@45s» / «@45с» → («Заголовок», 45)."""
    m = re.search(r"\s*@\s*(?:(\d+):(\d{2})|(\d+)\s*[sс]?)\s*$", head)
    if not m:
        return head, None
    seconds = int(m.group(1)) * 60 + int(m.group(2)) if m.group(1) else int(m.group(3))
    return head[:m.start()], seconds


# ---------- сервер ----------

class Bus:
    """Синхронизация окон через сервер: докладчик кладёт состояние, показ — нажатые клавиши.
    Окна опрашивают сервер раз в ~200 мс — работает в любом WebView (SSE в WebKit на macOS
    буферизуется и не доходит)."""

    def __init__(self):
        self.lock = threading.Lock()
        self.state = None
        self.keys = []      # [(seq, key)]
        self.seq = 0
        self.seen = 0.0     # когда окно показа последний раз спрашивало состояние
        self.closed = False

    def push_key(self, key):
        with self.lock:
            self.seq += 1
            self.keys = [*self.keys[-50:], (self.seq, key)]

    def keys_since(self, since):
        with self.lock:
            if since > self.seq:  # сервер перезапущен — начинаем с нуля
                since = 0
            return [k for n, k in self.keys if n > since], self.seq


def make_handler(slides, notes_path, args):
    page_dir = Path(getattr(sys, "_MEIPASS", Path(__file__).resolve().parent)) / "web"
    bus = Bus()

    class H(BaseHTTPRequestHandler):
        bus_ = bus

        def log_message(self, *a):
            pass

        def send(self, code, body: bytes, ctype):
            self.send_response(code)
            self.send_header("Content-Type", ctype)
            self.send_header("Content-Length", str(len(body)))
            self.send_header("Cache-Control", "no-store" if "json" in ctype or "html" in ctype else "max-age=3600")
            self.end_headers()
            self.wfile.write(body)

        def do_GET(self):
            p = urlparse(self.path).path
            if p in ("/", "/presenter"):
                return self.send(200, (page_dir / "presenter.html").read_bytes(), "text/html; charset=utf-8")
            if p == "/audience":
                return self.send(200, (page_dir / "audience.html").read_bytes(), "text/html; charset=utf-8")
            if p == "/api/version":
                v = notes_path.stat().st_mtime_ns if notes_path and notes_path.exists() else 0
                return self.send(200, json.dumps({"v": v}).encode(), "application/json")
            if p == "/api/deck":
                try:
                    meta, notes, warnings = parse_notes(notes_path)
                except Exception as e:  # битый md не должен ронять показ
                    meta, notes, warnings = {}, {}, [f"ошибка чтения заметок: {e}"]
                extra = [n for n in notes if n > len(slides) or n < 1]
                if extra:
                    warnings.append(f"заметки к несуществующим слайдам: {extra}")
                data = {
                    "title": meta.get("title", args.deck.stem),
                    "minutes": float(meta.get("minutes", args.minutes or 0) or 0),
                    "count": len(slides),
                    "notes": {str(k): v for k, v in notes.items()},
                    "warnings": warnings,
                }
                return self.send(200, json.dumps(data, ensure_ascii=False).encode(), "application/json")
            if p == "/api/state":  # окно показа
                bus.seen = time.monotonic()
                return self.send(200, json.dumps(bus.state or {}).encode(), "application/json")
            if p == "/api/poll":  # окно докладчика: новые клавиши и есть ли показ
                since = int(parse_qs(urlparse(self.path).query).get("since", ["0"])[0] or 0)
                keys, seq = bus.keys_since(since)
                data = {"keys": keys, "seq": seq, "audience": time.monotonic() - bus.seen < 1.5}
                return self.send(200, json.dumps(data, ensure_ascii=False).encode(), "application/json")
            m = re.fullmatch(r"/slide/(\d+)\.png", p)
            if m and 1 <= int(m.group(1)) <= len(slides):
                return self.send(200, slides[int(m.group(1)) - 1].read_bytes(), "image/png")
            self.send(404, b"not found", "text/plain")

        def do_POST(self):
            p = urlparse(self.path).path
            try:
                msg = json.loads(self.rfile.read(int(self.headers.get("Content-Length", 0))) or b"{}")
            except ValueError:
                return self.send(400, b"bad json", "text/plain")
            if p == "/api/state":
                bus.state = msg
            elif p == "/api/key":
                bus.push_key(str(msg.get("key", "")))
            else:
                return self.send(404, b"not found", "text/plain")
            self.send(200, b"{}", "application/json")

    return H


def serve(slides, notes_path, args, port):
    """Поднимает сервер на первом свободном порту от port, возвращает (server, port)."""
    handler = make_handler(slides, notes_path, args)
    for p in range(port, port + 20):
        try:
            srv = ThreadingHTTPServer(("127.0.0.1", p), handler)
            srv.bus = handler.bus_
            return srv, p
        except OSError:
            print(f"  порт {p} занят, пробую следующий")
    raise RuntimeError(f"Нет свободного порта в {port}–{port + 19}")


def main():
    if len(sys.argv) == 3 and sys.argv[1] == "--to-toml":
        meta, sections = read_md(Path(sys.argv[2]))
        sys.stdout.write(to_toml(meta, sections))
        return
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("deck", type=Path, help="PDF или PPTX")
    ap.add_argument("notes", type=Path, nargs="?", help="текст спикера: .toml или .md")
    ap.add_argument("-m", "--minutes", type=float, help="длительность выступления (таймер)")
    ap.add_argument("-p", "--port", type=int, default=8765)
    ap.add_argument("-w", "--width", type=int, default=1920, help="ширина рендера слайдов, px")
    ap.add_argument("--no-browser", action="store_true")
    args = ap.parse_args()

    if not args.deck.exists():
        sys.exit(f"Нет файла {args.deck}")
    if args.notes and not args.notes.exists():
        sys.exit(f"Нет файла {args.notes}")
    try:
        slides = render_slides(args.deck, args.width)
    except (RuntimeError, subprocess.CalledProcessError) as e:
        sys.exit(str(e))
    if not slides:
        sys.exit("Не удалось получить слайды")
    _, notes, warnings = parse_notes(args.notes)
    words = sum(n["words"] for n in notes.values())
    print(f"Слайдов: {len(slides)}, заметок: {len(notes)}, слов: {words} (~{words / 130:.1f} мин)")
    missing = [i for i in range(1, len(slides) + 1) if i not in notes]
    if args.notes and missing:
        print(f"  без заметок: {missing}")
    for w in warnings:
        print("  !", w)

    try:
        srv, port = serve(slides, args.notes, args, args.port)
    except RuntimeError as e:
        sys.exit(str(e))
    url = f"http://127.0.0.1:{port}/"
    print(f"Докладчик: {url}\nПоказ:     {url}audience\nCtrl+C — выход")
    if not args.no_browser:
        webbrowser.open(url)
    try:
        srv.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
