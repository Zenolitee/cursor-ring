# Writes assets/scheme/: the site's dot + ring look as a full Windows cursor scheme (17 roles) plus install.inf.
# Right-click install.inf -> Install, then pick "Cursor Ring" in Mouse settings -> Additional mouse settings -> Pointers.
# Shapes are signed-distance functions in a 32-unit design grid, rendered at several sizes so the size slider stays crisp.
# Stdlib only.
import struct, zlib, math, pathlib

SIZES = (32, 48, 64, 96, 128)
CORE = (255, 241, 214)        # #fff1d6
GOLD = (240, 164, 90)         # link accent, same as ring.rs
GLOW = (255, 217, 160)
EDGE = (24, 18, 12)           # dark outline so the light shapes still show on white
OUT = 1.1                     # outline width, design units
TAU = 2 * math.pi

# --- shapes: each returns p -> distance (negative inside), in design units ---
def circle(cx, cy, r):
    return lambda x, y: math.hypot(x - cx, y - cy) - r

def ring(cx, cy, r, w):
    return lambda x, y: abs(math.hypot(x - cx, y - cy) - r) - w / 2

def seg(ax, ay, bx, by, w):
    def d(x, y):
        px, py, vx, vy = x - ax, y - ay, bx - ax, by - ay
        t = max(0.0, min(1.0, (px * vx + py * vy) / (vx * vx + vy * vy)))
        return math.hypot(px - vx * t, py - vy * t) - w / 2
    return d

def arc(cx, cy, r, w, start, sweep):          # degrees, y-down so positive sweep runs clockwise on screen
    a0, sw = math.radians(start), math.radians(sweep)
    ends = [(cx + r * math.cos(a), cy + r * math.sin(a)) for a in (a0, a0 + sw)]
    def d(x, y):
        if (math.atan2(y - cy, x - cx) - a0) % TAU <= sw:
            return abs(math.hypot(x - cx, y - cy) - r) - w / 2
        return min(math.hypot(x - ex, y - ey) for ex, ey in ends) - w / 2
    return d

def tri(*pts):                                # convex polygon, any winding
    def d(x, y):
        edge, sides = 1e9, set()
        for (ax, ay), (bx, by) in zip(pts, pts[1:] + pts[:1]):
            vx, vy, px, py = bx - ax, by - ay, x - ax, y - ay
            t = max(0.0, min(1.0, (px * vx + py * vy) / (vx * vx + vy * vy)))
            edge = min(edge, math.hypot(px - vx * t, py - vy * t))
            sides.add(vx * py - vy * px > 0)
        return -edge if len(sides) == 1 else edge    # same side of every edge = inside
    return d

def union(*fs):
    return lambda x, y: min(f(x, y) for f in fs)

def arrow2(deg, half=10.0):                   # double-headed arrow through the centre
    c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg))
    at = lambda u, v: (16 + u * c - v * s, 16 + u * s + v * c)
    head = lambda k: tri(at(k * (half + 3), 0), at(k * (half - 2.5), 4.5), at(k * (half - 2.5), -4.5))
    return union(seg(*at(-half, 0), *at(half, 0), 2), head(1), head(-1))

def spinner(frames, make):
    return [make(360 * i / frames) for i in range(frames)]

DOT = circle(16, 16, 3)

# role -> (file stem, shape or list of frames, hotspot in design units, fill colour)
ROLES = [
    ('pointer',     DOT, (16, 16), CORE),
    ('help',        union(circle(9, 9, 3), arc(22, 15, 3.5, 2, 180, 270), seg(22, 18.5, 22, 20.5, 2), circle(22, 25, 1.3)), (9, 9), CORE),
    ('working',     spinner(12, lambda a: union(DOT, arc(16, 16, 9, 2, a, 100))), (16, 16), CORE),
    ('busy',        spinner(12, lambda a: arc(16, 16, 8, 2.6, a, 270)), (16, 16), CORE),
    ('precision',   union(seg(16, 4, 16, 11, 1.6), seg(16, 21, 16, 28, 1.6), seg(4, 16, 11, 16, 1.6), seg(21, 16, 28, 16, 1.6), circle(16, 16, 1.4)), (16, 16), CORE),
    ('text',        union(seg(16, 6, 16, 26, 1.8), seg(12.5, 6, 19.5, 6, 1.8), seg(12.5, 26, 19.5, 26, 1.8)), (16, 16), CORE),
    ('handwriting', union(seg(10, 22, 24, 8, 3), tri((6, 26), (8.5, 21.5), (10.5, 23.5))), (6, 26), CORE),
    ('unavailable', union(ring(16, 16, 8, 2.6), seg(10.3, 10.3, 21.7, 21.7, 2.6)), (16, 16), CORE),
    ('vert',        arrow2(90), (16, 16), CORE),
    ('horz',        arrow2(0), (16, 16), CORE),
    ('dgn1',        arrow2(45, 8.5), (16, 16), CORE),
    ('dgn2',        arrow2(-45, 8.5), (16, 16), CORE),
    ('move',        union(arrow2(0), arrow2(90)), (16, 16), CORE),
    ('alternate',   union(seg(16, 10, 16, 28, 2), tri((16, 3), (21, 11), (11, 11))), (16, 3), CORE),
    ('link',        union(circle(16, 16, 3), ring(16, 16, 9, 1.6)), (16, 16), GOLD),
    ('pin',         union(ring(16, 10, 6, 2), circle(16, 10, 2), seg(16, 16, 16, 28, 2)), (16, 29), CORE),
    ('person',      union(circle(16, 10, 3.5), arc(16, 25, 7, 2.6, 180, 180)), (16, 16), CORE),
]

def render(shape, fill, S):
    k = S / 32
    rows = []
    for y in range(S):
        row = bytearray([0])                                      # PNG filter: none
        for x in range(S):
            d = shape((x + 0.5) / k, (y + 0.5) / k)
            layers = ((GLOW, 0.45 * math.exp(-(max(d, 0) / 3.5) ** 2)),
                      (EDGE, 0.85 * max(0.0, min(1.0, 0.5 - (d - OUT) * k))),
                      (fill, max(0.0, min(1.0, 0.5 - d * k))))
            r = g = b = a = 0.0                                   # premultiplied "over", bottom to top
            for (cr, cg, cb), ca in layers:
                r, g, b, a = cr * ca + r * (1 - ca), cg * ca + g * (1 - ca), cb * ca + b * (1 - ca), ca + a * (1 - ca)
            row += bytes((round(r / a), round(g / a), round(b / a), round(255 * a))) if a > 0 else bytes(4)
        rows.append(bytes(row))
    return png(S, rows)

def chunk(kind, data):
    return struct.pack('>I', len(data)) + kind + data + struct.pack('>I', zlib.crc32(kind + data) & 0xffffffff)

def png(S, rows):
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', S, S, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(b''.join(rows), 9)) + chunk(b'IEND', b''))

def cur(shape, hot, fill):
    # ICONDIR (type 2 = cursor) + one ICONDIRENTRY per size; "planes/bitcount" hold the hotspot
    imgs = [(S, render(shape, fill, S)) for S in SIZES]
    out, off = struct.pack('<HHH', 0, 2, len(imgs)), 6 + 16 * len(imgs)
    for S, data in imgs:
        out += struct.pack('<BBBBHHII', S, S, 0, 0, round(hot[0] * S / 32), round(hot[1] * S / 32), len(data), off)
        off += len(data)
    return out + b''.join(data for _, data in imgs)

def riff(kind, data):
    return kind + struct.pack('<I', len(data)) + data + b'\0' * (len(data) & 1)

def ani(frames, hot, fill, jiffies=4):        # 4/60 s per frame
    icons = b''.join(riff(b'icon', cur(f, hot, fill)) for f in frames)
    anih = struct.pack('<9I', 36, len(frames), len(frames), 0, 0, 0, 0, jiffies, 1)   # flags: AF_ICON
    return riff(b'RIFF', b'ACON' + riff(b'anih', anih) + riff(b'LIST', b'fram' + icons))

NAME, DIR = 'Cursor Ring', 'Cursor Ring'

def inf(files):
    keys = [stem for stem, *_ in ROLES]
    paths = ','.join(f'%10%\\%CUR_DIR%\\%{k}%' for k in keys)
    return '\r\n'.join([
        '; Installs the Cursor Ring pointer scheme. Right-click -> Install, then choose it in Mouse settings -> Pointers.',
        '[Version]', 'signature="$CHICAGO$"', '',
        '[DefaultInstall]', 'CopyFiles = Scheme.Cur', 'AddReg    = Scheme.Reg', '',
        '[DestinationDirs]', 'Scheme.Cur = 10,"%CUR_DIR%"', '',
        '[Scheme.Reg]', f'HKCU,"Control Panel\\Cursors\\Schemes","%SCHEME_NAME%",,"{paths}"', '',
        '[Scheme.Cur]', *files, '',
        '[Strings]', f'CUR_DIR     = "Cursors\\{DIR}"', f'SCHEME_NAME = "{NAME}"',
        *(f'{k:<11} = "{f}"' for k, f in zip(keys, files)), ''])

if __name__ == '__main__':
    out = pathlib.Path(__file__).resolve().parent.parent / 'assets' / 'scheme'
    out.mkdir(parents=True, exist_ok=True)
    files = []
    for stem, shape, hot, fill in ROLES:
        name, data = (f'{stem}.ani', ani(shape, hot, fill)) if isinstance(shape, list) else (f'{stem}.cur', cur(shape, hot, fill))
        assert data[:4] == b'RIFF' and struct.unpack('<I', data[4:8])[0] == len(data) - 8 if name.endswith('.ani') \
            else struct.unpack('<HHH', data[:6]) == (0, 2, len(SIZES))
        (out / name).write_bytes(data)
        files.append(name)
    (out / 'install.inf').write_text(inf(files), encoding='ascii', newline='')
    print(out, len(files), 'cursors')
