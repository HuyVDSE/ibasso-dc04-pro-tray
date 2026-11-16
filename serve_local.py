"""Serve only the downloaded controller on loopback. No cloud proxy."""
import json
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parent / 'local-web'
HOST, PORT = '127.0.0.1', 8044

class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=str(ROOT), **kwargs)

    def end_headers(self):
        # The vendor's translation runtime compiles messages with new Function.
        # Restrict outbound connections without changing that runtime behavior.
        self.send_header('Content-Security-Policy', "connect-src 'self'; img-src 'self' data: blob:; font-src 'self' data:; object-src 'none'; base-uri 'self'")
        self.send_header('Permissions-Policy', 'hid=(self)')
        self.send_header('X-Content-Type-Options', 'nosniff')
        self.send_header('Cache-Control', 'no-cache')
        super().end_headers()

    def do_GET(self):
        path = urlsplit(self.path).path
        if path == '/local-health':
            data = json.dumps({'app': 'ibasso-local-controller', 'cloud_proxy': False}).encode()
            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)
            return
        if path.startswith('/api/'):
            self.send_error(503, 'Cloud services are unavailable in this local copy')
            return
        super().do_GET()

if __name__ == '__main__':
    print(f'iBasso local controller: http://{HOST}:{PORT}/', flush=True)
    ThreadingHTTPServer((HOST, PORT), Handler).serve_forever()
