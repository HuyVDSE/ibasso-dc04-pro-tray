"""Download public static iBasso controller assets; never fetch account/API data."""
import concurrent.futures
import hashlib
import json
import re
import subprocess
from pathlib import Path
from urllib.parse import urljoin, urlsplit, unquote

ORIGIN = 'https://devices.ibasso.cn'
ROOT = Path(__file__).resolve().parent / 'local-web'
ROOT.mkdir(exist_ok=True)
EXT = r'(?:js|css|png|jpg|jpeg|webp|svg|ico|woff2?|ttf|gif|avif)'
QUOTED = re.compile(r'''["'`]([^"'`\s<>]+?\.''' + EXT + r''')(?:\?[^"'`\s<>]*)?["'`]''', re.I)
CSS_URL = re.compile(r'url\(\s*[\'"]?([^\)\'"\s]+)', re.I)
seen = set()
failures = []
external = set()
records = []

def fetch(path):
    dest = ROOT / unquote(path.lstrip('/'))
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.stat().st_size:
        return path, dest.read_bytes(), None
    result = subprocess.run(['curl.exe', '-f', '-L', '--retry', '2', '--connect-timeout', '10', '--max-time', '40', '-sS', ORIGIN + path, '-o', str(dest)], capture_output=True, text=True)
    if result.returncode:
        return path, None, result.stderr.strip()
    return path, dest.read_bytes(), None

pending = {'/index.html'}
while pending:
    batch = sorted(pending - seen)
    if not batch:
        break
    seen.update(batch)
    pending = set()
    with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
        for path, data, error in pool.map(fetch, batch):
            if error:
                failures.append({'path': path, 'error': error})
                continue
            records.append({'path': path, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()})
            if not path.endswith(('.js', '.css', '.html')):
                continue
            source = data.decode('utf-8')
            candidates = QUOTED.findall(source) + CSS_URL.findall(source)
            for candidate in candidates:
                if any(s in candidate for s in ('${', 'data:', 'blob:', '*', '\\')):
                    continue
                # Vite's dependency map stores assets/... relative to the site root.
                base = ORIGIN + '/' if candidate.startswith(('assets/', 'images/')) else ORIGIN + path
                url = urlsplit(urljoin(base, candidate))
                if url.netloc != urlsplit(ORIGIN).netloc:
                    external.add(url.geturl())
                    continue
                if re.search(r'\.' + EXT + r'$', url.path, re.I):
                    pending.add(url.path)
    print(f'Downloaded {len(records)}; failed {len(failures)}; discovered {len(pending - seen)} more', flush=True)

(ROOT.parent / 'mirror-manifest.json').write_text(json.dumps({'origin': ORIGIN, 'files': records, 'failures': failures, 'external_asset_urls': sorted(external)}, ensure_ascii=False, indent=2), encoding='utf-8')
print('Mirror finished. See mirror-manifest.json.', flush=True)
